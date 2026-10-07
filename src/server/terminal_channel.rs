//! A single, non-persistent shell carried by an authenticated remote session.
use base::message_proto::{
    terminal_action, TerminalAction, TerminalClosed, TerminalData, TerminalImage,
    TerminalImageRequest, TerminalInputAck, TerminalOpened, TerminalResponse,
};
use hbb_common::{
    anyhow::{anyhow, bail, Result},
    libc, log,
    tokio::{self, sync::mpsc},
};

pub const MAX_FRAME_BYTES: usize = 16 * 1024;

pub fn supported() -> bool {
    cfg!(all(
        feature = "terminal-channel",
        unix,
        not(any(target_os = "android", target_os = "ios"))
    ))
}

pub fn size(rows: u32, cols: u32) -> Result<(u16, u16)> {
    if rows == 0 || cols == 0 || rows > 1000 || cols > 1000 {
        bail!("Invalid terminal dimensions");
    }
    Ok((rows as u16, cols as u16))
}

enum Output {
    Data(Vec<u8>),
    Image(TerminalImage),
    InputAck(TerminalInputAck),
    Closed,
}

pub struct TerminalChannel {
    pub resume_token: String,
    retained_output: std::collections::VecDeque<Vec<u8>>,
    retained_bytes: usize,
    output_omitted: bool,
    ended: bool,
    output: mpsc::Receiver<Output>,
    image: std::sync::mpsc::SyncSender<TerminalImageRequest>,
    next_output: tokio::time::Instant,
    #[cfg(all(
        feature = "terminal-channel",
        unix,
        not(any(target_os = "android", target_os = "ios"))
    ))]
    pty: Box<dyn portable_pty::MasterPty + Send>,
    #[cfg(all(
        feature = "terminal-channel",
        unix,
        not(any(target_os = "android", target_os = "ios"))
    ))]
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    #[cfg(all(
        feature = "terminal-channel",
        unix,
        not(any(target_os = "android", target_os = "ios"))
    ))]
    input: Option<std::sync::mpsc::SyncSender<(Vec<u8>, u32)>>,
}

impl TerminalChannel {
    #[cfg(all(
        feature = "terminal-channel",
        unix,
        not(any(target_os = "android", target_os = "ios"))
    ))]
    pub fn open(rows: u32, cols: u32) -> Result<Self> {
        use portable_pty::{CommandBuilder, PtySize};
        use std::{
            io::{Read, Write},
            sync::mpsc as sync_mpsc,
            thread,
        };
        let (rows, cols) = size(rows, cols)?;
        // A system service must not expose a root shell in place of the logged-in user.
        if unsafe { libc::geteuid() } == 0 {
            bail!("Terminal requires the logged-in user's server process");
        }
        let pair = portable_pty::native_pty_system().openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/bin/sh".into());
        let mut cmd = CommandBuilder::new(shell);
        cmd.arg("-l");
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Ok(home) = std::env::var("HOME") {
            cmd.cwd(home);
        }
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let child = pair.slave.spawn_command(cmd)?;
        let pid = child.process_id();
        drop(pair.slave);
        let (output_tx, output) = mpsc::channel(16);
        let (input, input_rx) = sync_mpsc::sync_channel::<(Vec<u8>, u32)>(16);
        let input_tx = output_tx.clone();
        thread::spawn(move || {
            while let Ok((data, sequence)) = input_rx.recv() {
                let result = writer.write_all(&data).and_then(|_| writer.flush());
                let error = result
                    .as_ref()
                    .err()
                    .map(ToString::to_string)
                    .unwrap_or_default();
                if sequence != 0
                    && input_tx
                        .blocking_send(Output::InputAck(TerminalInputAck {
                            sequence,
                            error,
                            ..Default::default()
                        }))
                        .is_err()
                {
                    break;
                }
                if let Err(err) = result {
                    log::trace!("Terminal writer stopped: {err}");
                    break;
                }
            }
        });
        let (image, image_rx) = sync_mpsc::sync_channel::<TerminalImageRequest>(1);
        let image_tx = output_tx.clone();
        thread::spawn(move || {
            while let Ok(request) = image_rx.recv() {
                if let Err(err) = read_image(&request, pid, &image_tx) {
                    let response = TerminalImage {
                        request_id: request.request_id,
                        done: true,
                        error: err.to_string(),
                        ..Default::default()
                    };
                    if image_tx.blocking_send(Output::Image(response)).is_err() {
                        break;
                    }
                }
            }
        });
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            let mut pending = Vec::new();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Err(err) => {
                        log::trace!("Terminal reader stopped: {err}");
                        break;
                    }
                    Ok(n) => {
                        pending.extend_from_slice(&buf[..n]);
                        // Preserve UTF-8 codepoints across PTY reads for the Dart decoder.
                        let end = match std::str::from_utf8(&pending) {
                            Err(e) if e.error_len().is_none() => e.valid_up_to(),
                            _ => pending.len(),
                        };
                        if end > 0
                            && output_tx
                                .blocking_send(Output::Data(pending.drain(..end).collect()))
                                .is_err()
                        {
                            return;
                        }
                    }
                }
            }
            if !pending.is_empty() {
                if let Err(err) = output_tx.blocking_send(Output::Data(pending)) {
                    log::trace!("Terminal output receiver closed: {err}");
                }
            }
            if let Err(err) = output_tx.blocking_send(Output::Closed) {
                log::trace!("Terminal receiver closed: {err}");
            }
        });
        Ok(Self {
            resume_token: hbb_common::uuid::Uuid::new_v4().to_string(),
            retained_output: Default::default(),
            retained_bytes: 0,
            output_omitted: false,
            ended: false,
            output,
            image,
            next_output: tokio::time::Instant::now(),
            pty: pair.master,
            child: Some(child),
            input: Some(input),
        })
    }

    #[cfg(not(all(
        feature = "terminal-channel",
        unix,
        not(any(target_os = "android", target_os = "ios"))
    )))]
    pub fn open(_rows: u32, _cols: u32) -> Result<Self> {
        bail!("Terminal channel is unsupported")
    }

    pub fn action(&mut self, action: TerminalAction) -> Result<()> {
        #[cfg(all(
            feature = "terminal-channel",
            unix,
            not(any(target_os = "android", target_os = "ios"))
        ))]
        match action.union {
            Some(terminal_action::Union::Data(data))
                if data.terminal_id == 0
                    && !data.compressed
                    && data.data.len() <= MAX_FRAME_BYTES =>
            {
                self.input
                    .as_ref()
                    .ok_or_else(|| anyhow!("Terminal is closed"))?
                    .try_send((data.data.to_vec(), data.input_sequence))
                    .map_err(|_| anyhow!("Terminal input queue is full or closed"))?;
                return Ok(());
            }
            Some(terminal_action::Union::Image(request)) => {
                if request.path.len() > 4096 {
                    bail!("Image path is too long");
                }
                self.image
                    .try_send(request)
                    .map_err(|_| anyhow!("Image preview is busy or closed"))?;
                return Ok(());
            }
            Some(terminal_action::Union::Resize(resize)) if resize.terminal_id == 0 => {
                let (rows, cols) = size(resize.rows, resize.cols)?;
                self.pty.resize(portable_pty::PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })?;
                return Ok(());
            }
            _ => {}
        }
        bail!("Invalid terminal action")
    }

    pub fn buffer_detached_output(&mut self) {
        for _ in 0..64 {
            let Ok(output) = self.output.try_recv() else {
                break;
            };
            match output {
                Output::Data(data) => {
                    self.retained_bytes += data.len();
                    self.retained_output.push_back(data);
                    while self.retained_bytes > 1024 * 1024 {
                        if let Some(old) = self.retained_output.pop_front() {
                            self.retained_bytes -= old.len();
                            self.output_omitted = true;
                        }
                    }
                }
                Output::Closed => self.ended = true,
                Output::Image(_) | Output::InputAck(_) => {}
            }
        }
    }

    pub fn opened(&self) -> TerminalResponse {
        let mut response = TerminalResponse::new();
        response.set_opened(TerminalOpened {
            terminal_id: 0,
            success: true,
            service_id: self.resume_token.clone(),
            message: if self.output_omitted {
                "Some terminal output was omitted while disconnected.".into()
            } else {
                String::new()
            },
            replay_terminal_output: !self.retained_output.is_empty(),
            ..Default::default()
        });
        response
    }
}

pub async fn receive(channel: &mut Option<TerminalChannel>) -> TerminalResponse {
    let Some(session) = channel.as_mut() else {
        return std::future::pending().await;
    };
    let mut response = TerminalResponse::new();
    // Keep the deadline across select! cancellation so video traffic cannot restart it.
    tokio::time::sleep_until(session.next_output).await;
    if let Some(data) = session.retained_output.pop_front() {
        session.retained_bytes -= data.len();
        session.next_output = tokio::time::Instant::now() + std::time::Duration::from_millis(16);
        response.set_data(TerminalData {
            terminal_id: 0,
            data: data.into(),
            replayed: true,
            ..Default::default()
        });
        return response;
    }
    if session.ended {
        channel.take();
        response.set_closed(TerminalClosed {
            terminal_id: 0,
            ..Default::default()
        });
        return response;
    }
    match session.output.recv().await {
        Some(Output::Data(data)) => {
            session.next_output =
                tokio::time::Instant::now() + std::time::Duration::from_millis(16);
            // At most one 4KB PTY chunk per 16ms; the bounded queue pushes back on the shell.
            response.set_data(TerminalData {
                terminal_id: 0,
                data: data.into(),
                ..Default::default()
            });
        }
        Some(Output::Image(data)) => {
            session.next_output =
                tokio::time::Instant::now() + std::time::Duration::from_millis(16);
            response.set_image(data);
        }
        Some(Output::InputAck(ack)) => response.set_input_ack(ack),
        Some(Output::Closed) | None => {
            channel.take();
            response.set_closed(TerminalClosed {
                terminal_id: 0,
                ..Default::default()
            });
        }
    }
    response
}

#[cfg(all(
    feature = "terminal-channel",
    unix,
    not(any(target_os = "android", target_os = "ios"))
))]
fn read_image(
    request: &TerminalImageRequest,
    pid: Option<u32>,
    tx: &mpsc::Sender<Output>,
) -> Result<()> {
    use std::{fs::OpenOptions, io::Read, os::unix::fs::OpenOptionsExt, path::PathBuf};
    let path = if let Some(path) = request.path.strip_prefix("~/") {
        PathBuf::from(std::env::var("HOME")?).join(path)
    } else {
        let path = PathBuf::from(&request.path);
        if path.is_absolute() {
            path
        } else {
            let cwd = pid
                .and_then(|p| shell_working_directory(p).ok())
                .ok_or_else(|| {
                    anyhow!("Cannot resolve the shell working directory; use an absolute path")
                })?;
            cwd.join(path)
        }
    };
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["png", "jpg", "jpeg", "gif", "webp", "bmp"].contains(&extension.as_str()) {
        bail!("Unsupported image format");
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    const LIMIT: usize = 8 * 1024 * 1024;
    if !metadata.is_file() || metadata.len() > LIMIT as u64 {
        bail!("Image must be a regular file of at most 8 MB");
    }
    let mut total = 0;
    let mut buf = [0u8; MAX_FRAME_BYTES];
    loop {
        let n = file.read(&mut buf)?;
        total += n;
        if total > LIMIT {
            bail!("Image exceeds 8 MB");
        }
        let response = TerminalImage {
            request_id: request.request_id,
            data: buf[..n].to_vec().into(),
            done: n == 0,
            ..Default::default()
        };
        tx.blocking_send(Output::Image(response))
            .map_err(|_| anyhow!("Preview connection closed"))?;
        if n == 0 {
            return Ok(());
        }
    }
}

#[cfg(all(feature = "terminal-channel", target_os = "macos"))]
fn shell_working_directory(pid: u32) -> Result<std::path::PathBuf> {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let pid = i32::try_from(pid)?;
    if pid <= 0 {
        bail!("Invalid shell PID");
    }
    let mut info = std::mem::MaybeUninit::<libc::proc_vnodepathinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_vnodepathinfo>();
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            info.as_mut_ptr().cast(),
            size as i32,
        )
    };
    if read != size as i32 {
        bail!("Cannot read shell working directory");
    }
    let info = unsafe { info.assume_init() };
    // Bound the scan to the native path buffer, including its terminating NUL.
    let path = unsafe {
        std::slice::from_raw_parts(
            info.pvi_cdir.vip_path.as_ptr().cast::<u8>(),
            std::mem::size_of_val(&info.pvi_cdir.vip_path),
        )
    };
    let end = path
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("Invalid shell working directory"))?;
    let path = std::path::PathBuf::from(OsString::from_vec(path[..end].to_vec()));
    if !path.is_absolute() {
        bail!("Invalid shell working directory");
    }
    Ok(path)
}

#[cfg(all(
    feature = "terminal-channel",
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "ios"))
))]
fn shell_working_directory(pid: u32) -> Result<std::path::PathBuf> {
    Ok(std::fs::read_link(format!("/proc/{pid}/cwd"))?)
}

#[cfg(all(test, feature = "terminal-channel", target_os = "macos"))]
mod macos_tests {
    use super::*;

    #[test]
    fn resolves_current_directory_without_procfs() {
        assert_eq!(
            shell_working_directory(std::process::id())
                .unwrap()
                .canonicalize()
                .unwrap(),
            std::env::current_dir().unwrap().canonicalize().unwrap()
        );
        assert!(shell_working_directory(0).is_err());
        assert!(shell_working_directory(u32::MAX).is_err());
    }

    #[tokio::test]
    async fn opens_resizes_and_receives_a_native_shell() {
        let mut session = TerminalChannel::open(24, 80).unwrap();
        let mut action = TerminalAction::new();
        action.set_resize(base::message_proto::ResizeTerminal {
            terminal_id: 0,
            rows: 30,
            cols: 90,
            ..Default::default()
        });
        session.action(action).unwrap();
        let mut action = TerminalAction::new();
        action.set_data(TerminalData {
            terminal_id: 0,
            data: b"printf 'macos-native-shell-ok\\n'\r".to_vec().into(),
            ..Default::default()
        });
        session.action(action).unwrap();
        let mut channel = Some(session);
        let mut data = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if let Some(base::message_proto::terminal_response::Union::Data(chunk)) =
                    receive(&mut channel).await.union
                {
                    data.extend_from_slice(&chunk.data);
                    if data.windows(23).any(|s| s == b"macos-native-shell-ok\r\n") {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
    }
}

impl Drop for TerminalChannel {
    fn drop(&mut self) {
        self.output.close();
        #[cfg(all(
            feature = "terminal-channel",
            unix,
            not(any(target_os = "android", target_os = "ios"))
        ))]
        {
            let input = self.input.take();
            if let Some(mut child) = self.child.take() {
                std::thread::spawn(move || {
                    if let Some(pid) = child.process_id() {
                        kill_shell_jobs(pid);
                        if matches!(child.try_wait(), Ok(None)) {
                            if let Err(err) = child.kill() {
                                log::trace!("Terminal child cleanup: {err}");
                            }
                        }
                    }
                    drop(input);
                    if let Err(err) = child.wait() {
                        log::trace!("Terminal child wait: {err}");
                    }
                });
            }
        }
    }
}

#[cfg(all(
    feature = "terminal-channel",
    unix,
    not(any(target_os = "android", target_os = "ios"))
))]
fn kill_shell_jobs(shell: u32) {
    let output = match std::process::Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => {
            log::trace!("Cannot enumerate terminal descendants");
            return;
        }
    };
    let processes: Vec<(u32, u32)> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
        })
        .collect();
    let mut descendants = vec![shell];
    let mut index = 0;
    while index < descendants.len() {
        let parent = descendants[index];
        for &(pid, ppid) in &processes {
            if ppid == parent && pid != shell && !descendants.contains(&pid) {
                descendants.push(pid);
            }
        }
        index += 1;
    }
    for pid in descendants.into_iter().skip(1).rev() {
        if unsafe { libc::kill(pid as i32, libc::SIGKILL) } != 0 {
            log::trace!("Terminal job cleanup: {}", std::io::Error::last_os_error());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_dimensions() {
        assert!(size(0, 80).is_err());
        assert!(size(24, u32::MAX).is_err());
        assert_eq!(size(24, 80).unwrap(), (24, 80));
    }
}

#[cfg(all(test, feature = "terminal-channel", target_os = "linux"))]
mod pty_tests {
    use super::*;
    use base::message_proto::{terminal_response, ResizeTerminal};
    use std::time::Duration;

    fn write(session: &mut TerminalChannel, data: &[u8]) {
        let mut action = TerminalAction::new();
        action.set_data(TerminalData {
            terminal_id: 0,
            data: data.to_vec().into(),
            ..Default::default()
        });
        session.action(action).unwrap();
    }

    async fn until(session: &mut Option<TerminalChannel>, expected: &str) -> String {
        let mut output = String::new();
        let result = tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if let Some(terminal_response::Union::Data(data)) = receive(session).await.union {
                    assert!(data.data.len() <= MAX_FRAME_BYTES);
                    output.push_str(std::str::from_utf8(&data.data).unwrap());
                    if output.contains(expected) {
                        return;
                    }
                } else {
                    panic!("Shell ended before producing {expected}: {output}");
                }
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "Waiting for {expected:?}; received {output:?}"
        );
        output
    }

    #[tokio::test]
    async fn shell_environment_resize_interrupt_utf8_and_cleanup() {
        let mut session = Some(TerminalChannel::open(24, 80).unwrap());
        let pid = session
            .as_ref()
            .unwrap()
            .child
            .as_ref()
            .unwrap()
            .process_id()
            .unwrap();
        write(
            session.as_mut().unwrap(),
            b"printf '\\nENV:%s:%s\\n' \"$TERM\" \"$COLORTERM\"\r",
        );
        until(&mut session, "ENV:xterm-256color:truecolor").await;
        write(
            session.as_mut().unwrap(),
            "printf '\\n中文输入输出\\n'\r".as_bytes(),
        );
        until(&mut session, "\r\n中文输入输出\r\n").await;
        write(
            session.as_mut().unwrap(),
            b"trap 'printf WINCH_ACK' WINCH; printf '\\nTRAP_READY\\n'\r",
        );
        until(&mut session, "\r\nTRAP_READY\r\n").await;
        let mut action = TerminalAction::new();
        action.set_resize(ResizeTerminal {
            terminal_id: 0,
            rows: 42,
            cols: 101,
            ..Default::default()
        });
        session.as_mut().unwrap().action(action).unwrap();
        until(&mut session, "WINCH_ACK").await;
        write(session.as_mut().unwrap(), b"stty size\r");
        until(&mut session, "42 101\r\n").await;
        write(session.as_mut().unwrap(), b"sleep 30\r");
        until(&mut session, "sleep 30").await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        write(session.as_mut().unwrap(), b"\x03");
        tokio::time::sleep(Duration::from_millis(100)).await;
        write(
            session.as_mut().unwrap(),
            b"printf '\\nINTERRUPT_%s\\n' OK\r",
        );
        until(&mut session, "INTERRUPT_OK\r\n").await;
        drop(session);
        tokio::time::timeout(Duration::from_secs(3), async {
            while unsafe { libc::kill(pid as i32, 0) } == 0 {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn destroy_stops_background_jobs_and_acknowledged_paste_is_complete() {
        let mut session = Some(TerminalChannel::open(24, 80).unwrap());
        write(
            session.as_mut().unwrap(),
            b"sleep 30 & printf '\\nBACKGROUND_PID:%s\\nJOB_READY\\n' $!\r",
        );
        let output = until(&mut session, "\r\nJOB_READY\r\n").await;
        let background: i32 = output
            .lines()
            .find_map(|line| {
                line.strip_prefix("BACKGROUND_PID:")
                    .and_then(|value| value.trim().parse().ok())
            })
            .unwrap();
        write(session.as_mut().unwrap(), b"stty -icanon -echo; python3 -c \"import sys,time; print('RAW_READY',flush=True); time.sleep(.1); d=sys.stdin.buffer.read(180000); print('INPUT_BYTES:%s'%len(d),flush=True)\"; stty sane\r");
        until(&mut session, "RAW_READY\r\n").await;
        let mut output = String::new();
        for (index, chunk) in vec![b'x'; 180000].chunks(4096).enumerate() {
            let sequence = index as u32 + 1;
            let mut action = TerminalAction::new();
            action.set_data(TerminalData {
                terminal_id: 0,
                data: chunk.to_vec().into(),
                input_sequence: sequence,
                ..Default::default()
            });
            session.as_mut().unwrap().action(action).unwrap();
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    match receive(&mut session).await.union {
                        Some(terminal_response::Union::InputAck(ack)) => {
                            assert_eq!(ack.sequence, sequence);
                            assert!(ack.error.is_empty());
                            break;
                        }
                        Some(terminal_response::Union::Data(data)) => {
                            output.push_str(std::str::from_utf8(&data.data).unwrap())
                        }
                        _ => panic!("Expected data or input acknowledgement"),
                    }
                }
            })
            .await
            .unwrap();
        }
        if !output.contains("INPUT_BYTES:180000") {
            until(&mut session, "INPUT_BYTES:180000").await;
        }
        drop(session);
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let running = std::fs::read_to_string(format!("/proc/{background}/stat"))
                    .ok()
                    .map(|stat| !stat.contains(") Z "))
                    .unwrap_or(false);
                if !running {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn cancelled_output_reads_preserve_data_under_backpressure() {
        let mut session = Some(TerminalChannel::open(24, 80).unwrap());
        write(
            session.as_mut().unwrap(),
            b"python3 -c \"print('x'*200000); print('STREAM_DONE')\"\r",
        );
        let mut output = String::new();
        for _ in 0..200 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(1)) => {},
                response = receive(&mut session) => {
                    if let Some(terminal_response::Union::Data(data)) = response.union {
                        output.push_str(std::str::from_utf8(&data.data).unwrap());
                    }
                },
            }
        }
        assert!(
            !output.is_empty(),
            "Other connection traffic starved terminal output"
        );
        output.push_str(&until(&mut session, "\r\nSTREAM_DONE\r\n").await);
        assert!(
            output.contains(&"x".repeat(200000)),
            "PTY output was dropped or reordered"
        );
    }
}

#[cfg(all(test, feature = "terminal-channel", target_os = "linux"))]
mod image_tests {
    use super::*;
    #[test]
    fn image_frames_preserve_bytes_and_reject_large_or_nonregular_files() {
        let path = std::env::temp_dir().join(format!("rustdesk-image-{}.png", std::process::id()));
        let data = vec![0x85; MAX_FRAME_BYTES + 37];
        std::fs::write(&path, &data).unwrap();
        let request = TerminalImageRequest {
            request_id: 7,
            path: path.to_string_lossy().into(),
            ..Default::default()
        };
        let (tx, mut rx) = mpsc::channel(16);
        read_image(&request, None, &tx).unwrap();
        let mut received = Vec::new();
        loop {
            let Output::Image(frame) = rx.blocking_recv().unwrap() else {
                panic!("Expected image");
            };
            assert_eq!(frame.request_id, 7);
            assert!(frame.data.len() <= MAX_FRAME_BYTES);
            received.extend_from_slice(&frame.data);
            if frame.done {
                break;
            }
        }
        assert_eq!(received, data);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(8 * 1024 * 1024 + 1)
            .unwrap();
        assert!(read_image(&request, None, &tx).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(read_image(&request, None, &tx).is_err());
        std::fs::remove_dir(&path).unwrap();
    }
}
