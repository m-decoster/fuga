use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::{self, event};
use futures::future::join_all;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState, Wrap};
use ratatui::{DefaultTerminal, Frame};

use crate::supervisor::{ProcessStatus, Supervisor};

/// The UI component that has focus.
#[derive(Debug, Clone, Copy)]
enum Focus {
    /// The process overview has focus.
    Processes,
    /// The log view has focus.
    Logs,
}

/// The UI representation of a process's state.
#[derive(Clone)]
struct ProcessState {
    /// The process's canonical name.
    name: String,
    /// The command (and arguments) used to launch the process.
    command: String,
    /// The process id (pid).
    id: u32,
    /// The process's current status.
    status: ProcessStatus,
    /// The log output of the process.
    log_lines: Vec<String>,
}

/// The current UI state.
struct AppState<'a> {
    /// The canonical name of the application.
    name: String,
    /// A reference to the supervisor, to manage processes and get process information.
    supervisor: &'a mut Supervisor,
    /// The process states as they were last updated.
    latest_process_states: Vec<ProcessState>,
}

/// The TUI application.
pub struct App<'a> {
    /// Flag indicating that the application should quit.
    should_quit: bool,
    /// The current (visual) state.
    app_state: AppState<'a>,
    /// Indicates which component has focus.
    focus: Focus,
    // For process table.
    /// The index of the process that is highlighted.
    selected_process_index: Option<usize>,
    /// The index of the process for which logs are currently being shown.
    /// This value can differ from `selected_process_index`, if that value is changed after
    /// viewing the logs for another process.
    selected_process_index_for_logs: Option<usize>,
    /// The table state of the process table.
    process_table: TableState,

    // For logs.
    /// The current log scrolling position.
    log_scroll_position: u16,
}

impl<'a> App<'a> {
    /// Initialize an `App`.
    ///
    /// # Arguments
    /// - `supervisor` - A mutable reference to the `Supervisor`.
    pub fn new(supervisor: &'a mut Supervisor) -> Self {
        Self {
            should_quit: false,
            app_state: AppState {
                name: supervisor.name().into(),
                supervisor,
                latest_process_states: Vec::new(),
            },
            selected_process_index: None,
            selected_process_index_for_logs: None,
            process_table: TableState::default(),
            focus: Focus::Processes,
            log_scroll_position: 0,
        }
    }

    /// Collects the latest state from the supervisor.
    async fn gather_state(&mut self) {
        self.app_state.supervisor.restart_stopped_processes().await;

        self.app_state.latest_process_states =
            join_all(self.app_state.supervisor.child_iter().map(async |child| {
                let process_description = child.description();
                let process_name = process_description.name.clone();
                let process_command = format!(
                    "{} {}",
                    process_description.command.clone(),
                    process_description.args.join(" ")
                );
                let process_id = child.id().await.unwrap_or_default();
                let process_status = child.status().await;
                let log_lines = child.logs().await;

                ProcessState {
                    name: process_name,
                    command: process_command,
                    id: process_id,
                    status: process_status,
                    log_lines,
                }
            }))
            .await;
    }

    /// Run the main application loop.
    ///
    /// # Arguments
    /// - `terminal` - A terminal instance.
    ///
    /// This method can fail if the supervisor returns an `Err`.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
        while !self.should_quit {
            self.gather_state().await;
            terminal.draw(|frame| self.render(frame))?;
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                match key.code {
                    KeyCode::Char('q') => {
                        self.should_quit = true;
                    }
                    KeyCode::Char('j') | KeyCode::Down => self.next_row(),
                    KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
                    KeyCode::PageUp => self.scroll_to_top(),
                    KeyCode::PageDown => self.scroll_to_bottom(),
                    KeyCode::Char('l') => self.show_logs(),
                    KeyCode::Char('r') => self.restart_process().await,
                    KeyCode::Char('s') => self.stop_process().await,
                    KeyCode::Char('n') => self.switch_focus(),
                    _ => {}
                }
            }
        }
        self.quit().await;
        Ok(())
    }

    /// Switch focus between logs and processes.
    fn switch_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Logs => Focus::Processes,
            Focus::Processes => Focus::Logs,
        }
    }

    /// Go to the next row in the process table or scroll down in the logs.
    fn next_row(&mut self) {
        match self.focus {
            Focus::Processes => {
                self.selected_process_index = match self.selected_process_index {
                    None => Some(0),
                    Some(i) => Some((i + 1) % self.app_state.supervisor.num_processes()),
                };
                self.process_table.select(self.selected_process_index);
            }
            Focus::Logs => {
                self.scroll_logs_down();
            }
        }
    }

    /// Go to the previous row in the process table or scroll down in the logs.
    fn previous_row(&mut self) {
        match self.focus {
            Focus::Processes => {
                self.selected_process_index = match self.selected_process_index {
                    None => Some(0),
                    Some(i) => Some(if i > 0 {
                        i - 1
                    } else {
                        self.app_state.supervisor.num_processes() - 1
                    }),
                };
                self.process_table.select(self.selected_process_index);
            }
            Focus::Logs => {
                self.scroll_logs_up();
            }
        }
    }

    /// Scroll to the top of the process table or the logs.
    fn scroll_to_top(&mut self) {
        match self.focus {
            Focus::Processes => {
                self.selected_process_index = Some(0);
                self.process_table.select(self.selected_process_index);
            }
            Focus::Logs => {
                if let Some(i) = self.selected_process_index_for_logs {
                    let logs = &self.app_state.latest_process_states[i].log_lines;
                    if !logs.is_empty() {
                        self.log_scroll_position = 0;
                    }
                }
            }
        }
    }

    /// Scroll to the bottom of the process table or the logs.
    fn scroll_to_bottom(&mut self) {
        match self.focus {
            Focus::Processes => {
                self.selected_process_index = Some(self.app_state.supervisor.num_processes() - 1);
                self.process_table.select(self.selected_process_index);
            }
            Focus::Logs => {
                if let Some(i) = self.selected_process_index_for_logs {
                    let logs = &self.app_state.latest_process_states[i].log_lines;
                    if !logs.is_empty() {
                        let max_scroll = logs.len().saturating_sub(1) as u16;
                        self.log_scroll_position = max_scroll;
                    }
                }
            }
        }
    }

    //// Update the log view, showing the logs for the currently selected process.
    fn show_logs(&mut self) {
        self.selected_process_index_for_logs = self.selected_process_index;
        self.log_scroll_position = 0;
    }

    /// Scroll down in the logs.
    fn scroll_logs_down(&mut self) {
        if let Some(i) = self.selected_process_index_for_logs {
            let logs = &self.app_state.latest_process_states[i].log_lines;
            if !logs.is_empty() {
                let max_scroll = logs.len().saturating_sub(1) as u16;
                if self.log_scroll_position < max_scroll {
                    self.log_scroll_position += 1;
                }
            }
        }
    }

    /// Scroll up in the logs.
    fn scroll_logs_up(&mut self) {
        if self.log_scroll_position > 0 {
            self.log_scroll_position -= 1;
        }
    }

    /// Restarts the selected process.
    async fn restart_process(&mut self) {
        if let Some(index) = self.selected_process_index {
            self.app_state.supervisor.restart(index).await;
        }
    }

    /// Stops the selected process.
    async fn stop_process(&mut self) {
        if let Some(index) = self.selected_process_index
            && let Some(child) = self.app_state.supervisor.child_mut(index)
        {
            child.kill().await;
        }
    }

    /// Quits the application and notify the supervisor to stop all processes.
    async fn quit(&mut self) {
        self.app_state.supervisor.stop_all().await
    }

    /// Render the UI.
    ///
    /// # Arguments
    /// - `frame` - The UI frame to render into.
    pub fn render(&mut self, frame: &mut Frame) {
        let block_area = self.render_outer_area(frame, frame.area());
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(block_area);
        self.render_table(frame, layout[0]);
        self.render_logs(frame, layout[1]);
    }

    /// Render the outer area border.
    ///
    /// # Arguments
    /// - `frame` - The UI frame to render into.
    /// - `area` - The available area.
    ///
    /// # Returns
    /// - The inner area in which children can render.
    fn render_outer_area(&self, frame: &mut Frame, area: Rect) -> Rect {
        let block = Block::new()
            .title("Fuga")
            .title(Line::from(&*self.app_state.name).centered())
            .title(Line::from(format!("v{}", env!("CARGO_PKG_VERSION"))).right_aligned())
            .title_bottom("[j/k]: scroll")
            .title_bottom("[PgUp/PgDn]: to top/bottom")
            .title_bottom("[r]: (re)start process")
            .title_bottom("[s]: stop process")
            .title_bottom("[l]: show process logs")
            .title_bottom("[n]: switch focus")
            .title_bottom("[q]: quit");
        let inner_rect = block.inner(area);
        frame.render_widget(block, area);
        inner_rect
    }

    /// Render the process table.
    ///
    /// # Arguments
    /// - `frame` - The UI frame to render into.
    /// - `area` - The available area.
    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let mut block = Block::bordered().title("Processes");
        if let Focus::Processes = self.focus {
            block = block.border_style(Style::new().fg(Color::Blue).bold());
        }
        let inner_rect = block.inner(area);

        let header = ["Name", "Command", "PID", "Status"]
            .into_iter()
            .map(|string| Text::from(string).bold())
            .map(Cell::from)
            .collect::<Row>();

        let rows = self
            .app_state
            .latest_process_states
            .clone()
            .into_iter()
            .map(|process_state| {
                let cell_name = Cell::from(Text::from(process_state.name.clone()));
                let cell_cmd = Cell::from(Text::from(process_state.command.clone()));
                let cell_pid = Cell::from(Text::from(process_state.id.to_string()));
                let mut cell_status = Cell::from(Text::from(format!("{:?}", process_state.status)));
                cell_status = match process_state.status {
                    ProcessStatus::Idle => cell_status.style(Style::default().fg(Color::Yellow)),
                    ProcessStatus::Running => cell_status.style(Style::default().fg(Color::Blue)),
                    ProcessStatus::Stopped => cell_status.style(Style::default().fg(Color::Reset)),
                    ProcessStatus::Success => cell_status.style(Style::default().fg(Color::Green)),
                    ProcessStatus::Error => cell_status.style(Style::default().fg(Color::Red)),
                };

                Row::new([cell_name, cell_cmd, cell_pid, cell_status])
            });

        let selected_row_style = Style::default().add_modifier(Modifier::REVERSED);

        let table = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Min(10 + 1),
                Constraint::Min(10 + 1),
                Constraint::Length(10 + 1),
                Constraint::Min(10),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style);

        frame.render_widget(block, area);
        frame.render_stateful_widget(table, inner_rect, &mut self.process_table);
    }

    /// Render the logs.
    ///
    /// # Arguments
    /// - `frame` - The UI frame to render into.
    /// - `area` - The available area.
    fn render_logs(&self, frame: &mut Frame, area: Rect) {
        let process_name = self
            .app_state
            .supervisor
            .child(self.selected_process_index_for_logs.unwrap_or_default())
            .map(|child| child.description().name.clone())
            .unwrap_or_default();

        let mut block = Block::bordered().title("Logs").title_bottom(
            match self.selected_process_index_for_logs {
                None => "Select a process to view its logs",
                Some(_) => &process_name,
            },
        );
        if let Focus::Logs = self.focus {
            block = block.border_style(Style::new().fg(Color::Blue).bold());
        }
        let inner_rect = block.inner(area);

        let scrollbar_width = 1;
        let text_area = Rect {
            x: inner_rect.x,
            y: inner_rect.y,
            width: inner_rect.width - scrollbar_width,
            height: inner_rect.height,
        };

        if let Some(i) = self.selected_process_index_for_logs {
            let logs = self.app_state.latest_process_states[i].clone().log_lines;

            // Wrap all log lines using textwrap
            let mut wrapped_lines: Vec<Line> = Vec::new();
            for line in logs.iter() {
                let wrapped = textwrap::wrap(line, inner_rect.width as usize);
                for w in wrapped {
                    let styled_line = if line.starts_with("[ERR]") {
                        Line::from(Span::styled(w.to_string(), Style::new().fg(Color::Red)))
                    } else {
                        Line::from(w.to_string())
                    };
                    wrapped_lines.push(styled_line);
                }
            }

            let total_rows = wrapped_lines.len() as u16;
            let visible_height = inner_rect.height;
            let max_scroll = total_rows.saturating_sub(visible_height);
            let scroll_pos = self.log_scroll_position.min(max_scroll);

            // Visible portion of wrapped lines
            let visible_lines: Vec<Line> = wrapped_lines
                .iter()
                .skip(scroll_pos as usize)
                .take(visible_height as usize)
                .cloned()
                .collect();

            // Render logs
            let paragraph_logs = Paragraph::new(visible_lines)
                .wrap(Wrap { trim: false })
                .scroll((0, 0));
            frame.render_widget(paragraph_logs, text_area);

            // Render scrollbar
            let scrollbar_height = visible_height.min(total_rows);
            let mut scrollbar: Vec<Line> = Vec::new();
            for i in 0..scrollbar_height {
                let pos_ratio = i as f32 / scrollbar_height as f32;
                let scroll_ratio = scroll_pos as f32 / total_rows as f32;
                if (pos_ratio - scroll_ratio).abs() < 1.0 / scrollbar_height as f32 {
                    scrollbar.push(Line::from(Span::styled("█", Style::new().fg(Color::Blue))));
                } else {
                    scrollbar.push(Line::from(" "));
                }
            }

            let scrollbar_area = Rect {
                x: inner_rect.x + inner_rect.width - scrollbar_width,
                y: inner_rect.y,
                width: scrollbar_width,
                height: inner_rect.height,
            };
            let paragraph_scrollbar = Paragraph::new(scrollbar)
                .style(Style::default().bg(Color::Reset))
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph_scrollbar, scrollbar_area);
        }

        frame.render_widget(block, area);
    }
}
