use base::message_proto::TerminalResourceUsage;
use hbb_common::tokio::{
    self,
    task::JoinHandle,
    time::{Duration, Instant},
};

pub struct ResourceMonitor {
    next_sample: Instant,
    sample: Option<JoinHandle<TerminalResourceUsage>>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            next_sample: Instant::now(),
            sample: None,
        }
    }

    pub async fn receive(&mut self) -> TerminalResourceUsage {
        if !cfg!(all(
            feature = "terminal-channel",
            any(target_os = "linux", target_os = "macos")
        )) {
            return std::future::pending().await;
        }
        if self.sample.is_none() {
            tokio::time::sleep_until(self.next_sample).await;
            self.next_sample = Instant::now() + Duration::from_secs(5);
            self.sample = Some(tokio::task::spawn_blocking(read_usage));
        }
        // Retain the worker across select! cancellation; never start overlapping reads.
        let usage = match self.sample.as_mut() {
            Some(sample) => sample.await.unwrap_or_default(),
            None => TerminalResourceUsage::default(),
        };
        self.sample = None;
        usage
    }
}

fn read_usage() -> TerminalResourceUsage {
    #[cfg(all(feature = "terminal-channel", target_os = "linux"))]
    {
        use hbb_common::libc;
        let mut usage = TerminalResourceUsage::default();
        if let Ok(text) = std::fs::read_to_string("/proc/meminfo") {
            if let Some((total, used)) = memory_usage(&text) {
                usage.memory_total = total;
                usage.memory_used = used;
            }
        }
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(b"/\0".as_ptr().cast(), stat.as_mut_ptr()) } == 0 {
            let stat = unsafe { stat.assume_init() };
            if let Some((total, used)) = disk_usage(
                stat.f_blocks as u64,
                stat.f_bfree as u64,
                stat.f_frsize as u64,
            ) {
                usage.disk_total = total;
                usage.disk_used = used;
            }
        }
        return usage;
    }
    #[cfg(all(feature = "terminal-channel", target_os = "macos"))]
    {
        use hbb_common::{
            libc,
            sysinfo::{RefreshKind, System},
        };
        let system = System::new_with_specifics(RefreshKind::new().with_memory());
        let mut usage = TerminalResourceUsage::default();
        let total = system.total_memory();
        let used = system.used_memory();
        // A failed VM statistics read leaves used_memory at zero in sysinfo.
        if total > 0 && used > 0 && used <= total {
            usage.memory_total = total;
            usage.memory_used = used;
        }
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(b"/\0".as_ptr().cast(), stat.as_mut_ptr()) } == 0 {
            let stat = unsafe { stat.assume_init() };
            if let Some((total, used)) = disk_usage(
                stat.f_blocks as u64,
                stat.f_bfree as u64,
                stat.f_frsize as u64,
            ) {
                usage.disk_total = total;
                usage.disk_used = used;
            }
        }
        return usage;
    }
    #[cfg(not(all(
        feature = "terminal-channel",
        any(target_os = "linux", target_os = "macos")
    )))]
    TerminalResourceUsage::default()
}

#[cfg(any(test, all(feature = "terminal-channel", target_os = "linux")))]
fn memory_usage(text: &str) -> Option<(u64, u64)> {
    let value = |key: &str| {
        let mut fields = text
            .lines()
            .find(|line| line.starts_with(key))?
            .split_whitespace();
        fields.next()?;
        let kb = fields.next()?.parse::<u64>().ok()?;
        if fields.next()? != "kB" {
            return None;
        }
        kb.checked_mul(1024)
    };
    let total = value("MemTotal:")?;
    let available = value("MemAvailable:")?;
    Some((total, total.checked_sub(available)?))
}

#[cfg(any(
    test,
    all(
        feature = "terminal-channel",
        any(target_os = "linux", target_os = "macos")
    )
))]
fn disk_usage(blocks: u64, free: u64, block_size: u64) -> Option<(u64, u64)> {
    Some((
        blocks.checked_mul(block_size)?,
        blocks.checked_sub(free)?.checked_mul(block_size)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_available_memory_and_allocated_disk_blocks() {
        assert_eq!(
            memory_usage("MemTotal: 1000 kB\nMemFree: 10 kB\nMemAvailable: 400 kB\n"),
            Some((1024000, 614400))
        );
        assert_eq!(memory_usage("MemTotal: 1000 kB\n"), None);
        assert_eq!(disk_usage(100, 40, 4096), Some((409600, 245760)));
        assert_eq!(disk_usage(10, 20, 4096), None);
        assert_eq!(disk_usage(u64::MAX, 0, 4096), None);
    }

    #[cfg(all(
        feature = "terminal-channel",
        any(target_os = "linux", target_os = "macos")
    ))]
    #[tokio::test]
    async fn samples_real_host_without_repeating_during_quiet_interval() {
        let mut monitor = ResourceMonitor::new();
        let usage = monitor.receive().await;
        assert!(usage.memory_total > 0 && usage.memory_used <= usage.memory_total);
        assert!(usage.disk_total > 0 && usage.disk_used <= usage.disk_total);
        assert!(
            tokio::time::timeout(Duration::from_millis(30), monitor.receive())
                .await
                .is_err()
        );
    }
}
