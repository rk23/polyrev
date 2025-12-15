//! Reusable TUI widgets

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

// Color scheme
pub const COLOR_BG: Color = Color::Rgb(9, 8, 12);
pub const COLOR_PANEL: Color = Color::Rgb(9, 8, 12);
pub const COLOR_ACCENT: Color = Color::Rgb(159, 160, 156);
pub const COLOR_FOCUS: Color = Color::Cyan;

/// Status message tone for styling
#[derive(Clone, Copy, Default)]
pub enum StatusTone {
    #[default]
    Info,
    Success,
    Error,
    Warning,
}

impl StatusTone {
    pub fn color(self) -> Color {
        match self {
            StatusTone::Info => Color::Cyan,
            StatusTone::Success => Color::Green,
            StatusTone::Error => Color::Red,
            StatusTone::Warning => Color::Yellow,
        }
    }
}

/// Create a themed block with consistent styling
pub fn themed_block(title: impl Into<String>, border_color: Color) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            title.into(),
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(COLOR_PANEL).fg(Color::White))
}

/// Create a centered rectangle for modal dialogs
#[allow(dead_code)]
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(area);
    let vertical_chunk = popup_layout[1];
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(vertical_chunk)[1]
}

/// Draw a command palette at the bottom of the screen
pub fn draw_command_palette(frame: &mut Frame<'_>, area: Rect, buffer: &str) {
    let height = 3;
    if area.height < height + 2 {
        return;
    }
    let popup = Rect {
        x: area.x + 2,
        y: area.y + area.height - height - 1,
        width: area.width.saturating_sub(4),
        height,
    };
    frame.render_widget(Clear, popup);
    let block = themed_block("Command", COLOR_ACCENT);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let paragraph = Paragraph::new(format!(":{}", buffer))
        .style(Style::default().bg(COLOR_PANEL).fg(Color::White))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Draw a status bar with message and help text
pub fn draw_status_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    message: &str,
    tone: StatusTone,
    state_line: &str,
    help_line: &str,
) {
    let info = Line::styled(
        message,
        Style::default()
            .fg(tone.color())
            .add_modifier(Modifier::BOLD),
    );
    let state = Line::from(state_line.to_string());
    let help = Line::from(help_line.to_string());

    let paragraph = Paragraph::new(vec![info, state, help])
        .style(Style::default().bg(COLOR_PANEL).fg(Color::White))
        .block(themed_block("Status", COLOR_ACCENT))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

/// Cross-platform clipboard copy
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let commands: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else {
        &[
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };

    for (cmd, args) in commands {
        if let Ok(mut child) = Command::new(cmd)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            let status = child.wait()?;
            if status.success() {
                return Ok(());
            }
        }
    }

    anyhow::bail!("no clipboard utility found (tried: pbcopy, clip, xclip, xsel)")
}
