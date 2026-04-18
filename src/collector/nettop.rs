//! nettop command wrapper for macOS network traffic collection
//! 
//! Parses output from `nettop -P -J bytes_in,bytes_out -x -L 1 -l 1`
//! 
//! Output format (CSV):
//!   time,process_name.pid,bytes_in,bytes_out

use super::ProcessTraffic;
use std::process::Command;

pub struct NettopCollector {
    interval: u32,
}

impl NettopCollector {
    pub fn new() -> Self {
        Self { interval: 1 }
    }

    /// Collect process traffic using nettop
    #[cfg(target_os = "macos")]
    pub fn collect(&self) -> Result<Vec<ProcessTraffic>, String> {
        let output = Command::new("nettop")
            .args(["-P", "-J", "bytes_in,bytes_out", "-x", "-L", "1", "-l", "1"])
            .output()
            .map_err(|e| format!("Failed to execute nettop: {}", e))?;

        if !output.status.success() {
            return Err(format!("nettop command failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_output(&stdout)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn collect(&self) -> Result<Vec<ProcessTraffic>, String> {
        Ok(Vec::new())
    }

    /// Parse nettop CSV output
    /// Format: time,process_name.pid,bytes_in,bytes_out
    #[cfg(target_os = "macos")]
    fn parse_output(&self, output: &str) -> Result<Vec<ProcessTraffic>, String> {
        let mut processes = Vec::new();
        
        for line in output.lines() {
            // Skip header line
            if line.starts_with("time,") || line.trim().is_empty() {
                continue;
            }
            
            if let Some(process) = self.parse_line(line)? {
                processes.push(process);
            }
        }
        
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
        // Format: "name.pid" or just "name" if no PID
        let (name, pid) = if let Some(last_dot) = process_field.rfind('.') {
            let name_part = &process_field[..last_dot];
            let pid_part = &process_field[last_dot + 1..];
            let pid = pid_part.parse::<u32>().unwrap_or(0);
            (name_part.to_string(), pid)
        } else {
            (process_field.to_string(), 0)
        };

        // Parse bytes (they have leading spaces)
        let bytes_in = parts[2].trim().parse::<u64>().unwrap_or(0);
        let bytes_out = parts[3].trim().parse::<u64>().unwrap_or(0);

        // Skip kernel_task and zero-traffic entries
        if name == "kernel_task" || (bytes_in == 0 && bytes_out == 0) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        let collector = NettopCollector::new();
        
        // Test parsing
        let line = "00:14:27.376597,syslogd.130, 0, 6425,";
        let result = collector.parse_line(line).unwrap().unwrap();
        assert_eq!(result.name, "syslogd");
        assert_eq!(result.pid, 130);
        assert_eq!(result.bytes_in, 0);
        assert_eq!(result.bytes_out, 6425);
    }
}
