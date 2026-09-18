use super::{HookEvent, HookInput, Result, SessionEndReason};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Activity {
    pub agents: BTreeMap<String, Agent>,
    pub ended: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Agent {
    pub agent_type: String,
    pub started_at: u64,
    pub responded_at: Option<u64>,
}

impl Activity {
    pub fn apply(&mut self, event: HookEvent, now: u64) {
        match event {
            HookEvent::SessionStart { source } => {
                // Compaction does not end background subagents.
                if source != "compact" {
                    *self = Self::default();
                }
            }
            HookEvent::SessionEnd { .. } => self.ended = true,
            HookEvent::SubagentStart {
                agent_id,
                agent_type,
            } => {
                if !self.ended {
                    let agent = self.agents.entry(agent_id).or_insert(Agent {
                        agent_type: agent_type.clone(),
                        started_at: now,
                        responded_at: None,
                    });
                    // Duplicate starts retain elapsed time; a resumed agent starts a new turn.
                    if agent.responded_at.take().is_some() {
                        agent.started_at = now;
                    }
                    agent.agent_type = agent_type;
                }
            }
            HookEvent::SubagentStop { agent_id, .. } => {
                if let Some(agent) = self.agents.get_mut(&agent_id) {
                    agent.responded_at.get_or_insert(now);
                }
            }
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ProcessActivity {
    sessions: BTreeMap<String, Activity>,
}

pub struct ActivityStore {
    root: PathBuf,
    process_id: u32,
}

impl ActivityStore {
    pub fn new(root: PathBuf, process_id: u32) -> Self {
        Self { root, process_id }
    }

    fn path(&self) -> PathBuf {
        self.root.join(format!("{}.json", self.process_id))
    }

    fn read_file(path: &Path) -> Result<Option<ProcessActivity>> {
        match File::open(path) {
            Ok(file) => Ok(Some(serde_json::from_reader(file)?)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn read(&self, session_id: &str) -> Result<Option<Activity>> {
        let path = self.path();
        let lock = match File::open(path.with_extension("lock")) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        lock.lock_shared()?;
        if lock.metadata()?.len() != 0 {
            return Ok(None);
        }
        Ok(Self::read_file(&path)?.and_then(|mut state| state.sessions.remove(session_id)))
    }

    pub fn record(&self, input: HookInput, now: u64) -> Result<()> {
        let starting = matches!(input.event, HookEvent::SessionStart { .. });
        if starting {
            fs::create_dir_all(&self.root)?;
        }
        let path = self.path();
        let lock_path = path.with_extension("lock");
        // Only SessionStart opens a process lifetime. Late subagent events cannot reopen it.
        let lock = match File::options()
            .read(true)
            .write(true)
            .create(starting)
            .truncate(false)
            .open(&lock_path)
        {
            Ok(file) => file,
            Err(error) if !starting && error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        lock.lock()?;
        if matches!(&input.event, HookEvent::SessionEnd { reason } if reason.exits_process()) {
            Self::remove_file(&path)?;
            // Retire handles already waiting on this lock before unlinking its name.
            // Those hooks must not recreate the activity file after cleanup.
            lock.set_len(1)?;
            Self::remove_file(&lock_path)?;
            return Ok(());
        }
        if lock.metadata()?.len() != 0 {
            return Ok(());
        }
        let mut state = Self::read_file(&path)?.unwrap_or_default();
        if matches!(
            input.event,
            HookEvent::SessionEnd {
                reason: SessionEndReason::Clear
            }
        ) {
            state.sessions.remove(&input.session_id);
            if state.sessions.is_empty() {
                Self::remove_file(&path)?;
            } else {
                fs::write(path, serde_json::to_vec(&state)?)?;
            }
            return Ok(());
        }
        let activity = if starting {
            state.sessions.entry(input.session_id).or_default()
        } else {
            let Some(activity) = state.sessions.get_mut(&input.session_id) else {
                return Ok(());
            };
            activity
        };
        activity.apply(input.event, now);
        fs::write(path, serde_json::to_vec(&state)?)?;
        Ok(())
    }

    fn remove_file(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

pub fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}
