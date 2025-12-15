//! Findings view - displays and manages review findings

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

use crate::config::Priority;
use crate::parser::Finding;
use crate::tui::widgets::{themed_block, COLOR_ACCENT, COLOR_FOCUS, COLOR_PANEL};
use crate::tui::{ellipsize, sanitize_text, wrap_text};

/// Filter mode for findings
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum FindingFilter {
    #[default]
    All,
    P0,
    P1,
    P2,
}

impl FindingFilter {
    pub fn next(self) -> Self {
        match self {
            FindingFilter::All => FindingFilter::P0,
            FindingFilter::P0 => FindingFilter::P1,
            FindingFilter::P1 => FindingFilter::P2,
            FindingFilter::P2 => FindingFilter::All,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FindingFilter::All => "all",
            FindingFilter::P0 => "p0",
            FindingFilter::P1 => "p1",
            FindingFilter::P2 => "p2",
        }
    }

    pub fn matches(&self, priority: Priority) -> bool {
        match self {
            FindingFilter::All => true,
            FindingFilter::P0 => priority == Priority::P0,
            FindingFilter::P1 => priority == Priority::P1,
            FindingFilter::P2 => priority == Priority::P2,
        }
    }
}

/// State for the findings view
#[derive(Default)]
pub struct FindingsViewState {
    pub findings: Vec<Finding>,
    pub table_state: TableState,
    pub filter: FindingFilter,
    pub detail_scroll: u16,
    pub detail_focused: bool,
}

impl FindingsViewState {
    pub fn new(findings: Vec<Finding>) -> Self {
        let mut state = Self {
            findings,
            table_state: TableState::default(),
            filter: FindingFilter::All,
            detail_scroll: 0,
            detail_focused: false,
        };
        if !state.findings.is_empty() {
            state.table_state.select(Some(0));
        }
        state
    }

    pub fn filtered_findings(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| self.filter.matches(f.priority))
            .collect()
    }

    pub fn selected(&self) -> Option<&Finding> {
        let filtered = self.filtered_findings();
        self.table_state.selected().and_then(|i| filtered.get(i).copied())
    }

    pub fn move_selection(&mut self, delta: isize) {
        let filtered = self.filtered_findings();
        if filtered.is_empty() {
            self.table_state.select(None);
            return;
        }
        let len = filtered.len() as isize;
        let current = self.table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, len - 1) as usize;
        self.table_state.select(Some(next));
        self.detail_scroll = 0;
    }

    pub fn cycle_filter(&mut self) {
        self.filter = self.filter.next();
        // Reset selection when filter changes
        let filtered = self.filtered_findings();
        if filtered.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
        self.detail_scroll = 0;
    }

    pub fn scroll_detail(&mut self, delta: i16) {
        let next = (self.detail_scroll as i16 + delta).max(0) as u16;
        self.detail_scroll = next;
    }

    pub fn toggle_focus(&mut self) {
        self.detail_focused = !self.detail_focused;
    }
}

/// Draw the findings list table
pub fn draw_findings_table(frame: &mut Frame<'_>, area: Rect, state: &mut FindingsViewState) {
    let filtered = state.filtered_findings();

    let headers = Row::new(vec!["Pri", "ID", "Title", "File"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows = filtered.iter().map(|finding| {
        let pri_style = priority_style(finding.priority);
        Row::new(vec![
            finding.priority.to_string(),
            finding.id.clone(),
            ellipsize(&sanitize_text(&finding.title), 40),
            ellipsize(&finding.file.display().to_string(), 30),
        ])
        .style(pri_style)
    });

    let border = if !state.detail_focused {
        COLOR_FOCUS
    } else {
        COLOR_ACCENT
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(12),
            Constraint::Percentage(50),
            Constraint::Percentage(30),
        ],
    )
    .header(headers)
    .block(themed_block(
        format!(
            "Findings ({}) · filter={}",
            filtered.len(),
            state.filter.label()
        ),
        border,
    ))
    .column_spacing(1)
    .style(Style::default().bg(COLOR_PANEL).fg(Color::White))
    .highlight_style(
        Style::default()
            .bg(Color::Rgb(56, 80, 109))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(table, area, &mut state.table_state);
}

/// Draw the finding detail panel
pub fn draw_finding_detail(frame: &mut Frame<'_>, area: Rect, state: &FindingsViewState) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(finding) = state.selected() {
        // Title and ID
        lines.push(Line::from(vec![Span::styled(
            sanitize_text(&finding.title),
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(vec![
            Span::raw("ID: "),
            Span::styled(&finding.id, Style::default().fg(Color::Cyan)),
            Span::raw(" · Type: "),
            Span::raw(&finding.finding_type),
        ]));

        // Priority and location
        lines.push(Line::from(vec![
            Span::raw("Priority: "),
            Span::styled(
                finding.priority.to_string(),
                Style::default()
                    .fg(priority_color(finding.priority))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " · File: {}:{}",
                finding.file.display(),
                finding.line
            )),
        ]));

        if let Some(model) = &finding.model {
            lines.push(Line::from(vec![
                Span::raw("Model: "),
                Span::styled(model, Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::default());

        // Description
        lines.push(Line::from(vec![Span::styled(
            "Description:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for line in wrap_text(&finding.description, 60) {
            lines.push(Line::from(format!("  {}", line)));
        }

        // Snippet
        if let Some(snippet) = &finding.snippet {
            lines.push(Line::default());
            lines.push(Line::from(vec![Span::styled(
                "Snippet:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for line in snippet.lines().take(10) {
                lines.push(Line::from(vec![Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::Yellow),
                )]));
            }
        }

        // Remediation
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(
            "Remediation:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for line in wrap_text(&finding.remediation, 60) {
            lines.push(Line::from(format!("  {}", line)));
        }

        // Acceptance criteria
        if !finding.acceptance_criteria.is_empty() {
            lines.push(Line::default());
            lines.push(Line::from(vec![Span::styled(
                "Acceptance Criteria:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for (i, criterion) in finding.acceptance_criteria.iter().enumerate() {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}. ", i + 1), Style::default().fg(Color::Cyan)),
                    Span::raw(sanitize_text(criterion)),
                ]));
            }
        }

        // References
        if !finding.references.is_empty() {
            lines.push(Line::default());
            lines.push(Line::from(vec![Span::styled(
                "References:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for reference in &finding.references {
                lines.push(Line::from(vec![
                    Span::styled("  → ", Style::default().fg(Color::Blue)),
                    Span::raw(sanitize_text(reference)),
                ]));
            }
        }
    } else {
        lines.push(Line::from("No finding selected"));
    }

    let border = if state.detail_focused {
        COLOR_FOCUS
    } else {
        COLOR_ACCENT
    };

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(COLOR_PANEL).fg(Color::White))
        .block(themed_block("Detail", border))
        .scroll((state.detail_scroll, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Draw the full findings view (table + detail)
pub fn draw_findings_view(frame: &mut Frame<'_>, area: Rect, state: &mut FindingsViewState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)].as_ref())
        .split(area);

    draw_findings_table(frame, chunks[0], state);
    draw_finding_detail(frame, chunks[1], state);
}

fn priority_color(priority: Priority) -> Color {
    match priority {
        Priority::P0 => Color::Red,
        Priority::P1 => Color::Yellow,
        Priority::P2 => Color::Blue,
    }
}

fn priority_style(priority: Priority) -> Style {
    Style::default().fg(priority_color(priority))
}
