//! Main TUI application state and event loop

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::Block;
use ratatui::style::Style;
use ratatui::{Frame, Terminal};

use crate::parser::Finding;
use crate::planner::{PerspectiveResult, UnifiedPlan};

use super::views::{
    draw_findings_view, draw_plan_view, FindingsViewState, PlanPane, PlanViewState,
};
use super::widgets::{
    draw_command_palette, draw_status_bar, copy_to_clipboard, StatusTone, COLOR_BG,
};

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const PAGE_JUMP: usize = 10;

/// Configuration for launching the TUI
#[derive(Clone)]
pub struct TuiConfig {
    /// Initial findings to display (if any)
    pub findings: Vec<Finding>,

    /// Initial perspective results (if planning)
    pub perspective_results: Vec<PerspectiveResult>,

    /// Unified plan (if available)
    pub unified_plan: Option<UnifiedPlan>,

    /// Path to findings JSON for live reload
    pub findings_path: Option<PathBuf>,

    /// Start in plan mode vs findings mode
    pub start_in_plan_mode: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            findings: Vec::new(),
            perspective_results: Vec::new(),
            unified_plan: None,
            findings_path: None,
            start_in_plan_mode: false,
        }
    }
}

/// Which view is active
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewKind {
    #[default]
    Findings,
    Plan,
}

impl ViewKind {
    fn label(&self) -> &'static str {
        match self {
            ViewKind::Findings => "findings",
            ViewKind::Plan => "plan",
        }
    }
}

/// Main application state
pub struct App {
    /// Current view
    current_view: ViewKind,

    /// Findings view state
    findings_state: FindingsViewState,

    /// Plan view state
    plan_state: PlanViewState,

    /// Path to findings file for reload
    findings_path: Option<PathBuf>,

    /// Status message and tone
    status_message: String,
    status_tone: StatusTone,

    /// Command mode state
    command_mode: bool,
    command_buffer: String,

    /// Last refresh time for auto-reload
    last_refresh: Instant,

    /// Pending 'g' for gg command
    pending_g: bool,
    last_g_press: Option<Instant>,
}

impl App {
    fn new(config: TuiConfig) -> Self {
        let current_view = if config.start_in_plan_mode {
            ViewKind::Plan
        } else {
            ViewKind::Findings
        };

        let findings_state = FindingsViewState::new(config.findings);
        let mut plan_state = PlanViewState::new();

        if !config.perspective_results.is_empty() {
            plan_state.set_perspective_results(config.perspective_results);
        }

        if let Some(plan) = config.unified_plan {
            plan_state.set_unified_plan(plan);
        }

        Self {
            current_view,
            findings_state,
            plan_state,
            findings_path: config.findings_path,
            status_message: "Press ':' for commands, 'q' to quit".to_string(),
            status_tone: StatusTone::Info,
            command_mode: false,
            command_buffer: String::new(),
            last_refresh: Instant::now(),
            pending_g: false,
            last_g_press: None,
        }
    }

    fn set_status<S: Into<String>>(&mut self, message: S, tone: StatusTone) {
        self.status_message = message.into();
        self.status_tone = tone;
    }

    fn reload_findings(&mut self) {
        if let Some(path) = &self.findings_path {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    match serde_json::from_str::<Vec<Finding>>(&content) {
                        Ok(findings) => {
                            let count = findings.len();
                            self.findings_state = FindingsViewState::new(findings);
                            self.set_status(
                                format!("Reloaded {} findings from {}", count, path.display()),
                                StatusTone::Success,
                            );
                        }
                        Err(e) => {
                            self.set_status(
                                format!("Failed to parse findings: {}", e),
                                StatusTone::Error,
                            );
                        }
                    }
                }
                Err(e) => {
                    self.set_status(format!("Failed to read {}: {}", path.display(), e), StatusTone::Error);
                }
            }
        } else {
            self.set_status("No findings file configured for reload", StatusTone::Warning);
        }
    }

    fn check_auto_refresh(&mut self) {
        if self.command_mode {
            return;
        }
        if self.last_refresh.elapsed() >= AUTO_REFRESH_INTERVAL {
            // Could implement live file watching here
            self.last_refresh = Instant::now();
        }
    }

    fn enter_command_mode(&mut self) {
        self.command_mode = true;
        self.command_buffer.clear();
        self.set_status(":", StatusTone::Info);
    }

    fn execute_command(&mut self, command: &str) {
        match command {
            "findings" | "f" => {
                self.current_view = ViewKind::Findings;
                self.set_status("Switched to findings view", StatusTone::Info);
            }
            "plan" | "p" => {
                self.current_view = ViewKind::Plan;
                self.set_status("Switched to plan view", StatusTone::Info);
            }
            "reload" | "r" => {
                self.reload_findings();
            }
            "q" | "quit" => {
                // Will be handled in main loop
            }
            _ => {
                self.set_status(format!("Unknown command: {}", command), StatusTone::Error);
            }
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> bool {
        if !self.command_mode {
            return false;
        }
        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_buffer.clear();
                self.set_status("Command canceled", StatusTone::Info);
            }
            KeyCode::Enter => {
                let command = self.command_buffer.trim().to_string();
                self.command_mode = false;
                self.command_buffer.clear();
                if command.is_empty() {
                    self.set_status("Empty command", StatusTone::Info);
                } else {
                    self.execute_command(&command);
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                self.command_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            _ => {}
        }
        true
    }

    fn handle_lower_g(&mut self) {
        let now = Instant::now();
        if self.pending_g {
            if let Some(prev) = self.last_g_press {
                if now.duration_since(prev) <= Duration::from_millis(800) {
                    // gg - jump to start
                    match self.current_view {
                        ViewKind::Findings => {
                            self.findings_state.table_state.select(Some(0));
                            self.findings_state.detail_scroll = 0;
                        }
                        ViewKind::Plan => {
                            self.plan_state.perspectives_state.select(Some(0));
                            self.plan_state.tasks_state.select(Some(0));
                        }
                    }
                    self.set_status("Jumped to top (gg)", StatusTone::Info);
                    self.pending_g = false;
                    self.last_g_press = None;
                    return;
                }
            }
        }
        self.pending_g = true;
        self.last_g_press = Some(now);
        self.set_status("Press g again to jump to top", StatusTone::Info);
    }

    fn cancel_pending_g(&mut self) {
        self.pending_g = false;
        self.last_g_press = None;
    }

    fn copy_current_item(&mut self) {
        let json = match self.current_view {
            ViewKind::Findings => {
                self.findings_state.selected().map(|f| {
                    serde_json::to_string_pretty(f).unwrap_or_default()
                })
            }
            ViewKind::Plan => {
                // Copy current task or perspective details
                None
            }
        };

        if let Some(content) = json {
            match copy_to_clipboard(&content) {
                Ok(()) => self.set_status("Copied to clipboard", StatusTone::Success),
                Err(e) => self.set_status(format!("Copy failed: {}", e), StatusTone::Error),
            }
        } else {
            self.set_status("Nothing selected to copy", StatusTone::Warning);
        }
    }

    fn state_line(&self) -> String {
        match self.current_view {
            ViewKind::Findings => {
                let total = self.findings_state.findings.len();
                let filtered = self.findings_state.filtered_findings().len();
                format!(
                    "View: {} · Filter: {} · Showing: {}/{}",
                    self.current_view.label(),
                    self.findings_state.filter.label(),
                    filtered,
                    total
                )
            }
            ViewKind::Plan => {
                let perspectives = self.plan_state.perspective_results.len();
                let tasks = self.plan_state.unified_plan.as_ref().map(|p| p.tasks.len()).unwrap_or(0);
                let unanswered = self.plan_state.unanswered_count();
                format!(
                    "View: {} · Perspectives: {} · Tasks: {} · Unanswered: {}",
                    self.current_view.label(),
                    perspectives,
                    tasks,
                    unanswered
                )
            }
        }
    }

    fn help_line(&self) -> &'static str {
        match self.current_view {
            ViewKind::Findings => {
                "Keys: Tab focus · j/k nav · c filter · y copy · r reload · : cmd · q quit"
            }
            ViewKind::Plan => {
                "Keys: Tab pane · j/k nav · 1-9 answer · y copy · : cmd · q quit"
            }
        }
    }
}

/// Run the TUI with the given configuration
pub fn run_tui(config: TuiConfig) -> Result<()> {
    let mut app = App::new(config);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, &mut app);

    cleanup_terminal(terminal)?;
    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        app.check_auto_refresh();

        terminal.draw(|frame| draw_ui(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Handle command mode first
                if app.handle_command_key(key) {
                    // Check if quit command was entered
                    if app.command_buffer.is_empty()
                        && matches!(key.code, KeyCode::Enter)
                        && (app.status_message.starts_with("Unknown command")
                            || app.status_message == "Empty command")
                    {
                        continue;
                    }
                    continue;
                }

                // Cancel pending g for non-g keys
                if !matches!(key.code, KeyCode::Char('g')) {
                    app.cancel_pending_g();
                }

                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char(':') => app.enter_command_mode(),
                    KeyCode::Char('g') => app.handle_lower_g(),
                    KeyCode::Char('y') => app.copy_current_item(),
                    KeyCode::Char('r') if ctrl => app.reload_findings(),

                    // Navigation
                    KeyCode::Tab => {
                        match app.current_view {
                            ViewKind::Findings => app.findings_state.toggle_focus(),
                            ViewKind::Plan => app.plan_state.cycle_pane(),
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match app.current_view {
                            ViewKind::Findings => {
                                if app.findings_state.detail_focused {
                                    app.findings_state.scroll_detail(1);
                                } else {
                                    app.findings_state.move_selection(1);
                                }
                            }
                            ViewKind::Plan => app.plan_state.move_selection(1),
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        match app.current_view {
                            ViewKind::Findings => {
                                if app.findings_state.detail_focused {
                                    app.findings_state.scroll_detail(-1);
                                } else {
                                    app.findings_state.move_selection(-1);
                                }
                            }
                            ViewKind::Plan => app.plan_state.move_selection(-1),
                        }
                    }
                    KeyCode::PageDown => {
                        match app.current_view {
                            ViewKind::Findings => {
                                if app.findings_state.detail_focused {
                                    app.findings_state.scroll_detail(PAGE_JUMP as i16);
                                } else {
                                    app.findings_state.move_selection(PAGE_JUMP as isize);
                                }
                            }
                            ViewKind::Plan => app.plan_state.move_selection(PAGE_JUMP as isize),
                        }
                    }
                    KeyCode::PageUp => {
                        match app.current_view {
                            ViewKind::Findings => {
                                if app.findings_state.detail_focused {
                                    app.findings_state.scroll_detail(-(PAGE_JUMP as i16));
                                } else {
                                    app.findings_state.move_selection(-(PAGE_JUMP as isize));
                                }
                            }
                            ViewKind::Plan => app.plan_state.move_selection(-(PAGE_JUMP as isize)),
                        }
                    }
                    KeyCode::Char('G') => {
                        // Jump to end
                        match app.current_view {
                            ViewKind::Findings => {
                                let len = app.findings_state.filtered_findings().len();
                                if len > 0 {
                                    app.findings_state.table_state.select(Some(len - 1));
                                }
                            }
                            ViewKind::Plan => {
                                // Jump to last task
                                if let Some(plan) = &app.plan_state.unified_plan {
                                    if !plan.tasks.is_empty() {
                                        app.plan_state.tasks_state.select(Some(plan.tasks.len() - 1));
                                    }
                                }
                            }
                        }
                    }

                    // Filtering (findings view)
                    KeyCode::Char('c') | KeyCode::Char('C') if app.current_view == ViewKind::Findings => {
                        app.findings_state.cycle_filter();
                        app.set_status(
                            format!("Filter: {}", app.findings_state.filter.label()),
                            StatusTone::Info,
                        );
                    }

                    // Answer questions (plan view) with number keys
                    KeyCode::Char(c @ '1'..='9') if app.current_view == ViewKind::Plan => {
                        if app.plan_state.active_pane == PlanPane::Questions {
                            let idx = (c as u8 - b'1') as usize;
                            // Clone the option to avoid borrow conflicts
                            let option_to_set = app.plan_state.unified_plan
                                .as_ref()
                                .and_then(|plan| {
                                    app.plan_state.questions_state.selected()
                                        .and_then(|qi| plan.questions.get(qi))
                                        .and_then(|q| q.options.get(idx).cloned())
                                });
                            if let Some(option) = option_to_set {
                                app.plan_state.set_current_answer(option.clone());
                                app.set_status(
                                    format!("Answer set: {}", option),
                                    StatusTone::Success,
                                );
                            }
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn draw_ui(frame: &mut Frame<'_>, app: &mut App) {
    // Background
    frame.render_widget(
        Block::default().style(Style::default().bg(COLOR_BG)),
        frame.size(),
    );

    // Layout: main content + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(10), Constraint::Length(5)].as_ref())
        .split(frame.size());

    // Draw current view
    match app.current_view {
        ViewKind::Findings => {
            draw_findings_view(frame, chunks[0], &mut app.findings_state);
        }
        ViewKind::Plan => {
            draw_plan_view(frame, chunks[0], &mut app.plan_state);
        }
    }

    // Draw status bar
    draw_status_bar(
        frame,
        chunks[1],
        &app.status_message,
        app.status_tone,
        &app.state_line(),
        app.help_line(),
    );

    // Draw command palette if active
    if app.command_mode {
        draw_command_palette(frame, frame.size(), &app.command_buffer);
    }
}

fn cleanup_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
