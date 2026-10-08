//! Multiplexed channel terminals on a terminal-only authenticated connection.
use super::{
    terminal_channel::{self, TerminalChannel},
    terminal_channel_sessions,
};
use base::message_proto::{
    terminal_action, terminal_response, TerminalAction, TerminalClosed, TerminalResponse,
};
use hbb_common::{
    anyhow::{anyhow, bail, Result},
    futures::future::select_all,
    tokio,
};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct ChannelGroup {
    channels: BTreeMap<i32, Option<TerminalChannel>>,
}

impl ChannelGroup {
    pub fn ids(&self) -> Vec<i32> {
        self.channels.keys().copied().collect()
    }
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    pub fn release(&mut self, owner: &str, keep: bool) {
        for (_, channel) in std::mem::take(&mut self.channels) {
            if let Some(channel) = channel {
                if keep {
                    if let Err(error) = terminal_channel_sessions::retain(owner.to_owned(), channel)
                    {
                        hbb_common::log::trace!("Terminal detach failed: {error}");
                    }
                } else {
                    terminal_channel_sessions::destroyed(&channel.resume_token);
                }
            }
        }
    }

    pub async fn action(
        &mut self,
        owner: &str,
        mut action: TerminalAction,
    ) -> Result<Option<TerminalResponse>> {
        let id = action_id(&action)?;
        if !(0..=1000).contains(&id) {
            bail!("Invalid terminal ID");
        }
        match action.union.as_ref() {
            Some(terminal_action::Union::Open(open)) => {
                terminal_channel::size(open.rows, open.cols)?;
                if !self.channels.contains_key(&id) {
                    if self.channels.len() >= 32 {
                        bail!("Too many terminal sessions");
                    }
                    if hbb_common::uuid::Uuid::parse_str(&open.resume_token).is_err() {
                        bail!("Invalid terminal resume token");
                    }
                    let resumed = terminal_channel_sessions::take(owner, &open.resume_token)?;
                    let mut channel = if let Some(mut channel) = resumed {
                        if let Err(error) = channel.prepare_reconnect_resize(open.rows, open.cols) {
                            terminal_channel_sessions::destroyed(&open.resume_token);
                            return Err(error);
                        }
                        channel
                    } else {
                        if !open.create_if_missing {
                            bail!("The retained shell is no longer available.");
                        }
                        terminal_channel_sessions::attached(owner, &open.resume_token);
                        let rows = open.rows;
                        let cols = open.cols;
                        match tokio::task::spawn_blocking(move || TerminalChannel::open(rows, cols))
                            .await
                        {
                            Ok(Ok(channel)) => channel,
                            Ok(Err(error)) => {
                                terminal_channel_sessions::destroyed(&open.resume_token);
                                return Err(error);
                            }
                            Err(error) => {
                                terminal_channel_sessions::destroyed(&open.resume_token);
                                return Err(error.into());
                            }
                        }
                    };
                    channel.resume_token = open.resume_token.clone();
                    terminal_channel_sessions::attached(owner, &channel.resume_token);
                    self.channels.insert(id, Some(channel));
                }
                let mut response = self
                    .channels
                    .get(&id)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| anyhow!("Terminal is closed"))?
                    .opened();
                set_response_id(&mut response, id);
                Ok(Some(response))
            }
            Some(terminal_action::Union::Close(close)) => {
                let channel = match self.channels.remove(&id).flatten() {
                    Some(channel) => Some(channel),
                    None if !close.resume_token.is_empty() => {
                        terminal_channel_sessions::take(owner, &close.resume_token)?
                    }
                    None => None,
                };
                if let Some(channel) = channel {
                    if close.keep_shell {
                        terminal_channel_sessions::retain(owner.to_owned(), channel)?;
                    } else {
                        terminal_channel_sessions::destroyed(&channel.resume_token);
                    }
                }
                let mut response = TerminalResponse::new();
                response.set_closed(TerminalClosed {
                    terminal_id: id,
                    keep_shell: close.keep_shell,
                    ..Default::default()
                });
                Ok(Some(response))
            }
            _ => {
                let channel = self
                    .channels
                    .get_mut(&id)
                    .and_then(Option::as_mut)
                    .ok_or_else(|| anyhow!("Terminal is not open"))?;
                match action.union.as_mut() {
                    Some(terminal_action::Union::Data(data)) => data.terminal_id = 0,
                    Some(terminal_action::Union::Resize(resize)) => resize.terminal_id = 0,
                    _ => {}
                }
                channel.action(action)?;
                Ok(None)
            }
        }
    }

    pub async fn receive(&mut self) -> TerminalResponse {
        if self.channels.is_empty() {
            return std::future::pending().await;
        }
        let receives: Vec<_> = self
            .channels
            .iter_mut()
            .map(|(&id, channel)| {
                Box::pin(async move {
                    let token = channel.as_ref().map(|c| c.resume_token.clone());
                    let mut response = terminal_channel::receive(channel).await;
                    set_response_id(&mut response, id);
                    (id, token, response)
                })
            })
            .collect();
        let ((id, token, response), _, _) = select_all(receives).await;
        if matches!(response.union, Some(terminal_response::Union::Closed(_))) {
            self.channels.remove(&id);
            if let Some(token) = token {
                terminal_channel_sessions::destroyed(&token);
            }
        }
        response
    }
}

pub fn action_id(action: &TerminalAction) -> Result<i32> {
    Ok(match action.union.as_ref() {
        Some(terminal_action::Union::Open(v)) => v.terminal_id,
        Some(terminal_action::Union::Data(v)) => v.terminal_id,
        Some(terminal_action::Union::Resize(v)) => v.terminal_id,
        Some(terminal_action::Union::Close(v)) => v.terminal_id,
        Some(terminal_action::Union::Image(v)) => v.terminal_id,
        _ => bail!("Invalid terminal action"),
    })
}

fn set_response_id(response: &mut TerminalResponse, id: i32) {
    match response.union.as_mut() {
        Some(terminal_response::Union::Opened(v)) => v.terminal_id = id,
        Some(terminal_response::Union::Data(v)) => v.terminal_id = id,
        Some(terminal_response::Union::Closed(v)) => v.terminal_id = id,
        Some(terminal_response::Union::Error(v)) => v.terminal_id = id,
        Some(terminal_response::Union::Image(v)) => v.terminal_id = id,
        Some(terminal_response::Union::InputAck(v)) => v.terminal_id = id,
        Some(terminal_response::Union::Resources(v)) => v.terminal_id = id,
        _ => {}
    }
}

impl Drop for ChannelGroup {
    fn drop(&mut self) {
        self.release("", false);
    }
}

#[cfg(all(
    test,
    feature = "terminal-channel",
    any(target_os = "linux", target_os = "macos")
))]
mod tests {
    use super::*;
    use base::message_proto::{
        CloseTerminal as TerminalClosedAction, OpenTerminal, TerminalData, TerminalImageRequest,
    };

    fn open(id: i32, token: String) -> TerminalAction {
        let mut action = TerminalAction::new();
        action.set_open(OpenTerminal {
            terminal_id: id,
            rows: 24,
            cols: 80,
            resume_token: token,
            create_if_missing: true,
            ..Default::default()
        });
        action
    }

    #[tokio::test]
    async fn independent_shells_keep_output_acknowledgements_and_close_scoped() {
        let mut group = ChannelGroup::default();
        let tokens: Vec<_> = (0..2)
            .map(|_| hbb_common::uuid::Uuid::new_v4().to_string())
            .collect();
        for id in 1..=2 {
            let response = group
                .action("owner", open(id, tokens[id as usize - 1].clone()))
                .await
                .unwrap()
                .unwrap();
            assert!(
                matches!(response.union, Some(terminal_response::Union::Opened(v)) if v.terminal_id == id && v.success)
            );
            let mut action = TerminalAction::new();
            action.set_data(TerminalData {
                terminal_id: id,
                data: format!("CHANNEL_STATE={id}; printf '\\nCHANNEL_%s\\n' {id}\r")
                    .into_bytes()
                    .into(),
                input_sequence: id as u32,
                ..Default::default()
            });
            group.action("owner", action).await.unwrap();
        }
        assert!(group
            .action("other-owner", open(3, tokens[0].clone()))
            .await
            .is_err());
        let mut outputs = BTreeMap::<i32, String>::new();
        let mut acknowledgements = std::collections::BTreeSet::new();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                match group.receive().await.union {
                    Some(terminal_response::Union::Data(v)) => {
                        outputs
                            .entry(v.terminal_id)
                            .or_default()
                            .push_str(&String::from_utf8_lossy(&v.data));
                    }
                    Some(terminal_response::Union::InputAck(v)) => {
                        assert_eq!(v.sequence, v.terminal_id as u32);
                        acknowledgements.insert(v.terminal_id);
                    }
                    _ => {}
                }
                if acknowledgements.len() == 2
                    && (1..=2).all(|id| {
                        outputs
                            .get(&id)
                            .map(|s| s.contains(&format!("CHANNEL_{id}\r\n")))
                            .unwrap_or(false)
                    })
                {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(!outputs[&1].contains("CHANNEL_2\r\n"));
        assert!(!outputs[&2].contains("CHANNEL_1\r\n"));
        let mut action = TerminalAction::new();
        action.set_close(TerminalClosedAction {
            terminal_id: 1,
            ..Default::default()
        });
        let response = group.action("owner", action).await.unwrap().unwrap();
        assert!(
            matches!(response.union, Some(terminal_response::Union::Closed(v)) if v.terminal_id == 1)
        );
        assert_eq!(group.ids(), vec![2]);
        group.release("owner", true);
        group
            .action("owner", open(2, tokens[1].clone()))
            .await
            .unwrap();
        let mut action = TerminalAction::new();
        action.set_data(TerminalData {
            terminal_id: 2,
            data: b"sleep .2; printf '\nRESUMED_%s\n' \"$CHANNEL_STATE\"; stty size\r"
                .to_vec()
                .into(),
            ..Default::default()
        });
        group.action("owner", action).await.unwrap();
        let mut resumed = String::new();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !resumed.contains("RESUMED_2\r\n") || !resumed.contains("24 80\r\n") {
                if let Some(terminal_response::Union::Data(v)) = group.receive().await.union {
                    assert_eq!(v.terminal_id, 2);
                    resumed.push_str(&String::from_utf8_lossy(&v.data));
                }
            }
        })
        .await
        .unwrap();
        group.release("owner", false);
        assert!(terminal_channel_sessions::take("owner", &tokens[0])
            .unwrap()
            .is_none());
        assert!(terminal_channel_sessions::take("owner", &tokens[1])
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn image_responses_return_to_the_requesting_terminal() {
        let mut group = ChannelGroup::default();
        let path = std::env::temp_dir().join(format!(
            "channel-image-{}.png",
            hbb_common::uuid::Uuid::new_v4()
        ));
        std::fs::write(&path, b"isolated-image-bytes").unwrap();
        for id in 1..=2 {
            group
                .action(
                    "image-owner",
                    open(id, hbb_common::uuid::Uuid::new_v4().to_string()),
                )
                .await
                .unwrap();
            let mut action = TerminalAction::new();
            action.set_image(TerminalImageRequest {
                terminal_id: id,
                request_id: 42,
                path: path.to_string_lossy().into(),
                ..Default::default()
            });
            group.action("image-owner", action).await.unwrap();
        }
        let mut completed = std::collections::BTreeSet::new();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while completed.len() < 2 {
                if let Some(terminal_response::Union::Image(v)) = group.receive().await.union {
                    assert_eq!(v.request_id, 42);
                    assert!(v.error.is_empty());
                    if !v.data.is_empty() {
                        assert_eq!(v.data.as_ref(), b"isolated-image-bytes");
                    }
                    if v.done {
                        completed.insert(v.terminal_id);
                    }
                }
            }
        })
        .await
        .unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(completed.into_iter().collect::<Vec<_>>(), vec![1, 2]);
        group.release("image-owner", false);
    }
}
