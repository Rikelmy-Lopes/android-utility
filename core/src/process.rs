//! Device process usage reported by `top`.

/// Resource used to order the process list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSort {
    /// Highest CPU usage first.
    Cpu,
    /// Highest resident memory usage first.
    Memory,
}

/// Resource usage of one process running on the device.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessUsage {
    /// Process identifier.
    pub pid: u32,
    /// CPU usage in percent. Values above 100 are possible on multi-core devices.
    pub cpu_percent: f32,
    /// Resident memory in bytes.
    pub memory_bytes: u64,
    /// Accumulated CPU time as reported by the device.
    pub cpu_time: String,
    /// Command line reported by the device.
    pub command: String,
}

/// Sorts processes in descending order of the selected resource.
pub fn sort_processes(processes: &mut [ProcessUsage], sort: ProcessSort) {
    match sort {
        ProcessSort::Cpu => processes.sort_by(|left, right| {
            right
                .cpu_percent
                .total_cmp(&left.cpu_percent)
                .then_with(|| right.memory_bytes.cmp(&left.memory_bytes))
                .then_with(|| left.pid.cmp(&right.pid))
        }),
        ProcessSort::Memory => processes.sort_by(|left, right| {
            right
                .memory_bytes
                .cmp(&left.memory_bytes)
                .then_with(|| right.cpu_percent.total_cmp(&left.cpu_percent))
                .then_with(|| left.pid.cmp(&right.pid))
        }),
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
            cpu_time: "00:00.00".to_owned(),
            command: format!("process-{pid}"),
        }
    }

    #[test]
    fn sorts_by_cpu_descending() {
        let mut processes = vec![
            process(1, 1.5, 500),
            process(2, 25.0, 100),
            process(3, 10.0, 900),
        ];

        sort_processes(&mut processes, ProcessSort::Cpu);

        assert_eq!(
            processes.iter().map(|item| item.pid).collect::<Vec<_>>(),
            [2, 3, 1]
        );
    }

    #[test]
    fn sorts_by_memory_descending() {
        let mut processes = vec![
            process(1, 1.5, 500),
            process(2, 25.0, 100),
            process(3, 10.0, 900),
        ];

        sort_processes(&mut processes, ProcessSort::Memory);

        assert_eq!(
            processes.iter().map(|item| item.pid).collect::<Vec<_>>(),
            [3, 1, 2]
        );
    }

    #[test]
    fn breaks_ties_deterministically() {
        let mut processes = vec![process(20, 5.0, 100), process(10, 5.0, 100)];

        sort_processes(&mut processes, ProcessSort::Cpu);

        assert_eq!(
            processes.iter().map(|item| item.pid).collect::<Vec<_>>(),
            [10, 20]
        );
    }
}
