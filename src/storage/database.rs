//! SQLite database for traffic history storage
//! 
//! Implements 7-day automatic cleanup (TTL strategy).

use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use chrono::{Utc, Duration};

/// Database handle for traffic data storage
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create new database connection and initialize schema
    pub fn new() -> SqliteResult<Self> {
        let db_path = Self::get_db_path();
        
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        
        let conn = Connection::open(&db_path)?;
        let db = Self { conn };
        db.init_schema()?;
        
        Ok(db)
    }

    /// Get database file path
    fn get_db_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".local/share/net_guard/traffic.db")
    }

    /// Initialize database schema
    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS traffic_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME NOT NULL,
                total_bytes_in INTEGER NOT NULL,
                total_bytes_out INTEGER NOT NULL,
                sample_count INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS process_snapshot (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME NOT NULL,
                pid INTEGER NOT NULL,
                process_name TEXT NOT NULL,
                bytes_in INTEGER NOT NULL,
                bytes_out INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_history_ts ON traffic_history(timestamp);
            CREATE INDEX IF NOT EXISTS idx_process_ts ON process_snapshot(timestamp);
        "#)?;

        self.cleanup_old_data()?;
        
        Ok(())
    }

    /// Record traffic data point
    pub fn record_traffic(&self, bytes_in: u64, bytes_out: u64) -> SqliteResult<()> {
        let now = Utc::now();
        
        self.conn.execute(
            "INSERT INTO traffic_history (timestamp, total_bytes_in, total_bytes_out, sample_count) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![now.to_rfc3339(), bytes_in as i64, bytes_out as i64],
        )?;
        
        Ok(())
    }

    /// Record process snapshot
    pub fn record_process_snapshot(&self, pid: u32, name: &str, bytes_in: u64, bytes_out: u64) -> SqliteResult<()> {
        let now = Utc::now();
        
        self.conn.execute(
            "INSERT INTO process_snapshot (timestamp, pid, process_name, bytes_in, bytes_out) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![now.to_rfc3339(), pid as i64, name, bytes_in as i64, bytes_out as i64],
        )?;
        
        Ok(())
    }

    /// Get hourly traffic data for last N hours
    #[allow(dead_code)]
    pub fn get_hourly_history(&self, hours: i64) -> SqliteResult<Vec<HourlyData>> {
        let cutoff = Utc::now() - Duration::hours(hours);
        
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, total_bytes_in, total_bytes_out, sample_count 
             FROM traffic_history 
             WHERE timestamp >= ?1 
             ORDER BY timestamp ASC"
        )?;
        
        let rows = stmt.query_map([cutoff.to_rfc3339()], |row| {
            Ok(HourlyData {
                timestamp: row.get::<_, String>(0)?,
                bytes_in: row.get::<_, i64>(1)? as u64,
                bytes_out: row.get::<_, i64>(2)? as u64,
                sample_count: row.get::<_, i64>(3)? as u64,
            })
        })?;
        
        let mut data = Vec::new();
        for row in rows {
            if let Ok(d) = row {
                data.push(d);
            }
        }
        
        Ok(data)
    }

    /// Get daily traffic data for last N days
    #[allow(dead_code)]
    pub fn get_daily_history(&self, days: i64) -> SqliteResult<Vec<DailyData>> {
        let cutoff = Utc::now() - Duration::days(days);
        
        let mut stmt = self.conn.prepare(
            "SELECT 
                date(timestamp) as day,
                SUM(total_bytes_in) as total_in,
                SUM(total_bytes_out) as total_out,
                SUM(sample_count) as samples
             FROM traffic_history 
             WHERE timestamp >= ?1 
             GROUP BY date(timestamp)
             ORDER BY day ASC"
        )?;
        
        let rows = stmt.query_map([cutoff.to_rfc3339()], |row| {
            Ok(DailyData {
                day: row.get::<_, String>(0)?,
                bytes_in: row.get::<_, i64>(1)? as u64,
                bytes_out: row.get::<_, i64>(2)? as u64,
                sample_count: row.get::<_, i64>(3)? as u64,
            })
        })?;
        
        let mut data = Vec::new();
        for row in rows {
            if let Ok(d) = row {
                data.push(d);
            }
        }
        
        Ok(data)
    }

    /// Clean up data older than 7 days
    pub fn cleanup_old_data(&self) -> SqliteResult<()> {
        let cutoff = Utc::now() - Duration::days(7);
        
        self.conn.execute(
            "DELETE FROM traffic_history WHERE timestamp < ?1",
            [cutoff.to_rfc3339()],
        )?;
        
        self.conn.execute(
            "DELETE FROM process_snapshot WHERE timestamp < ?1",
            [cutoff.to_rfc3339()],
        )?;
        
        Ok(())
    }
}

/// Hourly aggregated traffic data
#[derive(Debug, Clone)]
pub struct HourlyData {
    pub timestamp: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub sample_count: u64,
}

/// Daily aggregated traffic data
#[derive(Debug, Clone)]
pub struct DailyData {
    pub day: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub sample_count: u64,
}
