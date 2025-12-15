//! Plan view - displays planning perspectives and unified plan for approval

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap};
use ratatui::Frame;

use crate::planner::{
    PerspectiveResult, PerspectiveStatus, Risk, Severity, UnifiedPlan, UnifiedQuestion,
    UnifiedTask,
};
use crate::tui::widgets::{themed_block, COLOR_ACCENT, COLOR_FOCUS, COLOR_PANEL};
use crate::tui::{ellipsize, format_duration, wrap_text};

/// Which pane is focused in the plan view
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum PlanPane {
    #[default]
    Perspectives,
    Tasks,
    Questions,
    Detail,
}

impl PlanPane {
    pub fn next(self) -> Self {
        match self {
            PlanPane::Perspectives => PlanPane::Tasks,
            PlanPane::Tasks => PlanPane::Questions,
            PlanPane::Questions => PlanPane::Detail,
            PlanPane::Detail => PlanPane::Perspectives,
        }
    }
}

/// State for the plan view
#[derive(Default)]
pub struct PlanViewState {
    /// Results from each perspective
    pub perspective_results: Vec<PerspectiveResult>,

    /// The unified plan (after reduction)
    pub unified_plan: Option<UnifiedPlan>,

    /// Active pane
    pub active_pane: PlanPane,

    /// Selection states
    pub perspectives_state: ListState,
    pub tasks_state: TableState,
    pub questions_state: ListState,

    /// Detail scroll
    pub detail_scroll: u16,

    /// Currently selected item for detail display
    pub detail_item: DetailItem,

    /// Answers to questions (index -> answer)
    pub answers: Vec<Option<String>>,
}

/// What's being shown in the detail panel
#[derive(Clone, Default)]
#[allow(dead_code)]
pub enum DetailItem {
    #[default]
    None,
    Perspective(usize),
    Task(usize),
    Question(usize),
    Risk(usize),
}

impl PlanViewState {
    pub fn new() -> Self {
        let mut state = Self::default();
        state.perspectives_state.select(Some(0));
        state
    }

    pub fn set_perspective_results(&mut self, results: Vec<PerspectiveResult>) {
        self.perspective_results = results;
        if !self.perspective_results.is_empty() {
            self.perspectives_state.select(Some(0));
        }
    }

    pub fn set_unified_plan(&mut self, plan: UnifiedPlan) {
        let question_count = plan.questions.len();
        self.unified_plan = Some(plan);
        self.answers = vec![None; question_count];
        if self.unified_plan.as_ref().map(|p| !p.tasks.is_empty()).unwrap_or(false) {
            self.tasks_state.select(Some(0));
        }
        if question_count > 0 {
            self.questions_state.select(Some(0));
        }
    }

    pub fn cycle_pane(&mut self) {
        self.active_pane = self.active_pane.next();
        self.update_detail_from_selection();
    }

    pub fn move_selection(&mut self, delta: isize) {
        match self.active_pane {
            PlanPane::Perspectives => {
                let len = self.perspective_results.len();
                if len > 0 {
                    let current = self.perspectives_state.selected().unwrap_or(0) as isize;
                    let next = (current + delta).clamp(0, len as isize - 1) as usize;
                    self.perspectives_state.select(Some(next));
                }
            }
            PlanPane::Tasks => {
                if let Some(plan) = &self.unified_plan {
                    let len = plan.tasks.len();
                    if len > 0 {
                        let current = self.tasks_state.selected().unwrap_or(0) as isize;
                        let next = (current + delta).clamp(0, len as isize - 1) as usize;
                        self.tasks_state.select(Some(next));
                    }
                }
            }
            PlanPane::Questions => {
                if let Some(plan) = &self.unified_plan {
                    let len = plan.questions.len();
                    if len > 0 {
                        let current = self.questions_state.selected().unwrap_or(0) as isize;
                        let next = (current + delta).clamp(0, len as isize - 1) as usize;
                        self.questions_state.select(Some(next));
                    }
                }
            }
            PlanPane::Detail => {
                self.detail_scroll = (self.detail_scroll as isize + delta).max(0) as u16;
            }
        }
        self.update_detail_from_selection();
    }

    fn update_detail_from_selection(&mut self) {
        self.detail_item = match self.active_pane {
            PlanPane::Perspectives => {
                self.perspectives_state.selected().map(DetailItem::Perspective).unwrap_or(DetailItem::None)
            }
            PlanPane::Tasks => {
                self.tasks_state.selected().map(DetailItem::Task).unwrap_or(DetailItem::None)
            }
            PlanPane::Questions => {
                self.questions_state.selected().map(DetailItem::Question).unwrap_or(DetailItem::None)
            }
            PlanPane::Detail => self.detail_item.clone(),
        };
        // Reset scroll when changing items
        if !matches!(self.active_pane, PlanPane::Detail) {
            self.detail_scroll = 0;
        }
    }

    /// Set answer for current question
    pub fn set_current_answer(&mut self, answer: String) {
        if let Some(idx) = self.questions_state.selected() {
            if idx < self.answers.len() {
                self.answers[idx] = Some(answer);
            }
        }
    }

    /// Check if all questions are answered
    #[allow(dead_code)]
    pub fn all_questions_answered(&self) -> bool {
        self.answers.iter().all(|a| a.is_some())
    }

    /// Get count of unanswered questions
    pub fn unanswered_count(&self) -> usize {
        self.answers.iter().filter(|a| a.is_none()).count()
    }
}

/// Draw the perspectives panel (Phase 1 results)
pub fn draw_perspectives_panel(frame: &mut Frame<'_>, area: Rect, state: &mut PlanViewState) {
    let items: Vec<ListItem> = state
        .perspective_results
        .iter()
        .map(|result| {
            let (status_icon, status_style) = match &result.status {
                PerspectiveStatus::Completed => ("✓", Style::default().fg(Color::Green)),
                PerspectiveStatus::Failed { .. } => ("✗", Style::default().fg(Color::Red)),
                PerspectiveStatus::Skipped { .. } => ("○", Style::default().fg(Color::DarkGray)),
            };

            let task_count = result.fragment.as_ref().map(|f| f.tasks.len()).unwrap_or(0);
            let concern_count = result.fragment.as_ref().map(|f| f.concerns.len()).unwrap_or(0);

            let line = Line::from(vec![
                Span::styled(format!("{} ", status_icon), status_style),
                Span::raw(format!(
                    "{} ({} tasks, {} concerns)",
                    ellipsize(&result.perspective_name, 15),
                    task_count,
                    concern_count
                )),
            ]);
            ListItem::new(line)
        })
        .collect();

    let border = if state.active_pane == PlanPane::Perspectives {
        COLOR_FOCUS
    } else {
        COLOR_ACCENT
    };

    let list = List::new(items)
        .block(themed_block("Perspectives (Phase 1)", border))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(56, 80, 109))
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state.perspectives_state);
}

/// Draw the tasks panel (from unified plan)
pub fn draw_tasks_panel(frame: &mut Frame<'_>, area: Rect, state: &mut PlanViewState) {
    let empty_tasks: Vec<UnifiedTask> = Vec::new();
    let tasks = state
        .unified_plan
        .as_ref()
        .map(|p| &p.tasks)
        .unwrap_or(&empty_tasks);

    let headers = Row::new(vec!["ID", "Title", "Deps", "Perspectives"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows = tasks.iter().map(|task| {
        Row::new(vec![
            task.id.clone(),
            ellipsize(&task.title, 30),
            task.depends_on.len().to_string(),
            ellipsize(&task.perspectives.join(", "), 20),
        ])
    });

    let border = if state.active_pane == PlanPane::Tasks {
        COLOR_FOCUS
    } else {
        COLOR_ACCENT
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Percentage(45),
            Constraint::Length(5),
            Constraint::Percentage(30),
        ],
    )
    .header(headers)
    .block(themed_block(format!("Tasks ({})", tasks.len()), border))
    .column_spacing(1)
    .style(Style::default().bg(COLOR_PANEL).fg(Color::White))
    .highlight_style(
        Style::default()
            .bg(Color::Rgb(56, 80, 109))
            .add_modifier(Modifier::BOLD),
    );

    frame.render_stateful_widget(table, area, &mut state.tasks_state);
}

/// Draw the questions panel
pub fn draw_questions_panel(frame: &mut Frame<'_>, area: Rect, state: &mut PlanViewState) {
    let empty_questions: Vec<UnifiedQuestion> = Vec::new();
    let questions = state
        .unified_plan
        .as_ref()
        .map(|p| &p.questions)
        .unwrap_or(&empty_questions);

    let items: Vec<ListItem> = questions
        .iter()
        .enumerate()
        .map(|(idx, q)| {
            let answered = state.answers.get(idx).and_then(|a| a.as_ref());
            let (icon, style) = if answered.is_some() {
                ("✓", Style::default().fg(Color::Green))
            } else {
                ("?", Style::default().fg(Color::Yellow))
            };

            let line = Line::from(vec![
                Span::styled(format!("{} ", icon), style),
                Span::raw(ellipsize(&q.question, 50)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let unanswered = state.unanswered_count();
    let border = if state.active_pane == PlanPane::Questions {
        COLOR_FOCUS
    } else {
        COLOR_ACCENT
    };

    let title = if unanswered > 0 {
        format!("Questions ({} unanswered)", unanswered)
    } else {
        "Questions (all answered)".to_string()
    };

    let list = List::new(items)
        .block(themed_block(title, border))
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(56, 80, 109))
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut state.questions_state);
}

/// Draw the risks panel
pub fn draw_risks_panel(frame: &mut Frame<'_>, area: Rect, state: &PlanViewState) {
    let empty_risks: Vec<Risk> = Vec::new();
    let risks = state
        .unified_plan
        .as_ref()
        .map(|p| &p.risks)
        .unwrap_or(&empty_risks);

    let mut lines: Vec<Line> = Vec::new();

    if risks.is_empty() {
        lines.push(Line::from("No risks identified"));
    } else {
        for risk in risks {
            let severity_style = match risk.severity {
                Severity::High => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                Severity::Medium => Style::default().fg(Color::Yellow),
                Severity::Low => Style::default().fg(Color::Blue),
            };

            lines.push(Line::from(vec![
                Span::styled(format!("⚠ [{}] ", risk.severity), severity_style),
                Span::raw(ellipsize(&risk.description, 50)),
            ]));

            if !risk.raised_by.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  Raised by: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(risk.raised_by.join(", ")),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .style(Style::default().bg(COLOR_PANEL).fg(Color::White))
        .block(themed_block(format!("Risks ({})", risks.len()), COLOR_ACCENT))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Draw the detail panel
pub fn draw_plan_detail(frame: &mut Frame<'_>, area: Rect, state: &PlanViewState) {
    let mut lines: Vec<Line<'static>> = Vec::new();

    match &state.detail_item {
        DetailItem::None => {
            lines.push(Line::from("Select an item to view details"));
        }
        DetailItem::Perspective(idx) => {
            if let Some(result) = state.perspective_results.get(*idx) {
                lines.push(Line::from(vec![Span::styled(
                    result.perspective_name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(format!(
                    "ID: {} · Duration: {}",
                    result.perspective_id,
                    format_duration(result.duration)
                )));
                lines.push(Line::from(format!("Status: {}", result.status)));
                lines.push(Line::default());

                if let Some(fragment) = &result.fragment {
                    lines.push(Line::from(vec![Span::styled(
                        "Summary:".to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    )]));
                    for line in wrap_text(&fragment.summary, 55) {
                        lines.push(Line::from(format!("  {}", line)));
                    }

                    if !fragment.tasks.is_empty() {
                        lines.push(Line::default());
                        lines.push(Line::from(vec![Span::styled(
                            format!("Tasks ({}):", fragment.tasks.len()),
                            Style::default().add_modifier(Modifier::BOLD),
                        )]));
                        for task in &fragment.tasks {
                            lines.push(Line::from(format!("  • {}", task.title)));
                        }
                    }

                    if !fragment.concerns.is_empty() {
                        lines.push(Line::default());
                        lines.push(Line::from(vec![Span::styled(
                            format!("Concerns ({}):", fragment.concerns.len()),
                            Style::default().add_modifier(Modifier::BOLD),
                        )]));
                        for concern in &fragment.concerns {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  [{}] ", concern.severity),
                                    severity_style(concern.severity),
                                ),
                                Span::raw(ellipsize(&concern.description, 45)),
                            ]));
                        }
                    }
                }
            }
        }
        DetailItem::Task(idx) => {
            if let Some(plan) = &state.unified_plan {
                if let Some(task) = plan.tasks.get(*idx) {
                    lines.extend(render_task_detail(task));
                }
            }
        }
        DetailItem::Question(idx) => {
            if let Some(plan) = &state.unified_plan {
                if let Some(question) = plan.questions.get(*idx) {
                    let answer = state.answers.get(*idx).and_then(|a| a.clone());
                    lines.extend(render_question_detail(question, answer.as_ref()));
                }
            }
        }
        DetailItem::Risk(idx) => {
            if let Some(plan) = &state.unified_plan {
                if let Some(risk) = plan.risks.get(*idx) {
                    lines.extend(render_risk_detail(risk));
                }
            }
        }
    }

    let border = if state.active_pane == PlanPane::Detail {
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

fn render_task_detail(task: &UnifiedTask) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        task.title.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(format!("ID: {}", task.id)));

    if !task.perspectives.is_empty() {
        lines.push(Line::from(format!(
            "Perspectives: {}",
            task.perspectives.join(", ")
        )));
    }

    if !task.depends_on.is_empty() {
        lines.push(Line::from(format!(
            "Dependencies: {}",
            task.depends_on.join(", ")
        )));
    }

    if let Some(workflow) = &task.workflow {
        lines.push(Line::from(format!("Workflow: {}", workflow.clone())));
    }

    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "Description:".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    for line in wrap_text(&task.description, 55) {
        lines.push(Line::from(format!("  {}", line)));
    }

    if !task.files.target.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(
            "Target files:".to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for file in &task.files.target {
            lines.push(Line::from(format!("  • {}", file.display())));
        }
    }

    if !task.acceptance_criteria.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(
            "Acceptance Criteria:".to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for (i, ac) in task.acceptance_criteria.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::styled(format!("  {}. ", i + 1), Style::default().fg(Color::Cyan)),
                Span::raw(ac.criterion.clone()),
            ]));
            if !ac.verification.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("     Verify: ".to_string(), Style::default().fg(Color::DarkGray)),
                    Span::raw(ac.verification.clone()),
                ]));
            }
        }
    }
    lines
}

fn render_question_detail(question: &UnifiedQuestion, answer: Option<&String>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "Question:".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    for line in wrap_text(&question.question, 55) {
        lines.push(Line::from(format!("  {}", line)));
    }

    if !question.context.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(
            "Context:".to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for line in wrap_text(&question.context, 55) {
            lines.push(Line::from(format!("  {}", line)));
        }
    }

    if !question.raised_by.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Raised by: ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::raw(question.raised_by.join(", ")),
        ]));
    }

    if !question.blocks.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Blocks: ".to_string(), Style::default().fg(Color::Yellow)),
            Span::raw(question.blocks.join(", ")),
        ]));
    }

    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "Options:".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    for (i, opt) in question.options.iter().enumerate() {
        let is_selected = answer.map(|a| a == opt).unwrap_or(false);
        let marker = if is_selected { "●" } else { "○" };
        let style = if is_selected {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", marker), style),
            Span::styled(format!("[{}] {}", i + 1, opt.clone()), style),
        ]));
    }

    if let Some(ans) = answer {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("Your answer: ".to_string(), Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(ans.clone(), Style::default().fg(Color::Green)),
        ]));
    }
    lines
}

fn render_risk_detail(risk: &Risk) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("⚠ Risk: ".to_string(), severity_style(risk.severity)),
        Span::styled(risk.severity.to_string(), severity_style(risk.severity)),
    ]));

    lines.push(Line::default());
    lines.push(Line::from(vec![Span::styled(
        "Description:".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    for line in wrap_text(&risk.description, 55) {
        lines.push(Line::from(format!("  {}", line)));
    }

    if !risk.raised_by.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("Raised by: ".to_string(), Style::default().fg(Color::DarkGray)),
            Span::raw(risk.raised_by.join(", ")),
        ]));
    }

    if let Some(mitigation) = &risk.mitigation {
        lines.push(Line::default());
        lines.push(Line::from(vec![Span::styled(
            "Mitigation:".to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for line in wrap_text(mitigation, 55) {
            lines.push(Line::from(format!("  {}", line)));
        }
    }
    lines
}

fn severity_style(severity: Severity) -> Style {
    match severity {
        Severity::High => Style::default().fg(Color::Red),
        Severity::Medium => Style::default().fg(Color::Yellow),
        Severity::Low => Style::default().fg(Color::Blue),
    }
}

/// Draw the full plan view
pub fn draw_plan_view(frame: &mut Frame<'_>, area: Rect, state: &mut PlanViewState) {
    // Main layout: top section for Phase 1 + Phase 2, bottom for detail
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(area);

    // Top section: perspectives on left, tasks + questions on right
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)].as_ref())
        .split(main_chunks[0]);

    // Left: perspectives + risks
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(top_chunks[0]);

    draw_perspectives_panel(frame, left_chunks[0], state);
    draw_risks_panel(frame, left_chunks[1], state);

    // Right: tasks + questions
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
        .split(top_chunks[1]);

    draw_tasks_panel(frame, right_chunks[0], state);
    draw_questions_panel(frame, right_chunks[1], state);

    // Bottom: detail panel
    draw_plan_detail(frame, main_chunks[1], state);
}
