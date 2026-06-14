use app_core::ProjectId;
use app_workspace::DirtySignal;
use crossbeam_channel::Receiver;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

const DEBOUNCE: Duration = Duration::from_millis(400);
const MIN_FLUSH_INTERVAL: Duration = Duration::from_millis(500);

pub struct WorkspaceDirtyCoalescer {
    rx: Receiver<DirtySignal>,
    pending: HashMap<ProjectId, HashSet<String>>,
    last_signal_at: HashMap<ProjectId, Instant>,
    last_flushed_at: HashMap<ProjectId, Instant>,
}

impl WorkspaceDirtyCoalescer {
    pub fn new(rx: Receiver<DirtySignal>) -> Self {
        Self {
            rx,
            pending: HashMap::new(),
            last_signal_at: HashMap::new(),
            last_flushed_at: HashMap::new(),
        }
    }

    pub fn ingest(&mut self) {
        while let Ok(signal) = self.rx.try_recv() {
            self.pending
                .entry(signal.project_id.clone())
                .or_default()
                .insert(signal.rel_path);
            self.last_signal_at
                .insert(signal.project_id, Instant::now());
        }
    }

    pub fn drain_ready(&mut self) -> Vec<(ProjectId, Vec<String>)> {
        self.ingest();
        let now = Instant::now();
        let mut ready = Vec::new();

        let project_ids: Vec<ProjectId> = self.pending.keys().cloned().collect();
        for project_id in project_ids {
            let Some(last_signal) = self.last_signal_at.get(&project_id) else {
                continue;
            };
            let quiet = now.duration_since(*last_signal) >= DEBOUNCE;
            let max_interval_hit = self
                .last_flushed_at
                .get(&project_id)
                .is_some_and(|last_flush| now.duration_since(*last_flush) >= MIN_FLUSH_INTERVAL);
            if !quiet && !max_interval_hit {
                continue;
            }

            let Some(paths) = self.pending.remove(&project_id) else {
                continue;
            };
            if paths.is_empty() {
                continue;
            }

            self.last_signal_at.remove(&project_id);
            self.last_flushed_at.insert(project_id.clone(), now);
            ready.push((project_id, paths.into_iter().collect()));
        }

        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_core::ProjectId;
    use crossbeam_channel::unbounded;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn coalesces_burst_and_flushes_after_quiet_period() {
        let (tx, rx) = unbounded();
        let mut coalescer = WorkspaceDirtyCoalescer::new(rx);
        let project_id = ProjectId::from_string("project_test");

        tx.send(DirtySignal {
            project_id: project_id.clone(),
            rel_path: "src/a.rs".into(),
        })
        .unwrap();
        tx.send(DirtySignal {
            project_id: project_id.clone(),
            rel_path: "src/b.rs".into(),
        })
        .unwrap();

        assert!(coalescer.drain_ready().is_empty());
        thread::sleep(StdDuration::from_millis(450));
        let batches = coalescer.drain_ready();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].0, project_id);
        assert_eq!(batches[0].1.len(), 2);
    }
}
