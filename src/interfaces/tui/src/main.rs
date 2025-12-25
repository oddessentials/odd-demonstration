//! odd-dashboard - Terminal dashboard for Distributed Task Observatory
//!
//! This is the main entry point that orchestrates the TUI application.
//! All functionality is modularized in the library crate.

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
    Terminal,
};
use std::{
    error::Error,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

// Import from the library modules
use odd_dashboard::{
    App, AppMode, ClusterStatus, SetupProgress,
    TaskCreationStatus, UiLauncherState,
    PrereqStatus,
    LOGO, SPINNER_FRAMES, LOADING_MESSAGES, APP_VERSION,
    check_platform_support, print_version, print_help, run_doctor,
    check_all_prerequisites, has_missing_prerequisites,
    check_cluster_status, run_setup_script, load_ui_registry, open_browser, submit_job,
    get_install_command, copy_to_clipboard, execute_install_with_output,
};

// ============================================================================
// Rendering Functions
// ============================================================================

/// Renders the loading splash screen with animated spinner
fn render_loading_splash<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
    let message = LOADING_MESSAGES[frame_idx / 3 % LOADING_MESSAGES.len()];
    
    terminal.draw(|f| {
        let size = f.size();
        
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Length(18),
                Constraint::Percentage(25),
            ])
            .split(size);
        
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Min(40),
                Constraint::Percentage(25),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        let mut lines: Vec<Line> = Vec::new();
        
        for line in LOGO.lines() {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(Color::Green))
            ]));
        }
        
        lines.push(Line::from(""));
        
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
                    .border_style(Style::default().fg(Color::Green))
                    .title(" oddessentials.com ")
                    .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(splash, center_area);
    })?;
    
    Ok(())
}

/// Renders the launcher view when cluster is not detected
fn render_launcher_view<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
    
    terminal.draw(|f| {
        let size = f.size();
        
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Length(22),  // Increased to fit logo + hints
                Constraint::Percentage(15),
            ])
            .split(size);
        
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Min(50),
                Constraint::Percentage(15),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        let mut lines: Vec<Line> = Vec::new();
        
        // Padding
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        
        // Logo
        for line in LOGO.lines() {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(Color::Green))
            ]));
        }
        lines.push(Line::from(""));
        
        // Status message
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", spinner),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Cluster not detected",
                Style::default().fg(Color::Yellow),
            ),
        ]));
        
        lines.push(Line::from(""));
        
        // Action prompts
        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::Gray)),
            Span::styled("L", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" to launch cluster", Style::default().fg(Color::Gray)),
        ]));
        
        lines.push(Line::from(vec![
            Span::styled("  Press ", Style::default().fg(Color::Gray)),
            Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(" to quit", Style::default().fg(Color::Gray)),
        ]));
        
        let launcher = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" oddessentials.com ")
                    .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(launcher, center_area);
    })?;
    
    Ok(())
}

/// Renders the setup progress view
fn render_setup_progress<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    progress: &SetupProgress,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let spinner = SPINNER_FRAMES[frame_idx % SPINNER_FRAMES.len()];
    
    terminal.draw(|f| {
        let size = f.size();
        
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(20),
                Constraint::Length(1),
            ])
            .split(size);
        
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(5),
                Constraint::Min(70),
                Constraint::Percentage(5),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        let mut lines: Vec<Line> = Vec::new();
        
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        
        let logo_color = if progress.has_error { Color::Red } else { Color::Green };
        
        for line in LOGO.lines() {
            lines.push(Line::from(vec![
                Span::styled(line, Style::default().fg(logo_color))
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        
        let title_style = if progress.has_error {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else if progress.is_complete {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        };
        
        let status_text = if progress.has_error {
            "❌ Setup Failed"
        } else if progress.is_complete {
            "✅ Setup Complete!"
        } else {
            "🚀 Setting up cluster..."
        };
        
        lines.push(Line::from(vec![
            Span::styled(status_text, title_style),
        ]));
        
        lines.push(Line::from(""));
        
        if !progress.is_complete {
            let elapsed_str = if let Some(start) = progress.start_time {
                let elapsed = start.elapsed();
                let mins = elapsed.as_secs() / 60;
                let secs = elapsed.as_secs() % 60;
                format!("  ⏱ {:02}:{:02}", mins, secs)
            } else {
                String::new()
            };
            
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", spinner), Style::default().fg(Color::Yellow)),
                Span::styled(&progress.message, Style::default().fg(Color::White)),
                Span::styled(elapsed_str, Style::default().fg(Color::DarkGray)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}", &progress.message),
                    Style::default().fg(if progress.has_error { Color::Red } else { Color::Cyan }),
                ),
            ]));
            
            if progress.has_error && !progress.error_hint.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  💡 ", Style::default().fg(Color::Yellow)),
                    Span::styled(&progress.error_hint, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                ]));
                
                if !progress.remediation.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("  📋 To fix this:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    ]));
                    
                    for step in &progress.remediation {
                        lines.push(Line::from(vec![
                            Span::styled(format!("     {}", step), Style::default().fg(Color::White)),
                        ]));
                    }
                }
            }
        }
        
        let max_logs = if progress.has_error && !progress.remediation.is_empty() { 3 } else { 5 };
        if !progress.log_lines.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Log:", Style::default().fg(Color::DarkGray)),
            ]));
            let log_start = progress.log_lines.len().saturating_sub(max_logs);
            for log_line in progress.log_lines.iter().skip(log_start) {
                let color = if log_line.starts_with("[ERR]") { Color::Red } else { Color::DarkGray };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {}", log_line.chars().take(65).collect::<String>()),
                        Style::default().fg(color),
                    ),
                ]));
            }
        }
        
        if progress.is_complete {
            lines.push(Line::from(""));
            let continue_text = if progress.has_error {
                "  Press any key to return to launcher..."
            } else {
                "  Press any key to continue to dashboard..."
            };
            lines.push(Line::from(vec![
                Span::styled(continue_text, Style::default().fg(Color::Green)),
            ]));
        }
        
        let border_color = if progress.has_error { Color::Red } else { Color::Green };
        let setup_view = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .title(" Cluster Setup ")
                    .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(setup_view, center_area);
    })?;
    
    Ok(())
}

/// Renders the prerequisite setup view for guided installation
fn render_prerequisite_setup<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &App,
    frame_idx: usize,
) -> Result<(), Box<dyn Error>> {
    let prereqs = check_all_prerequisites();
    
    terminal.draw(|f| {
        let size = f.size();
        
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Min(20),
                Constraint::Percentage(10),
            ])
            .split(size);
        
        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Min(60),
                Constraint::Percentage(10),
            ])
            .split(vertical_center[1]);
        
        let center_area = horizontal_center[1];
        
        let mut lines: Vec<Line> = Vec::new();
        
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ⚙️  Prerequisites Check", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        
        // Show each prerequisite status
        for prereq in &prereqs {
            let (status_icon, status_color) = match &prereq.status {
                PrereqStatus::Installed => ("✅", Color::Green),
                PrereqStatus::Missing => ("❌", Color::Red),
                PrereqStatus::Installing => ("⏳", Color::Yellow),
                PrereqStatus::InstallFailed(_) => ("💥", Color::Red),
            };
            
            let version_str = prereq.version.as_ref()
                .map(|v| format!(" - {}", v))
                .unwrap_or_else(|| " - Not found".to_string());
            
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", status_icon), Style::default().fg(status_color)),
                Span::styled(&prereq.name, Style::default().fg(Color::White)),
                Span::styled(version_str, Style::default().fg(Color::DarkGray)),
            ]));
        }
        
        // Check if any are missing
        let missing: Vec<_> = prereqs.iter().filter(|p| matches!(p.status, PrereqStatus::Missing)).collect();
        
        if !missing.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ─────────────────────────────────", Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  🔧 Install missing prerequisites:", Style::default().fg(Color::Yellow)),
            ]));
            lines.push(Line::from(""));
            
            for (idx, prereq) in missing.iter().enumerate() {
                let selected = idx == app.prereq_state.selected_index;
                let prefix = if selected { "  [>] " } else { "  [ ] " };
                let style = if selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(&prereq.name, style),
                ]));
                
                // Show install command for selected
                if selected {
                    if let Some(cmd) = get_install_command(&prereq.name) {
                        lines.push(Line::from(vec![
                            Span::styled("        ", Style::default()),
                            Span::styled(cmd.clone(), Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }
            
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Press ", Style::default().fg(Color::Gray)),
                Span::styled("ENTER", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::styled(" to execute, ", Style::default().fg(Color::Gray)),
                Span::styled("C", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" to copy, ", Style::default().fg(Color::Gray)),
                Span::styled("ESC", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(" to skip", Style::default().fg(Color::Gray)),
            ]));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ✨ All prerequisites installed!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  Press any key to continue...", Style::default().fg(Color::Gray)),
            ]));
        }
        
        // Show feedback message if any
        if let Some(ref msg) = app.prereq_state.message {
            lines.push(Line::from(""));
            let msg_color = if msg.starts_with("✓") { Color::Green } 
                           else if msg.starts_with("✗") { Color::Red }
                           else { Color::Yellow };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}", msg), Style::default().fg(msg_color)),
            ]));
        }
        
        // Show captured output lines if any
        if !app.prereq_state.output_lines.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ─── Output ───", Style::default().fg(Color::DarkGray)),
            ]));
            for line in app.prereq_state.output_lines.iter().take(8) {
                let (color, text) = if line.starts_with("ERR:") {
                    (Color::Red, line.as_str())
                } else {
                    (Color::DarkGray, line.as_str())
                };
                // Truncate long lines
                let display_text = if text.len() > 60 {
                    format!("{}...", &text[..57])
                } else {
                    text.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}", display_text), Style::default().fg(color)),
                ]));
            }
        }
        
        let prereq_view = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" Prerequisites Setup ")
                    .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            );
        
        f.render_widget(prereq_view, center_area);
    })?;
    
    Ok(())
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() -> Result<(), Box<dyn Error>> {
    // === PHASE 1: Collect args (no external I/O yet) ===
    let args: Vec<String> = std::env::args().collect();
    
    // === PHASE 2: Platform validation (pure computation, no I/O) ===
    if let Err(msg) = check_platform_support() {
        eprintln!("ERROR: {}", msg);
        std::process::exit(1);
    }
    
    // === PHASE 3: CLI dispatch (before terminal initialization) ===
    match args.get(1).map(|s| s.as_str()) {
        Some("--version") | Some("-V") => {
            print_version();
            return Ok(());
        }
        Some("--help") | Some("-h") => {
            print_help();
            return Ok(());
        }
        Some("doctor") => {
            run_doctor();
            return Ok(());
        }
        Some(arg) if arg.starts_with('-') => {
            eprintln!("Unknown option: {}", arg);
            eprintln!("Run 'odd-dashboard --help' for usage.");
            std::process::exit(1);
        }
        _ => {}
    }
    
    // === PHASE 4: Now safe to perform I/O and initialize terminal ===
    let api_url = std::env::var("READ_MODEL_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let gateway_url = std::env::var("GATEWAY_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(api_url.clone(), gateway_url.clone());
    
    // Check if prerequisites are missing - offer guided setup
    if has_missing_prerequisites() {
        app.mode = AppMode::PrerequisiteSetup;
    } else {
        // Check cluster status
        let cluster_status = check_cluster_status();
        
        match cluster_status {
            ClusterStatus::Ready => {
                app.mode = AppMode::Loading;
                
                let loading_done = Arc::new(AtomicBool::new(false));
                let loading_done_clone = Arc::clone(&loading_done);
                
                let api_url_clone = app.api_url.clone();
                let gateway_url_clone = app.gateway_url.clone();
                let handle = thread::spawn(move || {
                    let mut background_app = App::new(api_url_clone, gateway_url_clone);
                    background_app.refresh();
                    loading_done_clone.store(true, Ordering::SeqCst);
                    background_app
                });
                
                let mut frame_idx = 0;
                while !loading_done.load(Ordering::SeqCst) {
                    render_loading_splash(&mut terminal, frame_idx)?;
                    frame_idx += 1;
                    thread::sleep(Duration::from_millis(80));
                    
                    if event::poll(Duration::from_millis(0))? {
                        if let Event::Key(key) = event::read()? {
                            if key.code == KeyCode::Char('q') {
                                disable_raw_mode()?;
                                execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
                                terminal.show_cursor()?;
                                return Ok(());
                            }
                        }
                    }
                }
                
                app = handle.join().expect("Background thread panicked");
                app.mode = AppMode::Dashboard;
            }
            ClusterStatus::NoPods | ClusterStatus::NotFound | ClusterStatus::Error(_) => {
                app.mode = AppMode::Launcher;
            }
        }
    }

    // Main event loop
    let mut frame_idx = 0;
    loop {
        match app.mode {
            AppMode::Loading => {
                render_loading_splash(&mut terminal, frame_idx)?;
            }
            AppMode::Launcher => {
                render_launcher_view(&mut terminal, frame_idx)?;
            }
            AppMode::SetupProgress => {
                let progress = app.setup_progress.lock().unwrap().clone();
                render_setup_progress(&mut terminal, &progress, frame_idx)?;
                
                if progress.is_complete {
                    if event::poll(Duration::from_millis(100))? {
                        if let Event::Key(_) = event::read()? {
                            if progress.has_error {
                                app.mode = AppMode::Launcher;
                            } else {
                                app.refresh();
                                app.mode = AppMode::Dashboard;
                            }
                        }
                    }
                }
            }
            AppMode::PrerequisiteSetup => {
                render_prerequisite_setup(&mut terminal, &app, frame_idx)?;
            }
            AppMode::Dashboard => {
                terminal.draw(|f| {
                    let main_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints([
                            Constraint::Length(15),
                            Constraint::Length(6),
                            Constraint::Min(8),
                            Constraint::Length(1),
                        ])
                        .split(f.size());

                    let header_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Length(32),
                            Constraint::Min(30),
                        ])
                        .split(main_chunks[0]);

                    // Render the ASCII logo
                    let logo_lines: Vec<Line> = LOGO
                        .lines()
                        .map(|line| {
                            Line::from(vec![Span::styled(
                                line,
                                Style::default().fg(Color::Green),
                            )])
                        })
                        .collect();
                    let logo_widget = Paragraph::new(logo_lines)
                        .alignment(Alignment::Center)
                        .block(Block::default().borders(Borders::ALL).title(" oddessentials.com "));
                    f.render_widget(logo_widget, header_chunks[0]);

                    // Right side: Title + Stats
                    let right_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3),
                            Constraint::Min(5),
                        ])
                        .split(header_chunks[1]);

                    let title = Paragraph::new(vec![Line::from(vec![
                        Span::styled(
                            " 📡 Distributed Task Observatory ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                    ])])
                    .block(Block::default().borders(Borders::ALL));
                    f.render_widget(title, right_chunks[0]);

                    // Stats
                    let stats_text = vec![
                        Line::from(vec![
                            Span::raw("  Total Jobs:     "),
                            Span::styled(format!("{}", app.stats.total_jobs), Style::default().fg(Color::Yellow)),
                        ]),
                        Line::from(vec![
                            Span::raw("  Completed:      "),
                            Span::styled(format!("{}", app.stats.completed_jobs), Style::default().fg(Color::Cyan)),
                        ]),
                        Line::from(vec![
                            Span::raw("  Failed:         "),
                            Span::styled(format!("{}", app.stats.failed_jobs), Style::default().fg(Color::Red)),
                        ]),
                        Line::from(vec![
                            Span::raw("  Last Event:     "),
                            Span::styled(&app.stats.last_event_time, Style::default().fg(Color::Blue)),
                        ]),
                    ];
                    let stats_block = Paragraph::new(stats_text)
                        .block(Block::default().title(" Statistics ").borders(Borders::ALL));
                    f.render_widget(stats_block, right_chunks[1]);

                    // Alerts pane
                    let alerts_content: Vec<Line> = if let Some(ref err) = app.alerts_error {
                        vec![Line::from(vec![
                            Span::styled(format!("⚠ {}", err), Style::default().fg(Color::Yellow)),
                        ])]
                    } else if app.alerts.is_empty() {
                        vec![Line::from(vec![
                            Span::styled("✓ No active alerts", Style::default().fg(Color::Cyan)),
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
                                "COMPLETED" => Style::default().fg(Color::Cyan),
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

                    // Help bar with version
                    let help = Paragraph::new(Line::from(vec![
                        Span::styled(" Q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw(" Quit  "),
                        Span::styled("R", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::raw(" Refresh  "),
                        Span::styled("N", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::raw(" New Task  "),
                        Span::styled("U", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::raw(" UIs"),
                        Span::raw("  │  "),
                        Span::styled(format!("v{}", APP_VERSION), Style::default().fg(Color::DarkGray)),
                    ]));
                    f.render_widget(help, main_chunks[3]);
                })?;
            }
            AppMode::TaskCreation => {
                terminal.draw(|f| {
                    let area = f.size();
                    let modal_width = 55u16;
                    let modal_height = 10u16;
                    let x = (area.width.saturating_sub(modal_width)) / 2;
                    let y = (area.height.saturating_sub(modal_height)) / 2;
                    let modal_area = Rect::new(x, y, modal_width, modal_height);
                    
                    f.render_widget(Clear, modal_area);
                    
                    let (status_line, status_color) = match &app.task_state.status {
                        TaskCreationStatus::Editing => ("Type job name, Enter to submit, Esc to cancel", Color::Gray),
                        TaskCreationStatus::Submitting => ("⏳ Submitting...", Color::Yellow),
                        TaskCreationStatus::Success(_) => ("✓ Job created! Press any key to close", Color::Green),
                        TaskCreationStatus::Error(_) => ("✗ Error - Press any key to close", Color::Red),
                    };
                    
                    let input_display = format!("  Job Type: {}_", app.task_state.job_type);
                    let error_msg = if let TaskCreationStatus::Error(e) = &app.task_state.status {
                        format!("  {}", e)
                    } else if let TaskCreationStatus::Success(id) = &app.task_state.status {
                        format!("  Job ID: {}", id)
                    } else {
                        String::new()
                    };
                    
                    let modal_lines = vec![
                        Line::from(""),
                        Line::from(vec![Span::styled(&input_display, Style::default().fg(Color::White))]),
                        Line::from(""),
                        Line::from(vec![Span::styled(status_line, Style::default().fg(status_color))]),
                        if !error_msg.is_empty() {
                            Line::from(vec![Span::styled(&error_msg, Style::default().fg(status_color))])
                        } else {
                            Line::from("")
                        },
                    ];
                    
                    let border_color = match &app.task_state.status {
                        TaskCreationStatus::Success(_) => Color::Green,
                        TaskCreationStatus::Error(_) => Color::Red,
                        _ => Color::Cyan,
                    };
                    
                    let modal = Paragraph::new(modal_lines)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(border_color))
                                .title(" ➕ New Task ")
                                .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
                        );
                    
                    f.render_widget(modal, modal_area);
                })?;
            }
            AppMode::UiLauncher => {
                // Load registry if not already loaded
                if app.launcher_state.registry.is_none() {
                    match load_ui_registry() {
                        Ok(registry) => {
                            app.launcher_state.registry = Some(registry);
                        }
                        Err(e) => {
                            app.launcher_state.error = Some(e.to_string());
                        }
                    }
                }
                
                terminal.draw(|f| {
                    let area = f.size();
                    let modal_width = 60u16;
                    let modal_height = 15u16;
                    let x = (area.width.saturating_sub(modal_width)) / 2;
                    let y = (area.height.saturating_sub(modal_height)) / 2;
                    let modal_area = Rect::new(x, y, modal_width, modal_height);
                    
                    f.render_widget(Clear, modal_area);
                    
                    let mut lines: Vec<Line> = vec![Line::from("")];
                    
                    if let Some(ref error) = app.launcher_state.error {
                        lines.push(Line::from(vec![
                            Span::styled(format!("  ⚠ {}", error), Style::default().fg(Color::Red)),
                        ]));
                        lines.push(Line::from(""));
                    }
                    
                    if let Some(ref registry) = app.launcher_state.registry {
                        for (idx, entry) in registry.entries.iter().enumerate() {
                            let selected = idx == app.launcher_state.selected_index;
                            let style = if selected {
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::White)
                            };
                            let prefix = if selected { " ▶ " } else { "   " };
                            
                            lines.push(Line::from(vec![
                                Span::styled(prefix, style),
                                Span::styled(&entry.emoji, style),
                                Span::styled(format!(" {} ", entry.name), style),
                                Span::styled(format!("(:{}", entry.port), Style::default().fg(Color::DarkGray)),
                                Span::styled(format!("{})", entry.path), Style::default().fg(Color::DarkGray)),
                            ]));
                            
                            if selected {
                                lines.push(Line::from(vec![
                                    Span::raw("      "),
                                    Span::styled(&entry.description, Style::default().fg(Color::Gray)),
                                ]));
                            }
                        }
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("  Loading...", Style::default().fg(Color::Yellow)),
                        ]));
                    }
                    
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled("  ↑/↓ ", Style::default().fg(Color::Yellow)),
                        Span::raw("Navigate  "),
                        Span::styled("Enter ", Style::default().fg(Color::Green)),
                        Span::raw("Open  "),
                        Span::styled("Esc ", Style::default().fg(Color::Red)),
                        Span::raw("Close"),
                    ]));
                    
                    let modal = Paragraph::new(lines)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Cyan))
                                .title(" 🌐 Launch UI ")
                                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                        );
                    
                    f.render_widget(modal, modal_area);
                })?;
            }
        }

        frame_idx = frame_idx.wrapping_add(1);
        thread::sleep(Duration::from_millis(50));

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events, ignore repeat/release
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match app.mode {
                    AppMode::Launcher => {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('l') | KeyCode::Char('L') => {
                                app.mode = AppMode::SetupProgress;
                                let progress = Arc::clone(&app.setup_progress);
                                thread::spawn(move || {
                                    run_setup_script(progress);
                                });
                            }
                            _ => {}
                        }
                    }
                    AppMode::PrerequisiteSetup => {
                        let prereqs = check_all_prerequisites();
                        let missing: Vec<_> = prereqs.iter()
                            .filter(|p| matches!(p.status, PrereqStatus::Missing))
                            .collect();
                        
                        if missing.is_empty() {
                            // All installed, continue to launcher
                            app.prereq_state.message = Some("All prerequisites installed!".to_string());
                            app.mode = AppMode::Launcher;
                        } else {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mode = AppMode::Launcher;
                                }
                                KeyCode::Up => {
                                    if app.prereq_state.selected_index > 0 {
                                        app.prereq_state.selected_index -= 1;
                                    }
                                }
                                KeyCode::Down => {
                                    if app.prereq_state.selected_index < missing.len().saturating_sub(1) {
                                        app.prereq_state.selected_index += 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    // Execute install command and capture output
                                    if let Some(prereq) = missing.get(app.prereq_state.selected_index) {
                                        app.prereq_state.message = Some(format!(
                                            "Installing {}...", prereq.name
                                        ));
                                        app.prereq_state.output_lines.clear();
                                        app.prereq_state.is_installing = true;
                                        
                                        // Execute and capture output
                                        let output = execute_install_with_output(&prereq.name);
                                        app.prereq_state.output_lines = output.lines;
                                        app.prereq_state.is_installing = false;
                                        
                                        if output.success {
                                            app.prereq_state.message = Some(format!(
                                                "✓ {} installed successfully!", prereq.name
                                            ));
                                        } else {
                                            let err_msg = output.error_message.unwrap_or_else(|| "Unknown error".to_string());
                                            app.prereq_state.message = Some(format!(
                                                "✗ Install failed: {}", err_msg
                                            ));
                                        }
                                    }
                                }
                                KeyCode::Char('c') | KeyCode::Char('C') => {
                                    // Copy to clipboard
                                    if let Some(prereq) = missing.get(app.prereq_state.selected_index) {
                                        match copy_to_clipboard(&prereq.name) {
                                            Ok(()) => {
                                                app.prereq_state.message = Some(format!(
                                                    "✓ Copied {} install command to clipboard!", prereq.name
                                                ));
                                            }
                                            Err(e) => {
                                                app.prereq_state.message = Some(format!("✗ {}", e));
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    AppMode::Dashboard => {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('r') => app.refresh(),
                            KeyCode::Char('n') => {
                                app.task_state = odd_dashboard::TaskCreationState::default();
                                app.mode = AppMode::TaskCreation;
                            }
                            KeyCode::Char('u') => {
                                app.launcher_state = UiLauncherState::default();
                                app.mode = AppMode::UiLauncher;
                            }
                            _ => {}
                        }
                    }
                    AppMode::TaskCreation => {
                        match &app.task_state.status {
                            TaskCreationStatus::Editing => {
                                match key.code {
                                    KeyCode::Esc => {
                                        app.mode = AppMode::Dashboard;
                                    }
                                    KeyCode::Enter => {
                                        if !app.task_state.job_type.trim().is_empty() {
                                            app.task_state.status = TaskCreationStatus::Submitting;
                                            let gateway_url = app.gateway_url.clone();
                                            let job_type = app.task_state.job_type.clone();
                                            
                                            match submit_job(&gateway_url, &job_type) {
                                                Ok(job_id) => {
                                                    app.task_state.status = TaskCreationStatus::Success(job_id);
                                                }
                                                Err(e) => {
                                                    app.task_state.status = TaskCreationStatus::Error(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char(c) => {
                                        app.task_state.job_type.push(c);
                                    }
                                    KeyCode::Backspace => {
                                        app.task_state.job_type.pop();
                                    }
                                    _ => {}
                                }
                            }
                            TaskCreationStatus::Submitting => {}
                            TaskCreationStatus::Success(_) | TaskCreationStatus::Error(_) => {
                                app.refresh();
                                app.mode = AppMode::Dashboard;
                            }
                        }
                    }
                    AppMode::UiLauncher => {
                        match key.code {
                            KeyCode::Esc => {
                                app.mode = AppMode::Dashboard;
                            }
                            KeyCode::Up => {
                                if app.launcher_state.selected_index > 0 {
                                    app.launcher_state.selected_index -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if let Some(ref registry) = app.launcher_state.registry {
                                    if app.launcher_state.selected_index < registry.entries.len().saturating_sub(1) {
                                        app.launcher_state.selected_index += 1;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(ref registry) = app.launcher_state.registry {
                                    if let Some(entry) = registry.entries.get(app.launcher_state.selected_index) {
                                        let url = format!("{}:{}{}", registry.base_url, entry.port, entry.path);
                                        match open_browser(&url) {
                                            Ok(()) => {
                                                app.launcher_state.error = None;
                                                app.mode = AppMode::Dashboard;
                                            }
                                            Err(e) => {
                                                app.launcher_state.error = Some(e.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        if key.code == KeyCode::Char('q') {
                            break;
                        }
                    }
                }
            }
        } else if app.mode == AppMode::Dashboard {
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
