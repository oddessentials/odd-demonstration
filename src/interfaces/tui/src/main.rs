use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Terminal,
};
use serde::Deserialize;
use std::{error::Error, io, time::Duration};

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

fn main() -> Result<(), Box<dyn Error>> {
    let api_url = std::env::var("READ_MODEL_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let gateway_url = std::env::var("GATEWAY_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(api_url, gateway_url);
    app.refresh();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(7),
                    Constraint::Length(6),
                    Constraint::Min(8),
                ])
                .split(f.size());

            // Title
            let title = Paragraph::new(vec![Line::from(vec![
                Span::styled(
                    " 📡 Distributed Task Observatory ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ])])
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

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
            f.render_widget(stats_block, chunks[1]);

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
            f.render_widget(alerts_block, chunks[2]);

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
            f.render_widget(table, chunks[3]);
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
