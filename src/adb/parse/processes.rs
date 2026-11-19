//! Parsing of the process table printed by `top`.

use android_utility_core::ProcessUsage;

/// Column positions in a `top -o ...` process table.
#[derive(Debug, Clone, Copy)]
struct TopColumns {
    pid: usize,
    cpu: usize,
    memory: usize,
    cpu_time: usize,
    command: usize,
}

impl TopColumns {
    fn from_header(tokens: &[&str]) -> Option<Self> {
        let find = |names: &[&str]| {
            tokens.iter().position(|token| {
                let normalized = token.trim_matches(['[', ']']).trim().to_ascii_lowercase();
                names.contains(&normalized.as_str())
            })
        };

        Some(Self {
            pid: find(&["pid"])?,
            cpu: find(&["%cpu", "cpu"])?,
            memory: find(&["res", "rss"])?,
            cpu_time: find(&["time+", "time"])?,
            command: find(&["args", "cmdline", "command"])?,
        })
    }

    fn parse_row(&self, tokens: &[&str]) -> Option<ProcessUsage> {
        if tokens.len() <= self.command {
            return None;
        }

        let pid = tokens.get(self.pid)?.parse().ok()?;
        let cpu_percent = parse_cpu_percent(tokens.get(self.cpu)?)?;
        let memory_bytes = parse_memory_bytes(tokens.get(self.memory)?)?;
        let cpu_time = (*tokens.get(self.cpu_time)?).to_owned();
        let command = tokens[self.command..].join(" ");

        if command.trim().is_empty() {
            return None;
        }

        Some(ProcessUsage {
            pid,
            cpu_percent,
            memory_bytes,
            cpu_time,
            command,
        })
    }
}

/// Parses the process table printed by `top -o PID,%CPU,RES,TIME+,ARGS`.
///
/// Summary lines before the table are ignored. The column header is located
/// from the output so that devices adding or reordering columns keep working.
pub fn parse_top_processes(output: &str) -> Vec<ProcessUsage> {
    let mut columns = None;
    let mut processes = Vec::new();

    for line in output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        // The sorted column is wrapped in brackets, which can remove the space
        // before it. Split those for the header without changing data rows.
        if line.contains("PID") {
            let spaced_header = line.replace('[', " [").replace(']', "] ");
            let header_tokens: Vec<&str> = spaced_header.split_whitespace().collect();

            if let Some(header) = TopColumns::from_header(&header_tokens) {
                columns = Some(header);
                continue;
            }
        }

        let Some(columns) = &columns else {
            continue;
        };

        if let Some(process) = columns.parse_row(&tokens) {
            processes.push(process);
        }
    }

    processes
}

fn parse_cpu_percent(text: &str) -> Option<f32> {
    let value: f64 = text.trim_end_matches('%').replace(',', ".").parse().ok()?;

    if value.is_finite() && value >= 0.0 {
        Some(value as f32)
    } else {
        None
    }
}

fn parse_memory_bytes(text: &str) -> Option<u64> {
    let text = text.trim().replace(',', ".");

    let suffix_start = text
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_ascii_alphabetic())
        .map_or(0, |(index, character)| index + character.len_utf8());

    let (number, suffix) = text.split_at(suffix_start);
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "" => 1024,
        "b" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" => 1024_u64.pow(4),
        _ => return None,
    };

    let value: f64 = number.parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }

    Some((value * multiplier as f64).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_top_process_table() {
        let processes = parse_top_processes(
            "Tasks: 550 total,   2 running, 548 sleeping,   0 stopped,   0 zombie\n\
             Mem:      5.5G total,      5.2G used,    300M free\n\
             800%cpu  62%user   2%nice  29%sys 707%idle   0%iow   0%irq   3%sirq   0%host\n\
             PID %CPU RES TIME+ ARGS\n\
             1604 8.0 284M 1:04.08 system_server\n\
             2201 25.3 120M 00:12.45 com.example.app\n\
             42 0.0 12.5M 00:00.01 /system/bin/surfaceflinger --flag value\n",
        );

        assert_eq!(processes.len(), 3);
        assert_eq!(processes[0].pid, 1604);
        assert_eq!(processes[0].cpu_percent, 8.0);
        assert_eq!(processes[0].memory_bytes, 284 * 1024 * 1024);
        assert_eq!(processes[0].cpu_time, "1:04.08");
        assert_eq!(processes[0].command, "system_server");
        assert_eq!(processes[2].memory_bytes, (12.5 * 1024.0 * 1024.0) as u64);
        assert_eq!(
            processes[2].command,
            "/system/bin/surfaceflinger --flag value"
        );
    }

    #[test]
    fn parses_top_columns_in_any_order() {
        let processes = parse_top_processes(
            "ARGS RES PID %CPU TIME+\n\
             com.example 1.0G 99 12.5 00:01.00\n",
        );

        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].pid, 99);
        assert_eq!(processes[0].cpu_percent, 12.5);
        assert_eq!(processes[0].memory_bytes, 1024 * 1024 * 1024);
    }

    #[test]
    fn parses_header_with_attached_sort_bracket() {
        let processes = parse_top_processes(
            "Tasks: 927 total,   1 running, 926 sleeping\n\
             800%cpu  13%user   3%nice  30%sys 747%idle\n\
             PID[%CPU] RES     TIME+ ARGS\n\
             31438  3.3 358M   7:23.60 com.google.android.gms\n",
        );

        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].pid, 31438);
        assert_eq!(processes[0].cpu_percent, 3.3);
        assert_eq!(processes[0].memory_bytes, 358 * 1024 * 1024);
        assert_eq!(processes[0].command, "com.google.android.gms");
    }

    #[test]
    fn skips_malformed_top_rows() {
        assert!(parse_top_processes("").is_empty());
        assert!(parse_top_processes("Tasks: 1 total\n").is_empty());

        let processes = parse_top_processes(
            "PID %CPU RES TIME+ ARGS\n\
             not-a-pid 1.0 1M 00:00.01 broken\n\
             10 broken 1M 00:00.01 broken\n\
             11 1.0 broken 00:00.01 broken\n\
             12 1.0 1M 00:00.01\n\
             13 -1.0 1M 00:00.01 broken\n",
        );

        assert!(processes.is_empty());
    }

    #[test]
    fn parses_memory_suffixes() {
        assert_eq!(parse_memory_bytes("0"), Some(0));
        assert_eq!(parse_memory_bytes("512"), Some(512 * 1024));
        assert_eq!(parse_memory_bytes("512K"), Some(512 * 1024));
        assert_eq!(parse_memory_bytes("1.5M"), Some(1_572_864));
        assert_eq!(parse_memory_bytes("2GiB"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_memory_bytes("512B"), Some(512));
        assert_eq!(parse_memory_bytes("broken"), None);
        assert_eq!(parse_memory_bytes("-1M"), None);
    }
}
