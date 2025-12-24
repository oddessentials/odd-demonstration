use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Terminal,
};
use serde::Deserialize;
use std::{error::Error, io, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::Duration};

/// ASCII art logo for the Distributed Task Observatory
const LOGO: &str = r#"
            .....        
         .#########.     
       .#####      ##    
      ####+###+    ##+   
    ######  +###+  ###   
  #### -######+######+   
+###.   +####.  +###+    
 ..   -###+########.     
    .###+   .####.       
     -+    +###.         
         -###-           
         -#-             
"#;

/// Animated spinner frames (Braille dots pattern)
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Loading messages that cycle for visual interest
const LOADING_MESSAGES: &[&str] = &[
    "Connecting to services",
    "Fetching statistics",
    "Loading job data",
    "Checking alerts",
];

#[derive(Deserialize, Debug, Clone, Default)]
struct Stats {
    #[serde(rename = "totalJobs")]
    total_jobs: i64,
    #[serde(rename = "completedJobs")]
    completed_jobs: i64,
    #[serde(rename = "failedJobs")]
    failed_jobs: i64,
    #[serde(rename = "lastEventTime")]
    last_event_time: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Job {
    id: String,
    #[serde(rename = "type")]
    job_type: String,
    status: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Deserialize, Debug, Clone)]
struct AlertLabels {
    alertname: Option<String>,
    severity: Option<String>,
    service: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Alert {
    labels: AlertLabels,
}

struct App {
    stats: Stats,
    jobs: Vec<Job>,
    alerts: Vec<Alert>,
    alerts_error: Option<String>,
    api_url: String,
    gateway_url: String,
    alert_retry_count: u8,
}

const MAX_ALERT_RETRIES: u8 = 3;

impl App {
    fn new(api_url: String, gateway_url: String) -> App {
        App {
            stats: Stats::default(),
            jobs: Vec::new(),
            alerts: Vec::new(),
            alerts_error: None,
            api_url,
            gateway_url,
            alert_retry_count: 0,
        }
    }

    fn refresh(&mut self) {
        // Fetch stats
        if let Ok(resp) = reqwest::blocking::get(format!("{}/stats", self.api_url)) {
            if let Ok(stats) = resp.json::<Stats>() {
                self.stats = stats;
            }
        }

        // Fetch recent jobs
        if let Ok(resp) = reqwest::blocking::get(format!("{}/jobs/recent", self.api_url)) {
            if let Ok(jobs) = resp.json::<Vec<Job>>() {
                self.jobs = jobs;
            }
        }

        // Fetch alerts with bounded retries (graceful degradation)
        if self.alert_retry_count < MAX_ALERT_RETRIES {
            match reqwest::blocking::Client::new()
                .get(format!("{}/proxy/alerts", self.gateway_url))
                .timeout(Duration::from_secs(2))
                .send()
            {
                Ok(resp) => {
                    if let Ok(alerts) = resp.json::<Vec<Alert>>() {
                        self.alerts = alerts;
                        self.alerts_error = None;
                        self.alert_retry_count = 0; // Reset on success
                    }
                }
                Err(e) => {
                    self.alert_retry_count += 1;
                    if self.alert_retry_count >= MAX_ALERT_RETRIES {
                        self.alerts_error = Some(format!("Unavailable ({})", e));
                    }
                }
            }
        }
    }
}

/// Renders the loading splash screen with animated spinner
fn render_loading_splash<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
    let message = LOADING_MESSAGES[frame_idx / 3 % LOADING_MESSAGES.len()];
    
    terminal.draw(|f| {
        let size = f.size();
        
        // Center the content vertically
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(18),
                Constraint::Percentage(25),
            ])
            .split(size);
        
        // Center the content horizontally
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Min(40),
                Constraint::Percentage(25),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        // Build the splash content
        let mut lines: Vec<Line> = Vec::new();
        
        // Add logo lines with cyan color
        for line in LOGO.lines() {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(Color::Cyan))
            ]));
        }
        
        // Add spacing
        lines.push(Line::from(""));
        
        // Add animated loading line with spinner
        let dots = ".".repeat((frame_idx % 4) + 1);
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", spinner),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}{}", message, dots),
                Style::default().fg(Color::White),
            ),
        ]));
        
        // Add subtle branding line
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "Distributed Task Observatory",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));
        
        let splash = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" oddessentials.com ")
                    .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(splash, center_area);
    })?;
    
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let api_url = std::env::var("READ_MODEL_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let gateway_url = std::env::var("GATEWAY_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and spawn background refresh
    let mut app = App::new(api_url, gateway_url);
    let loading_done = Arc::new(AtomicBool::new(false));
    let loading_done_clone = Arc::clone(&loading_done);
    
    // Spawn refresh in background thread
    let api_url_clone = app.api_url.clone();
    let gateway_url_clone = app.gateway_url.clone();
    let handle = thread::spawn(move || {
        let mut background_app = App::new(api_url_clone, gateway_url_clone);
        background_app.refresh();
        loading_done_clone.store(true, Ordering::SeqCst);
        background_app
    });
    
    // Animated loading splash while data loads
    let mut frame_idx = 0;
    while !loading_done.load(Ordering::SeqCst) {
        render_loading_splash(&mut terminal, frame_idx)?;
        frame_idx += 1;
        thread::sleep(Duration::from_millis(80));
        
        // Allow user to quit during loading with 'q'
        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    return Ok(());
                }
            }
        }
    }
    
    // Get the loaded app data from background thread
    app = handle.join().expect("Background thread panicked");

    loop {
        terminal.draw(|f| {
            // Main vertical layout
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(15), // Header with logo + title + stats (smaller logo)
                    Constraint::Length(6),  // Alerts
                    Constraint::Min(8),     // Jobs table
                ])
                .split(f.size());

            // Header area: Logo on left, Title+Stats on right
            let header_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(32), // Logo width (smaller)
                    Constraint::Min(30),    // Title + Stats
                ])
                .split(main_chunks[0]);

            // Render the ASCII logo (centered)
            let logo_lines: Vec<Line> = LOGO
                .lines()
                .map(|line| {
                    Line::from(vec![Span::styled(
                        line,
                        Style::default().fg(Color::Cyan),
                    )])
                })
                .collect();
            let logo_widget = Paragraph::new(logo_lines)
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title(" oddessentials.com "));
            f.render_widget(logo_widget, header_chunks[0]);

            // Right side: Title + Stats stacked vertically
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Title
                    Constraint::Min(5),    // Stats
                ])
                .split(header_chunks[1]);

            // Title
            let title = Paragraph::new(vec![Line::from(vec![
                Span::styled(
                    " 📡 Distributed Task Observatory ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ])])
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, right_chunks[0]);

            // Stats
            let stats_text = vec![
                Line::from(vec![
                    Span::raw("  Total Jobs:     "),
                    Span::styled(
                        format!("{}", app.stats.total_jobs),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  Completed:      "),
                    Span::styled(
                        format!("{}", app.stats.completed_jobs),
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  Failed:         "),
                    Span::styled(
                        format!("{}", app.stats.failed_jobs),
                        Style::default().fg(Color::Red),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  Last Event:     "),
                    Span::styled(&app.stats.last_event_time, Style::default().fg(Color::Blue)),
                ]),
            ];
            let stats_block = Paragraph::new(stats_text)
                .block(Block::default().title(" Statistics ").borders(Borders::ALL));
            f.render_widget(stats_block, right_chunks[1]);

            // Alerts pane (with graceful degradation)
            let alerts_content: Vec<Line> = if let Some(ref err) = app.alerts_error {
                vec![Line::from(vec![
                    Span::styled(format!("⚠ {}", err), Style::default().fg(Color::Yellow)),
                ])]
            } else if app.alerts.is_empty() {
                vec![Line::from(vec![
                    Span::styled("✓ No active alerts", Style::default().fg(Color::Green)),
                ])]
            } else {
                app.alerts.iter().take(3).map(|alert| {
                    let name = alert.labels.alertname.as_deref().unwrap_or("Unknown");
                    let severity = alert.labels.severity.as_deref().unwrap_or("warning");
                    let service = alert.labels.service.as_deref().unwrap_or("-");
                    let color = if severity == "critical" { Color::Red } else { Color::Yellow };
                    Line::from(vec![
                        Span::styled(format!("🚨 {} ", name), Style::default().fg(color)),
                        Span::styled(format!("[{}]", service), Style::default().fg(Color::Gray)),
                    ])
                }).collect()
            };
            let alerts_block = Paragraph::new(alerts_content)
                .block(Block::default().title(format!(" Alerts ({}) ", app.alerts.len())).borders(Borders::ALL));
            f.render_widget(alerts_block, main_chunks[1]);

            // Jobs table
            let header = Row::new(vec!["ID", "Type", "Status", "Created"])
                .style(Style::default().fg(Color::Yellow))
                .bottom_margin(1);

            let rows: Vec<Row> = app
                .jobs
                .iter()
                .take(10)
                .map(|job| {
                    let status_style = match job.status.as_str() {
                        "COMPLETED" => Style::default().fg(Color::Green),
                        "FAILED" => Style::default().fg(Color::Red),
                        "PENDING" => Style::default().fg(Color::Yellow),
                        _ => Style::default(),
                    };
                    Row::new(vec![
                        Cell::from(job.id.chars().take(8).collect::<String>()),
                        Cell::from(job.job_type.clone()),
                        Cell::from(job.status.clone()).style(status_style),
                        Cell::from(job.created_at.clone()),
                    ])
                })
                .collect();

            let widths = [
                Constraint::Length(10),
                Constraint::Length(20),
                Constraint::Length(12),
                Constraint::Min(25),
            ];
            let table = Table::new(rows)
            .header(header)
            .widths(&widths)
            .block(Block::default().title(" Recent Jobs ").borders(Borders::ALL));
            f.render_widget(table, main_chunks[2]);
        })?;

        // Handle input
        if event::poll(Duration::from_secs(2))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
                if key.code == KeyCode::Char('r') {
                    app.alert_retry_count = 0; // Reset retries on manual refresh
                    app.refresh();
                }
            }
        } else {
            app.refresh();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

// Unit tests - deterministic, no network/UI dependencies
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_default() {
        let stats = Stats::default();
        assert_eq!(stats.total_jobs, 0);
        assert_eq!(stats.completed_jobs, 0);
        assert_eq!(stats.failed_jobs, 0);
        assert!(stats.last_event_time.is_empty());
    }

    #[test]
    fn test_app_new() {
        let app = App::new("http://localhost:8080".to_string(), "http://localhost:3000".to_string());
        assert_eq!(app.api_url, "http://localhost:8080");
        assert_eq!(app.gateway_url, "http://localhost:3000");
        assert!(app.jobs.is_empty());
        assert!(app.alerts.is_empty());
        assert!(app.alerts_error.is_none());
        assert_eq!(app.alert_retry_count, 0);
    }

    #[test]
    fn test_max_alert_retries_constant() {
        // Verify the retry limit is reasonable
        assert!(MAX_ALERT_RETRIES >= 1);
        assert!(MAX_ALERT_RETRIES <= 10);
    }

    #[test]
    fn test_job_deserialization() {
        let json = r#"{"id": "abc123", "type": "PROCESS", "status": "COMPLETED", "createdAt": "2024-01-01T00:00:00Z"}"#;
        let job: Job = serde_json::from_str(json).expect("Failed to deserialize Job");
        assert_eq!(job.id, "abc123");
        assert_eq!(job.job_type, "PROCESS");
        assert_eq!(job.status, "COMPLETED");
        assert_eq!(job.created_at, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_stats_deserialization() {
        let json = r#"{"totalJobs": 100, "completedJobs": 90, "failedJobs": 10, "lastEventTime": "2024-01-01T12:00:00Z"}"#;
        let stats: Stats = serde_json::from_str(json).expect("Failed to deserialize Stats");
        assert_eq!(stats.total_jobs, 100);
        assert_eq!(stats.completed_jobs, 90);
        assert_eq!(stats.failed_jobs, 10);
        assert_eq!(stats.last_event_time, "2024-01-01T12:00:00Z");
    }
}
