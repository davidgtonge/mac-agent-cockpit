use app_core::{
    now_ms, ConversationId, ProcessNodeVm, ProcessRuntimeState, ProcessSample, ResourceBudget,
};
use nix::sys::signal::{kill, killpg, Signal};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("process io: {0}")]
    Io(#[from] std::io::Error),
    #[error("signal error: {0}")]
    Signal(#[from] nix::Error),
    #[error("invalid pid: {0}")]
    InvalidPid(i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcessGroup {
    pub conversation_id: ConversationId,
    pub root_pid: i32,
    pub pgid: i32,
    pub known_descendants: Vec<i32>,
    pub budget: ResourceBudget,
    pub latest_sample: ProcessSample,
    pub throttle_state: ThrottleState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottleState {
    pub enabled: bool,
    pub stopped: bool,
    pub last_pause_ms: u64,
    pub duty_cycle: f32,
}

impl Default for ThrottleState {
    fn default() -> Self {
        Self {
            enabled: true,
            stopped: false,
            last_pause_ms: 0,
            duty_cycle: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
struct PsRow {
    pid: i32,
    ppid: i32,
    pgid: i32,
    cpu: f32,
    rss_kb: u64,
}

#[derive(Default)]
pub struct ProcessSupervisor {
    groups: BTreeMap<ConversationId, AgentProcessGroup>,
    recent_samples: VecDeque<ProcessSample>,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        conversation_id: ConversationId,
        root_pid: i32,
        pgid: i32,
        budget: ResourceBudget,
    ) -> ProcessSample {
        let sample = ProcessSample {
            conversation_id: conversation_id.clone(),
            root_pid,
            pgid,
            cpu_percent: 0.0,
            memory_bytes: 0,
            process_count: 1,
            sampled_at_ms: now_ms(),
            state: ProcessRuntimeState::Running,
            nodes: Vec::new(),
        };
        self.groups.insert(
            conversation_id.clone(),
            AgentProcessGroup {
                conversation_id,
                root_pid,
                pgid,
                known_descendants: vec![root_pid],
                budget,
                latest_sample: sample.clone(),
                throttle_state: ThrottleState::default(),
            },
        );
        sample
    }

    pub fn unregister(&mut self, conversation_id: &ConversationId) {
        self.groups.remove(conversation_id);
    }

    pub fn update_budget(&mut self, conversation_id: &ConversationId, cpu_percent: f32) {
        if let Some(group) = self.groups.get_mut(conversation_id) {
            group.budget.max_cpu_percent = cpu_percent.max(1.0);
        }
    }

    pub fn pause(
        &mut self,
        conversation_id: &ConversationId,
    ) -> Result<ProcessSample, ProcessError> {
        self.signal_group(conversation_id, Signal::SIGSTOP)?;
        Ok(self.mark_state(conversation_id, ProcessRuntimeState::Paused))
    }

    pub fn resume(
        &mut self,
        conversation_id: &ConversationId,
    ) -> Result<ProcessSample, ProcessError> {
        self.signal_group(conversation_id, Signal::SIGCONT)?;
        Ok(self.mark_state(conversation_id, ProcessRuntimeState::Running))
    }

    pub fn kill_group(
        &mut self,
        conversation_id: &ConversationId,
    ) -> Result<ProcessSample, ProcessError> {
        let _ = self.signal_group(conversation_id, Signal::SIGTERM);
        thread::sleep(Duration::from_millis(150));
        let _ = self.signal_group(conversation_id, Signal::SIGKILL);
        Ok(self.mark_state(conversation_id, ProcessRuntimeState::Exited))
    }

    pub fn sample_all(&mut self) -> Result<Vec<ProcessSample>, ProcessError> {
        let ps = read_process_table()?;
        let ids: Vec<_> = self.groups.keys().cloned().collect();
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(sample) = self.sample_one_with_table(&id, &ps) {
                self.recent_samples.push_back(sample.clone());
                while self.recent_samples.len() > 512 {
                    self.recent_samples.pop_front();
                }
                out.push(sample);
            }
        }
        Ok(out)
    }

    pub fn throttle_tick(&mut self) -> Result<Vec<ProcessSample>, ProcessError> {
        let ps = read_process_table()?;
        let ids: Vec<_> = self.groups.keys().cloned().collect();
        let mut samples = Vec::new();
        for id in ids {
            let Some(sample) = self.sample_one_with_table(&id, &ps) else {
                continue;
            };
            let Some(group) = self.groups.get_mut(&id) else {
                continue;
            };
            if sample.state == ProcessRuntimeState::Paused
                || sample.state == ProcessRuntimeState::Exited
            {
                samples.push(sample);
                continue;
            }
            if group.throttle_state.enabled && sample.cpu_percent > group.budget.max_cpu_percent {
                let overage = (sample.cpu_percent / group.budget.max_cpu_percent).min(6.0);
                let pause_ms = ((overage - 1.0) * 120.0).clamp(25.0, 450.0) as u64;
                let _ = signal_group_by_pgid(group.pgid, Signal::SIGSTOP);
                for pid in &group.known_descendants {
                    let _ = kill(Pid::from_raw(*pid), Signal::SIGSTOP);
                }
                thread::sleep(Duration::from_millis(pause_ms));
                let _ = signal_group_by_pgid(group.pgid, Signal::SIGCONT);
                for pid in &group.known_descendants {
                    let _ = kill(Pid::from_raw(*pid), Signal::SIGCONT);
                }
                group.throttle_state.last_pause_ms = pause_ms;
                group.throttle_state.duty_cycle =
                    (group.budget.max_cpu_percent / sample.cpu_percent).clamp(0.05, 1.0);
                group.latest_sample.state = ProcessRuntimeState::Throttling;
            }
            if group.budget.max_memory_bytes > 0
                && sample.memory_bytes > group.budget.max_memory_bytes
            {
                group.latest_sample.state = ProcessRuntimeState::Throttling;
            }
            samples.push(group.latest_sample.clone());
        }
        Ok(samples)
    }

    fn signal_group(
        &mut self,
        conversation_id: &ConversationId,
        signal: Signal,
    ) -> Result<(), ProcessError> {
        let group = self
            .groups
            .get(conversation_id)
            .ok_or(ProcessError::InvalidPid(0))?
            .clone();
        signal_group_by_pgid(group.pgid, signal)?;
        for pid in group.known_descendants {
            let _ = kill(Pid::from_raw(pid), signal);
        }
        Ok(())
    }

    fn mark_state(
        &mut self,
        conversation_id: &ConversationId,
        state: ProcessRuntimeState,
    ) -> ProcessSample {
        let now = now_ms();
        if let Some(group) = self.groups.get_mut(conversation_id) {
            group.latest_sample.state = state;
            group.latest_sample.sampled_at_ms = now;
            return group.latest_sample.clone();
        }
        ProcessSample {
            conversation_id: conversation_id.clone(),
            root_pid: 0,
            pgid: 0,
            cpu_percent: 0.0,
            memory_bytes: 0,
            process_count: 0,
            sampled_at_ms: now,
            state,
            nodes: Vec::new(),
        }
    }

    fn sample_one_with_table(
        &mut self,
        conversation_id: &ConversationId,
        ps: &[PsRow],
    ) -> Option<ProcessSample> {
        let group = self.groups.get_mut(conversation_id)?;
        let mut descendants = collect_descendants(ps, group.root_pid, group.pgid);
        descendants.insert(group.root_pid);
        let mut cpu = 0.0;
        let mut rss_kb = 0_u64;
        let mut live = 0_usize;
        for row in ps {
            if descendants.contains(&row.pid) {
                live += 1;
                cpu += row.cpu;
                rss_kb += row.rss_kb;
            }
        }
        group.known_descendants = descendants.iter().copied().collect();
        let nodes: Vec<ProcessNodeVm> = ps
            .iter()
            .filter(|row| descendants.contains(&row.pid))
            .map(|row| ProcessNodeVm {
                pid: row.pid,
                ppid: row.ppid,
                cpu_percent: row.cpu,
                memory_bytes: row.rss_kb * 1024,
                command: None,
            })
            .collect();
        let state = if live == 0 {
            ProcessRuntimeState::Exited
        } else {
            group.latest_sample.state.clone()
        };
        let sample = ProcessSample {
            conversation_id: conversation_id.clone(),
            root_pid: group.root_pid,
            pgid: group.pgid,
            cpu_percent: cpu,
            memory_bytes: rss_kb * 1024,
            process_count: live,
            sampled_at_ms: now_ms(),
            state,
            nodes,
        };
        group.latest_sample = sample.clone();
        Some(sample)
    }
}

fn signal_group_by_pgid(pgid: i32, signal: Signal) -> Result<(), ProcessError> {
    if pgid <= 0 {
        return Err(ProcessError::InvalidPid(pgid));
    }
    killpg(Pid::from_raw(pgid), signal)?;
    Ok(())
}

fn read_process_table() -> Result<Vec<PsRow>, ProcessError> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,pgid=,pcpu=,rss="])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().filter_map(parse_ps_row).collect())
}

fn parse_ps_row(line: &str) -> Option<PsRow> {
    let parts: Vec<&str> = line.split_whitespace().take(5).collect();
    if parts.len() < 5 {
        return None;
    }
    Some(PsRow {
        pid: parts[0].parse().ok()?,
        ppid: parts[1].parse().ok()?,
        pgid: parts[2].parse().ok()?,
        cpu: parts[3].parse().unwrap_or(0.0),
        rss_kb: parts[4].parse().unwrap_or(0),
    })
}

fn collect_descendants(ps: &[PsRow], root_pid: i32, pgid: i32) -> BTreeSet<i32> {
    let mut out = BTreeSet::new();
    for row in ps {
        if row.pgid == pgid {
            out.insert(row.pid);
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for row in ps {
            if (row.ppid == root_pid || out.contains(&row.ppid)) && out.insert(row.pid) {
                changed = true;
            }
        }
    }
    out
}

pub fn spawn_process_group(
    program: &str,
    args: &[String],
    cwd: &Path,
    background_policy: bool,
) -> Result<Child, ProcessError> {
    let mut command = if cfg!(target_os = "macos") && background_policy {
        let mut c = Command::new("taskpolicy");
        c.arg("-b").arg(program);
        c
    } else {
        Command::new(program)
    };
    if !(cfg!(target_os = "macos") && background_policy) {
        command.args(args);
    } else {
        command.args(args);
    }
    command
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok(command.spawn()?)
}

pub fn spawn_shell_process_group(
    script: &str,
    cwd: &Path,
    background_policy: bool,
) -> Result<Child, ProcessError> {
    spawn_process_group(
        "/bin/zsh",
        &["-lc".to_string(), script.to_string()],
        cwd,
        background_policy,
    )
}
