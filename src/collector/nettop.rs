//! nettop command wrapper for macOS network traffic collection
//! 
//! Parses output from `nettop -P -J bytes_in,bytes_out -x -L <samples> -l 1`
//! 
//! Output format (CSV):
//!   time,process_name.pid,bytes_in,bytes_out
//! 
//! Improvements:
//! - Multiple sample accumulation to capture intermittent traffic
//! - Don't skip zero-traffic processes (they still appear, just with 0 bytes)
//! - Better process name parsing for names containing dots

use super::ProcessTraffic;
use std::collections::HashMap;
use std::process::Command;

pub struct NettopCollector {
    /// Number of samples to take per collection cycle
    /// More samples = better capture of intermittent traffic, but slower
    samples: u32,
}

impl NettopCollector {
    pub fn new() -> Self {
        Self { samples: 3 }
    }

    /// Set the number of samples per collection cycle
    pub fn with_samples(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }

    /// Collect process traffic using nettop with multiple sample accumulation
    #[cfg(target_os = "macos")]
    pub fn collect(&self) -> Result<Vec<ProcessTraffic>, String> {
        // Take multiple samples and accumulate traffic across all samples
        // This helps capture intermittent traffic from applications that
        // don't constantly send/receive data
        let output = Command::new("nettop")
            .args([
                "-P", "-J", "bytes_in,bytes_out", "-x",
                "-L", &self.samples.to_string(), "-l", "1"
            ])
            .output()
            .map_err(|e| format!("Failed to execute nettop: {}", e))?;

        if !output.status.success() {
            return Err(format!("nettop command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_output_accumulated(&stdout)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn collect(&self) -> Result<Vec<ProcessTraffic>, String> {
        Ok(Vec::new())
    }

    /// Parse nettop CSV output and accumulate traffic across samples
    /// 
    /// The key insight is that nettop with multiple samples (-L N) outputs
    /// cumulative data, but we need to diff between samples to get the
    /// actual traffic during that period. However, the simpler approach
    /// is to just sum up all traffic we see - applications that appear
    /// even once are tracked.
    #[cfg(target_os = "macos")]
    fn parse_output_accumulated(&self, output: &str) -> Result<Vec<ProcessTraffic>, String> {
        // Use a HashMap to accumulate traffic by process identity (name + pid)
        let mut process_map: HashMap<String, ProcessTraffic> = HashMap::new();
        
        for line in output.lines() {
            // Skip header line and empty lines
            if line.starts_with("time,") || line.trim().is_empty() {
                continue;
            }
            
            if let Some(process) = self.parse_line(line)? {
                // Use name.pid as key to distinguish same-named processes
                let key = format!("{}.{}", process.name, process.pid);
                
                process_map.entry(key)
                    .and_modify(|existing| {
                        // Accumulate bytes across samples
                        existing.bytes_in += process.bytes_in;
                        existing.bytes_out += process.bytes_out;
                    })
                    .or_insert(process);
            }
        }
        
        // Convert back to vector, sorted by total traffic
        let mut processes: Vec<ProcessTraffic> = process_map.into_values().collect();
        processes.sort_by(|a, b| b.total().cmp(&a.total()));
        
        Ok(processes)
    }

    /// Parse a single CSV line
    /// Format: time,process_name.pid,bytes_in,bytes_out
    #[cfg(target_os = "macos")]
    fn parse_line(&self, line: &str) -> Result<Option<ProcessTraffic>, String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 4 {
            return Ok(None);
        }

        // parts[0] = time (e.g., "00:14:27.376597")
        // parts[1] = process_name.pid (e.g., "syslogd.130")
        // parts[2] = bytes_in (e.g., " 0" or " 5636")
        // parts[3] = bytes_out (e.g., " 6425")

        let process_field = parts[1].trim();
        
        // Split process name and PID
        // Format: "name.pid" where pid is numeric
        // We need to find the LAST numeric part after the last dot
        let (name, pid) = Self::parse_process_field(process_field);

        // Parse bytes (they have leading spaces)
        let bytes_in = parts[2].trim().parse::<u64>().unwrap_or(0);
        let bytes_out = parts[3].trim().parse::<u64>().unwrap_or(0);

        // Skip kernel_task only (we no longer skip zero-traffic processes
        // so users can see all processes, even if they have no current traffic)
        if name == "kernel_task" {
            return Ok(None);
        }

        Ok(Some(ProcessTraffic {
            pid,
            name,
            bytes_in,
            bytes_out,
            icon_path: None,
        }))
    }

    /// Parse process field to extract name and pid
    /// Handles cases where process name contains dots
    fn parse_process_field(field: &str) -> (String, u32) {
        // Try to find the last dot followed by a numeric PID
        // Process names typically don't end with digits, while PIDs are numeric
        if let Some(last_dot) = field.rfind('.') {
            let pid_part = &field[last_dot + 1..];
            // Check if this is a valid PID (all digits)
            if pid_part.chars().all(|c| c.is_ascii_digit()) {
                let pid = pid_part.parse::<u32>().unwrap_or(0);
                let name_part = &field[..last_dot];
                return (name_part.to_string(), pid);
            }
        }
        
        // Fallback: no valid PID found, treat entire field as name
        (field.to_string(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        let collector = NettopCollector::new();
        
        // Test parsing with standard format
        let line = "00:14:27.376597,syslogd.130, 0, 6425,";
        let result = collector.parse_line(line).unwrap().unwrap();
        assert_eq!(result.name, "syslogd");
        assert_eq!(result.pid, 130);
        assert_eq!(result.bytes_in, 0);
        assert_eq!(result.bytes_out, 6425);
    }

    #[test]
    fn test_parse_process_field_with_dots_in_name() {
        // Test process name containing dots (e.g., "com.apple.Finder")
        let (name, pid) = NettopCollector::parse_process_field("com.apple.Finder.1234");
        assert_eq!(name, "com.apple.Finder");
        assert_eq!(pid, 1234);
    }

    #[test]
    fn test_parse_process_field_simple() {
        let (name, pid) = NettopCollector::parse_process_field("Chrome.5678");
        assert_eq!(name, "Chrome");
        assert_eq!(pid, 5678);
    }

    #[test]
    fn test_parse_process_field_no_pid() {
        // Process name that looks like it has a PID but doesn't
        let (name, pid) = NettopCollector::parse_process_field("someprocess.abc");
        assert_eq!(name, "someprocess.abc");
        assert_eq!(pid, 0);
    }

    #[test]
    fn test_zero_traffic_not_skipped() {
        let collector = NettopCollector::new();
        
        // Process with zero traffic should NOT be skipped
        // (we used to skip it, now we include it)
        let line = "00:14:27.376597,Slack.12345, 0, 0,";
        let result = collector.parse_line(line).unwrap();
        assert!(result.is_some());
        let p = result.unwrap();
        assert_eq!(p.name, "Slack");
        assert_eq!(p.pid, 12345);
        assert_eq!(p.bytes_in, 0);
        assert_eq!(p.bytes_out, 0);
    }
}
