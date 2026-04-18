//! NetGuard - macOS Network Traffic Monitor (TUI Version)
//! 
//! A terminal-based network traffic monitor using ratatui.

mod collector;
mod storage;
mod utils;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::{Terminal, Frame, backend::CrosstermBackend};
use parking_lot::Mutex as ParkMutex;

use collector::{TrafficCollector, ProcessTraffic};
use storage::Database;
use utils::{format_bytes, format_speed};

struct App {
    collector: TrafficCollector,
    database: Database,
    current_speed_in: u64,
    current_speed_out: u64,
    process_list: Vec<ProcessTraffic>,
    traffic_history: Vec<(u64, u64)>,
    running: bool,
}

impl App {
    fn new() -> Result<Self, String> {
        let database = Database::new().map_err(|e| e.to_string())?;
        let collector = TrafficCollector::new();
        
        Ok(Self {
            collector,
            database,
            current_speed_in: 0,
            current_speed_out: 0,
            process_list: Vec::new(),
            traffic_history: Vec::with_capacity(60),
            running: true,
        })
    }
    
    fn update(&mut self) {
        if let Ok(processes) = self.collector.collect() {
            let total_in: u64 = processes.iter().map(|p| p.bytes_in).sum();
            let total_out: u64 = processes.iter().map(|p| p.bytes_out).sum();
            
            self.process_list = processes;
            
            if self.traffic_history.len() >= 20 {
                self.traffic_history.remove(0);
            }
            self.traffic_history.push((total_in, total_out));
            
            // Simple speed calculation (diff from previous)
            if self.traffic_history.len() >= 2 {
                let prev = self.traffic_history[self.traffic_history.len() - 2];
                self.current_speed_in = total_in.saturating_sub(prev.0);
                self.current_speed_out = total_out.saturating_sub(prev.1);
            }
            
            // Record to DB
            self.database.record_traffic(total_in, total_out).ok();
            for p in &self.process_list {
                self.database.record_process_snapshot(p.pid, &p.name, p.bytes_in, p.bytes_out).ok();
            }
        }
    }
}

fn render_ui(f: &mut Frame, app: &App) {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(10),
        ])
        .split(f.size());
    
    // Title
    let title = ratatui::widgets::Paragraph::new("🌐 NetGuard - 网络流量监控")
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title("状态"));
    f.render_widget(title, chunks[0]);
    
    // Speed display
    let speed_text = format!(
        "↑ 上传: {:>10}    ↓ 下载: {:>10}",
        format_speed(app.current_speed_in),
        format_speed(app.current_speed_out)
    );
    let speed = ratatui::widgets::Paragraph::new(speed_text)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::Green))
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title("实时速度"));
    f.render_widget(speed, chunks[1]);
    
    // Process list
    let mut process_text = String::new();
    process_text.push_str(&format!("{:<30} {:>15} {:>15}\n", "进程", "↑ 上传", "↓ 下载"));
    process_text.push_str(&"─".repeat(60));
    process_text.push('\n');
    
    let mut sorted = app.process_list.clone();
    sorted.sort_by(|a, b| b.total().cmp(&a.total()));
    for (i, p) in sorted.iter().take(15).enumerate() {
        let color = match i {
            0 => ratatui::style::Color::Yellow,
            1 => ratatui::style::Color::LightBlue,
            2 => ratatui::style::Color::LightMagenta,
            _ => ratatui::style::Color::White,
        };
        process_text.push_str(&format!(
            "{:<30} {:>15} {:>15}\n",
            p.name.chars().take(28).collect::<String>(),
            format_bytes(p.bytes_out),
            format_bytes(p.bytes_in)
        ));
    }
    
    let process_block = ratatui::widgets::Paragraph::new(process_text)
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::White))
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title("📊 进程排名（按流量）"));
    f.render_widget(process_block, chunks[2]);
}

fn main() -> Result<(), String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| e.to_string())?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;
    
    let mut app = App::new()?;
    
    terminal.draw(|f| render_ui(f, &app)).map_err(|e| e.to_string())?;
    
    loop {
        if event::poll(Duration::from_millis(500)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q') || key.code == KeyCode::Esc {
                    app.running = false;
                    break;
                }
            }
        }
        
        app.update();
        terminal.draw(|f| render_ui(f, &app)).map_err(|e| e.to_string())?;
    }
    
    disable_raw_mode().map_err(|e| e.to_string())?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| e.to_string())?;
    
    println!("感谢使用 NetGuard！");
    Ok(())
}
