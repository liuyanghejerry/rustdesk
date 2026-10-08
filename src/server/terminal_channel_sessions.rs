//! Detached session terminals, accessible only after authentication with their resume token.
use super::terminal_channel::TerminalChannel;
use hbb_common::{
    anyhow::{bail, Result},
    config::Config,
};
use std::{
    collections::HashMap,
    sync::{Mutex, Once},
    time::Duration,
};

#[derive(Default)]
struct Registry {
    active: HashMap<String, String>,
    detached: HashMap<String, (String, TerminalChannel)>,
}

hbb_common::lazy_static::lazy_static! {
    static ref SESSIONS: Mutex<Registry> = Mutex::new(Registry::default());
}
static WATCH_PERMISSION: Once = Once::new();

pub fn retain(owner: String, channel: TerminalChannel) -> Result<()> {
    WATCH_PERMISSION.call_once(|| {
        std::thread::spawn(|| loop {
            std::thread::sleep(Duration::from_millis(16));
            if Config::get_option(base::config::keys::OPTION_ENABLE_TERMINAL) != "Y" {
                let removed = std::mem::take(&mut SESSIONS.lock().unwrap().detached);
                drop(removed);
            } else {
                for (_, channel) in SESSIONS.lock().unwrap().detached.values_mut() {
                    channel.buffer_detached_output();
                }
            }
        });
    });
    let mut sessions = SESSIONS.lock().unwrap();
    sessions.active.remove(&channel.resume_token);
    if sessions.detached.len() >= 100 {
        bail!("Too many retained terminal sessions");
    }
    sessions
        .detached
        .insert(channel.resume_token.clone(), (owner, channel));
    Ok(())
}

pub fn list(
    owner: &str,
    owner_token: &str,
    resume_tokens: &[String],
) -> Vec<base::message_proto::RetainedTerminalSession> {
    let mut sessions = SESSIONS.lock().unwrap();
    let mut result: Vec<_> = sessions
        .detached
        .values_mut()
        .filter(|(stored_owner, _)| stored_owner == owner)
        .filter_map(|(_, channel)| {
            // A peer ID is self-reported; listing also requires a secret proof.
            if !(hbb_common::uuid::Uuid::parse_str(owner_token).is_ok()
                && channel.owner_token == owner_token)
                && !resume_tokens.contains(&channel.resume_token)
            {
                return None;
            }
            channel.buffer_detached_output();
            channel.session_info()
        })
        .collect();
    result.sort_by_key(|session| session.pid);
    result
}

pub fn take(owner: &str, token: &str) -> Result<Option<TerminalChannel>> {
    let mut sessions = SESSIONS.lock().unwrap();
    if let Some(stored_owner) = sessions.active.get(token) {
        if stored_owner != owner {
            bail!("Terminal belongs to another peer");
        }
        bail!("Terminal is reconnecting");
    }
    if let Some((stored_owner, _)) = sessions.detached.get(token) {
        if stored_owner != owner {
            bail!("Terminal belongs to another peer");
        }
    }
    let channel = sessions.detached.remove(token).map(|(_, channel)| channel);
    if channel.is_some() {
        sessions.active.insert(token.to_owned(), owner.to_owned());
    }
    Ok(channel)
}

pub fn attached(owner: &str, token: &str) {
    SESSIONS
        .lock()
        .unwrap()
        .active
        .insert(token.to_owned(), owner.to_owned());
}

pub fn destroyed(token: &str) {
    SESSIONS.lock().unwrap().active.remove(token);
}

#[cfg(all(test, feature = "terminal-channel", target_os = "linux"))]
mod tests {
    use super::*;
    use base::message_proto::{terminal_response, TerminalAction, TerminalData};
    use hbb_common::tokio;

    #[tokio::test]
    async fn reconnect_preserves_shell_state_and_checks_owner() {
        let mut channel = TerminalChannel::open(24, 80).unwrap();
        let token = channel.resume_token.clone();
        attached("owner", &token);
        assert!(take("owner", &token).is_err());
        let marker = format!(
            "/tmp/rustdesk-detached-{}.done",
            hbb_common::uuid::Uuid::new_v4()
        );
        let command = format!("export RETAINED_STATE=continued; python3 -c \"print('x'*2000000); print('DETACHED_JOB_DONE'); open('{marker}','w').write('done')\"\r");
        let mut action = TerminalAction::new();
        action.set_data(TerminalData {
            data: command.into_bytes().into(),
            ..Default::default()
        });
        channel.action(action).unwrap();
        retain("owner".into(), channel).unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while !std::path::Path::new(&marker).exists() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("Detached jobs must keep running while output is collected");
        std::fs::remove_file(&marker).unwrap();
        assert!(take("other-owner", &token).is_err());
        let mut resumed = Some(take("owner", &token).unwrap().unwrap());
        let Some(terminal_response::Union::Opened(opened)) =
            resumed.as_ref().unwrap().opened().union
        else {
            panic!("Expected opened");
        };
        assert!(opened.replay_terminal_output);
        assert!(opened.message.contains("omitted"));
        let mut action = TerminalAction::new();
        action.set_data(TerminalData {
            data: b"printf '\nRESUMED:%s\n' \"$RETAINED_STATE\"\r"
                .to_vec()
                .into(),
            ..Default::default()
        });
        resumed.as_mut().unwrap().action(action).unwrap();
        tokio::time::timeout(Duration::from_secs(8), async {
            let mut text = String::new();
            loop {
                if let Some(terminal_response::Union::Data(data)) =
                    super::super::terminal_channel::receive(&mut resumed)
                        .await
                        .union
                {
                    text.push_str(&String::from_utf8_lossy(&data.data));
                    if text.contains("RESUMED:continued") && text.contains("DETACHED_JOB_DONE") {
                        break;
                    }
                }
            }
        })
        .await
        .unwrap();
    }
}
