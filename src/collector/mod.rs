//! Data collection module
//! 
//! Handles network traffic data collection using nettop.

mod nettop;
mod process;

pub use nettop::NettopCollector;
pub use process::{ProcessInfo, get_process_info};

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Traffic data for a single process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTraffic {
    pub pid: u32,
    pub name: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub icon_path: Option<String>,
}

impl ProcessTraffic {
    pub fn total(&self) -> u64 {
        self.bytes_in + self.bytes_out
    }
}

/// Aggregated traffic collector
pub struct TrafficCollector {
    nettop: NettopCollector,
    last_update: Option<Instant>,
    last_bytes_in: u64,
    last_bytes_out: u64,
}

impl TrafficCollector {
    pub fn new() -> Self {
        Self {
            nettop: NettopCollector::new(),
            last_update: None,
            last_bytes_in: 0,
            last_bytes_out: 0,
        }
    }

    /// Collect current traffic data
    pub fn collect(&mut self) -> Result<Vec<ProcessTraffic>, String> {
        let processes = self.nettop.collect()?;
        
        let mut total_in = 0u64;
        let mut total_out = 0u64;
        
        for p in &processes {
            total_in += p.bytes_in;
            total_out += p.bytes_out;
        }
        
        self.last_bytes_in = total_in;
        self.last_bytes_out = total_out;
        self.last_update = Some(Instant::now());
        
        Ok(processes)
    }

    /// Get current speed (bytes per second)
    #[allow(dead_code)]
    pub fn get_speed(&self) -> (u64, u64) {
        (self.last_bytes_in, self.last_bytes_out)
    }
}
