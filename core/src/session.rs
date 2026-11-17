//! Per-process resource usage recorded over a user-defined sampling session.

use std::time::{Duration, Instant};

use crate::ProcessUsage;

/// Maximum number of processes tracked during one session.
pub const MAX_TRACKED_PROCESSES: usize = 10;

/// Average resource usage of one process over a session.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessAverage {
    /// Process identifier.
    pub pid: u32,
    /// Command line reported by the most recent sample.
    pub command: String,
    /// Mean CPU usage across the samples that included this process, in percent.
    pub average_cpu_percent: f32,
    /// Mean resident memory across the samples that included this process, in bytes.
    pub average_memory_bytes: u64,
}

/// Results of a completed resource session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSummary {
    /// Time between starting and stopping the session.
    pub elapsed: Duration,
    /// Number of polling ticks recorded.
    pub updates: u64,
    /// Per-process averages, ordered by average CPU usage.
    pub processes: Vec<ProcessAverage>,
}

#[derive(Debug)]
struct ProcessAccumulator {
    pid: u32,
    command: String,
    updates: u64,
    cpu_sum: f64,
    memory_sum: u128,
    last_seen: u64,
}

impl ProcessAccumulator {
    fn from_process(process: &ProcessUsage, tick: u64) -> Self {
        Self {
            pid: process.pid,
            command: process.command.clone(),
            updates: 0,
            cpu_sum: 0.0,
            memory_sum: 0,
            last_seen: tick,
        }
    }

    fn record(&mut self, process: &ProcessUsage, tick: u64) {
        self.updates = self.updates.saturating_add(1);
        self.cpu_sum += f64::from(process.cpu_percent);
        self.memory_sum = self
            .memory_sum
            .saturating_add(u128::from(process.memory_bytes));
        self.command.clone_from(&process.command);
        self.last_seen = tick;
    }

    fn average(&self) -> ProcessAverage {
        let updates = self.updates;

        ProcessAverage {
            pid: self.pid,
            command: self.command.clone(),
            average_cpu_percent: if updates == 0 {
                0.0
            } else {
                (self.cpu_sum / updates as f64) as f32
            },
            average_memory_bytes: if updates == 0 {
                0
            } else {
                (self.memory_sum / u128::from(updates)) as u64
            },
        }
    }
}

/// Records per-process CPU and memory samples while a session is active.
#[derive(Debug, Default)]
pub struct ResourceSession {
    started: Option<Instant>,
    updates: u64,
    tick: u64,
    processes: Vec<ProcessAccumulator>,
}

impl ResourceSession {
    /// Returns `true` while samples are being collected.
    pub fn is_active(&self) -> bool {
        self.started.is_some()
    }

    /// Number of polling ticks recorded since the session started.
    pub fn updates(&self) -> u64 {
        self.updates
    }

    /// Starts a new session, discarding any previous counters.
    pub fn start(&mut self) {
        self.started = Some(Instant::now());
        self.reset_counters();
    }

    /// Records the leading processes of one polling tick.
    ///
    /// `processes` must already be ordered by the selected resource. Only the
    /// first [`MAX_TRACKED_PROCESSES`] entries are tracked so the per-tick cost
    /// stays bounded. When the tracked set is full, the process sampled least
    /// recently is replaced so the session follows the current leading
    /// processes.
    pub fn record(&mut self, processes: &[ProcessUsage]) {
        if !self.is_active() {
            return;
        }

        self.tick = self.tick.saturating_add(1);
        self.updates = self.updates.saturating_add(1);

        for process in processes.iter().take(MAX_TRACKED_PROCESSES) {
            if !process.cpu_percent.is_finite() || process.cpu_percent < 0.0 {
                continue;
            }

            if let Some(entry) = self
                .processes
                .iter_mut()
                .find(|entry| entry.pid == process.pid)
            {
                entry.record(process, self.tick);
                continue;
            }

            if self.processes.len() >= MAX_TRACKED_PROCESSES
                && let Some(index) = self
                    .processes
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, entry)| entry.last_seen)
                    .map(|(index, _)| index)
            {
                self.processes.swap_remove(index);
            }

            let mut entry = ProcessAccumulator::from_process(process, self.tick);
            entry.record(process, self.tick);
            self.processes.push(entry);
        }
    }

    /// Stops the session and returns its summary.
    ///
    /// Returns `None` when no session is active. Averages are zero when no
    /// samples were recorded.
    pub fn finish(&mut self) -> Option<SessionSummary> {
        let started = self.started.take()?;

        let mut processes: Vec<ProcessAverage> = self
            .processes
            .iter()
            .map(ProcessAccumulator::average)
            .collect();

        processes.sort_by(|left, right| {
            right
                .average_cpu_percent
                .total_cmp(&left.average_cpu_percent)
                .then_with(|| right.average_memory_bytes.cmp(&left.average_memory_bytes))
                .then_with(|| left.pid.cmp(&right.pid))
        });

        let summary = SessionSummary {
            elapsed: started.elapsed(),
            updates: self.updates,
            processes,
        };

        self.reset_counters();
        Some(summary)
    }

    /// Stops the session without producing a summary.
    pub fn cancel(&mut self) {
        self.started = None;
        self.reset_counters();
    }

    fn reset_counters(&mut self) {
        self.updates = 0;
        self.tick = 0;
        self.processes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, cpu_percent: f32, memory_bytes: u64) -> ProcessUsage {
        ProcessUsage {
            pid,
            cpu_percent,
            memory_bytes,
            cpu_time: String::new(),
            command: format!("process-{pid}"),
        }
    }

    fn active_session() -> ResourceSession {
        let mut session = ResourceSession::default();
        session.start();
        session
    }

    #[test]
    fn recording_requires_an_active_session() {
        let mut session = ResourceSession::default();

        session.record(&[process(1, 10.0, 100)]);

        assert_eq!(session.updates(), 0);
        assert!(!session.is_active());
    }

    #[test]
    fn each_process_has_its_own_average() {
        let mut session = active_session();

        session.record(&[process(1, 10.0, 100), process(2, 40.0, 400)]);
        session.record(&[process(1, 30.0, 300), process(2, 20.0, 200)]);

        let summary = session.finish().expect("active session");

        assert_eq!(summary.updates, 2);
        assert_eq!(summary.processes.len(), 2);

        let first = &summary.processes[0];
        assert_eq!(first.pid, 2);
        assert_eq!(first.average_cpu_percent, 30.0);
        assert_eq!(first.average_memory_bytes, 300);

        let second = &summary.processes[1];
        assert_eq!(second.pid, 1);
        assert_eq!(second.average_cpu_percent, 20.0);
        assert_eq!(second.average_memory_bytes, 200);
    }

    #[test]
    fn averages_count_only_the_samples_that_included_the_process() {
        let mut session = active_session();

        session.record(&[process(1, 10.0, 100), process(2, 20.0, 200)]);
        session.record(&[process(2, 40.0, 400)]);

        let summary = session.finish().expect("active session");

        let first = summary
            .processes
            .iter()
            .find(|entry| entry.pid == 1)
            .expect("process 1");
        assert_eq!(first.average_cpu_percent, 10.0);
        assert_eq!(first.average_memory_bytes, 100);

        let second = summary
            .processes
            .iter()
            .find(|entry| entry.pid == 2)
            .expect("process 2");
        assert_eq!(second.average_cpu_percent, 30.0);
        assert_eq!(second.average_memory_bytes, 300);
    }

    #[test]
    fn tracks_only_the_first_ten_processes() {
        let mut session = active_session();
        let processes: Vec<ProcessUsage> = (0..20).map(|pid| process(pid, 1.0, 1024)).collect();

        session.record(&processes);

        let summary = session.finish().expect("active session");

        assert_eq!(summary.processes.len(), MAX_TRACKED_PROCESSES);
        assert_eq!(
            summary
                .processes
                .iter()
                .map(|entry| entry.pid)
                .collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
    }

    #[test]
    fn replaces_processes_that_leave_the_leading_set() {
        let mut session = active_session();

        let first: Vec<ProcessUsage> = (0..10).map(|pid| process(pid, 1.0, 1)).collect();
        session.record(&first);

        let second: Vec<ProcessUsage> = (2..12).map(|pid| process(pid, 1.0, 1)).collect();
        session.record(&second);

        let summary = session.finish().expect("active session");
        let mut pids: Vec<u32> = summary.processes.iter().map(|entry| entry.pid).collect();
        pids.sort_unstable();

        assert_eq!(pids, (2..12).collect::<Vec<_>>());
    }

    #[test]
    fn starting_again_resets_previous_counters() {
        let mut session = active_session();
        session.record(&[process(1, 10.0, 100)]);

        session.start();
        assert_eq!(session.updates(), 0);

        let summary = session.finish().expect("active session");
        assert_eq!(summary.updates, 0);
        assert!(summary.processes.is_empty());
    }

    #[test]
    fn finishing_without_updates_returns_empty_averages() {
        let mut session = active_session();

        let summary = session.finish().expect("active session");

        assert_eq!(summary.updates, 0);
        assert!(summary.processes.is_empty());
    }

    #[test]
    fn finishing_without_a_session_returns_nothing() {
        let mut session = ResourceSession::default();

        assert!(session.finish().is_none());
    }

    #[test]
    fn cancelling_discards_the_session() {
        let mut session = active_session();
        session.record(&[process(1, 10.0, 100)]);

        session.cancel();

        assert!(!session.is_active());
        assert_eq!(session.updates(), 0);
        assert!(session.finish().is_none());
    }

    #[test]
    fn invalid_cpu_values_are_not_tracked() {
        let mut session = active_session();

        session.record(&[
            process(1, f32::NAN, 100),
            process(2, f32::INFINITY, 100),
            process(3, -1.0, 100),
            process(4, 5.0, 100),
        ]);

        let summary = session.finish().expect("active session");

        assert_eq!(summary.processes.len(), 1);
        assert_eq!(summary.processes[0].pid, 4);
    }
}
