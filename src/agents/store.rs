use super::{HookEvent, HookInput, Result};
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
            HookEvent::SessionEnd => self.ended = true,
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

pub struct ActivityStore {
    root: PathBuf,
}

impl ActivityStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self, session_id: &str) -> PathBuf {
        // Encode the complete ID so it is always a single filename, without collisions.
        let key: String = session_id.bytes().map(|b| format!("{b:02x}")).collect();
        self.root.join(format!("{key}.json"))
    }

    fn read_file(path: &Path) -> Result<Option<Activity>> {
        match File::open(path) {
            Ok(file) => Ok(Some(serde_json::from_reader(file)?)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn read(&self, session_id: &str) -> Result<Option<Activity>> {
        let path = self.path(session_id);
        let lock = match File::open(path.with_extension("lock")) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        lock.lock_shared()?;
        Self::read_file(&path)
    }

    pub fn record(&self, input: HookInput, now: u64) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        let path = self.path(&input.session_id);
        // A separate lock file serializes readers and writers across CLI processes.
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("lock"))?;
        lock.lock()?;
        let mut activity = Self::read_file(&path)?.unwrap_or_default();
        activity.apply(input.event, now);
        fs::write(path, serde_json::to_vec(&activity)?)?;
        Ok(())
    }
}

pub fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs())
}
