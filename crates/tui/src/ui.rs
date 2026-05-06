use crate::app::{
    AddReviewField, AddReviewState, App, BulkImportStage, BulkImportState, DiaryState, LoginField,
    LoginState, Screen, SettingsField, SettingsState, SetupState, StatusMsg, Tab,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
};

const APP_TITLE: &str = "Movies diary manager";

pub fn render(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::Setup(s) => draw_setup(frame, frame.area(), s),
        Screen::Login(s) => draw_login(frame, frame.area(), s),
        Screen::Main(m) => {
            let rows = Layout::vertical([
                Constraint::Length(3),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(frame.area());

            draw_tab_bar(frame, rows[0], m.tab);

            match m.tab {
                Tab::Diary => draw_diary(frame, rows[1], &m.diary),
                Tab::AddReview => draw_add_review(frame, rows[1], &m.add_review),
                Tab::BulkImport => draw_bulk_import(frame, rows[1], &m.bulk_import),
                Tab::Settings => draw_settings(frame, rows[1], &m.settings),
            }

            draw_status_bar(frame, rows[2], app.status.as_ref(), app.loading);
        }
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn draw_setup(frame: &mut Frame, area: Rect, state: &SetupState) {
    let popup = centered_rect(60, 14, area);
    let block = Block::default()
        .title(format!(" {APP_TITLE} — Setup "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(block, popup);

    let inner = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .margin(1)
    .split(popup);

    frame.render_widget(
        Paragraph::new("Enter the API server URL to continue.").alignment(Alignment::Center),
        inner[1],
    );

    let url_display = format!("{}_", state.api_url);
    let url_widget = Paragraph::new(url_display).block(
        Block::default()
            .title("API URL")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(url_widget, inner[2]);

    if let Some(err) = &state.error {
        frame.render_widget(
            Paragraph::new(Span::styled(err.as_str(), Style::default().fg(Color::Red))),
            inner[3],
        );
    }

    frame.render_widget(
        Paragraph::new("Enter to save and continue").alignment(Alignment::Center),
        inner[4],
    );
}

// ── Login ─────────────────────────────────────────────────────────────────────

fn draw_login(frame: &mut Frame, area: Rect, state: &LoginState) {
    let popup = centered_rect(60, 16, area);
    let block = Block::default()
        .title(format!(" {APP_TITLE} — Login "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(block, popup);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .margin(1)
    .split(popup);

    let email_style = if state.focused == LoginField::Email {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let pass_style = if state.focused == LoginField::Password {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let email_display = if state.focused == LoginField::Email {
        format!("{}_", state.email)
    } else {
        state.email.clone()
    };
    let pass_display = if state.focused == LoginField::Password {
        format!("{}_", "*".repeat(state.password.len()))
    } else {
        "*".repeat(state.password.len())
    };
    frame.render_widget(
        Paragraph::new(email_display).block(
            Block::default()
                .title("Email")
                .borders(Borders::ALL)
                .border_style(email_style),
        ),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(pass_display).block(
            Block::default()
                .title("Password")
                .borders(Borders::ALL)
                .border_style(pass_style),
        ),
        rows[3],
    );
    frame.render_widget(
        Paragraph::new("Tab: next field   Enter: login").alignment(Alignment::Center),
        rows[4],
    );
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

fn draw_tab_bar(frame: &mut Frame, area: Rect, active: Tab) {
    let tabs = [
        (Tab::Diary, "1: Diary"),
        (Tab::AddReview, "2: Add Review"),
        (Tab::BulkImport, "3: Bulk Import"),
        (Tab::Settings, "4: Settings"),
    ];

    let spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .flat_map(|(i, (tab, label))| {
            let style = if *tab == active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let sep = if i < tabs.len() - 1 { "  │  " } else { "" };
            vec![Span::styled(format!(" {label} "), style), Span::raw(sep)]
        })
        .collect();

    let tab_line = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(tab_line, area);
}

// ── Diary ─────────────────────────────────────────────────────────────────────

fn draw_diary(frame: &mut Frame, area: Rect, state: &DiaryState) {
    let cols =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).split(area);

    // Left: entry list
    let items: Vec<ListItem> = state
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let stars_str = stars(e.review.rating);
            let watched = &e.review.watched_at[..10.min(e.review.watched_at.len())];
            let title = truncate(&e.movie.title, 24);
            let line = format!("{watched}  {title:<24}  {stars_str}");
            let style = if i == state.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let can_load_more = (state.offset as u64 + state.entries.len() as u64) < state.total;
    let list_title = if can_load_more {
        format!(" Diary ({} entries) [m: load more] ", state.total)
    } else {
        format!(" Diary ({} entries) ", state.total)
    };
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    let list = List::new(items).block(Block::default().title(list_title).borders(Borders::ALL));
    frame.render_stateful_widget(list, cols[0], &mut list_state);

    // Delete confirmation overlay
    if state.delete_pending.is_some() {
        let confirm = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Delete this review?",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  y: confirm   n/Esc: cancel"),
        ])
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Center);
        let overlay = centered_rect(40, 8, cols[0]);
        frame.render_widget(ratatui::widgets::Clear, overlay);
        frame.render_widget(confirm, overlay);
    }

    // Right: history panel
    let history_block = Block::default()
        .title(" Movie History ")
        .borders(Borders::ALL);
    match &state.history {
        None => {
            let hint = Paragraph::new(vec![
                Line::from(""),
                Line::from("Select an entry and"),
                Line::from("press Enter to view"),
                Line::from("movie history."),
            ])
            .block(history_block)
            .alignment(Alignment::Center);
            frame.render_widget(hint, cols[1]);
        }
        Some(h) => {
            let mut lines = vec![
                Line::from(Span::styled(
                    format!("{} ({})", h.movie.title, h.movie.release_year),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from("─".repeat(cols[1].width.saturating_sub(2) as usize)),
            ];
            for v in &h.viewings {
                let watched = &v.watched_at[..10.min(v.watched_at.len())];
                lines.push(Line::from(format!("{watched}  {}", stars(v.rating))));
                if let Some(c) = &v.comment {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", truncate(c, 30)),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(format!("Trend: {}", h.trend)));

            frame.render_widget(
                Paragraph::new(lines)
                    .block(history_block)
                    .wrap(Wrap { trim: true }),
                cols[1],
            );
        }
    }
}

// ── Add Review ────────────────────────────────────────────────────────────────

fn draw_add_review(frame: &mut Frame, area: Rect, state: &AddReviewState) {
    let block = Block::default().title(" Add Review ").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // rows[0]=ExternalId  [1]=Title  [2]=Year  [3]=Rating  [4]=WatchedAt  [5]=Comment  [6]=Submit  [7]=hint
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(inner);

    let fs = |f: AddReviewField| {
        if state.focused == f {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        }
    };
    let ft = |s: &str, f: AddReviewField| {
        if state.focused == f {
            format!("{s}_")
        } else {
            s.to_string()
        }
    };

    frame.render_widget(
        Paragraph::new(ft(&state.external_id, AddReviewField::ExternalId)).block(
            Block::default()
                .title("External ID (TMDB/OMDB)")
                .borders(Borders::ALL)
                .border_style(fs(AddReviewField::ExternalId)),
        ),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(ft(&state.title, AddReviewField::Title)).block(
            Block::default()
                .title("Title")
                .borders(Borders::ALL)
                .border_style(fs(AddReviewField::Title)),
        ),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(ft(&state.year, AddReviewField::Year)).block(
            Block::default()
                .title("Year")
                .borders(Borders::ALL)
                .border_style(fs(AddReviewField::Year)),
        ),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{}  \u{2190} \u{2192} to adjust",
            stars(state.rating)
        ))
        .block(
            Block::default()
                .title("Rating (0-5)")
                .borders(Borders::ALL)
                .border_style(fs(AddReviewField::Rating)),
        ),
        rows[3],
    );
    frame.render_widget(
        Paragraph::new(ft(&state.watched_at, AddReviewField::WatchedAt)).block(
            Block::default()
                .title("Watched at (YYYY-MM-DDTHH:MM:SS)")
                .borders(Borders::ALL)
                .border_style(fs(AddReviewField::WatchedAt)),
        ),
        rows[4],
    );
    frame.render_widget(
        Paragraph::new(ft(&state.comment, AddReviewField::Comment)).block(
            Block::default()
                .title("Comment (optional)")
                .borders(Borders::ALL)
                .border_style(fs(AddReviewField::Comment)),
        ),
        rows[5],
    );

    let submit_style = if state.focused == AddReviewField::Submit {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    frame.render_widget(
        Paragraph::new("[ Submit ]")
            .style(submit_style)
            .alignment(Alignment::Center),
        rows[6],
    );
    frame.render_widget(
        Paragraph::new("Tab: next field   \u{2190}\u{2192}: rating   Enter: submit")
            .style(Style::default().fg(Color::DarkGray)),
        rows[7],
    );
}

// ── Bulk Import ───────────────────────────────────────────────────────────────

fn draw_bulk_import(frame: &mut Frame, area: Rect, state: &BulkImportState) {
    let block = Block::default()
        .title(" Bulk Import ")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(inner);

    // File path field (always visible)
    let path_style = if state.stage == BulkImportStage::EnterPath {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let path_display = if state.stage == BulkImportStage::EnterPath {
        format!("{}_", state.file_path)
    } else {
        state.file_path.clone()
    };
    frame.render_widget(
        Paragraph::new(path_display).block(
            Block::default()
                .title("File path (CSV)")
                .borders(Borders::ALL)
                .border_style(path_style),
        ),
        rows[0],
    );

    match &state.stage {
        BulkImportStage::EnterPath => {
            frame.render_widget(
                Paragraph::new("Enter to parse the file.").alignment(Alignment::Center),
                rows[1],
            );
        }

        BulkImportStage::Preview => {
            let valid = state.parsed.iter().filter(|r| r.result.is_ok()).count();
            let errors = state.parsed.iter().filter(|r| r.result.is_err()).count();
            let summary = format!("{valid} reviews ready, {errors} errors");

            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(summary, Style::default().fg(Color::Green))),
                Line::from(""),
            ];
            for row in &state.parsed {
                let (icon, text) = match &row.result {
                    Ok(r) => (
                        "\u{2713}",
                        format!(
                            "Row {}: {} \u{2014} rating {}",
                            row.row,
                            r.manual_title
                                .as_deref()
                                .or(r.external_metadata_id.as_deref())
                                .unwrap_or("?"),
                            r.rating
                        ),
                    ),
                    Err(e) => ("\u{2717}", format!("Row {}: {}", row.row, e)),
                };
                let style = if row.result.is_ok() {
                    Style::default()
                } else {
                    Style::default().fg(Color::Red)
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("{icon} "), style),
                    Span::raw(text),
                ]));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Enter: import all   Esc: back",
                Style::default().fg(Color::DarkGray),
            )));

            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), rows[1]);
        }

        BulkImportStage::Importing { done } => {
            let total = state.valid_requests.len();
            let ratio = if total > 0 {
                (*done as f64 / total as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let gauge_area = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .split(rows[1]);
            frame.render_widget(
                Paragraph::new(format!("Importing... {done} / {total}"))
                    .alignment(Alignment::Center),
                gauge_area[0],
            );
            frame.render_widget(
                Gauge::default()
                    .gauge_style(Style::default().fg(Color::Green))
                    .ratio(ratio),
                gauge_area[1],
            );

            let results: Vec<Line> = state
                .results
                .iter()
                .enumerate()
                .take(*done)
                .map(|(i, r)| {
                    let title = state
                        .valid_requests
                        .get(i)
                        .and_then(|r| {
                            r.manual_title
                                .as_deref()
                                .or(r.external_metadata_id.as_deref())
                        })
                        .unwrap_or("?");
                    match r {
                        None => Line::from(Span::styled(
                            format!("\u{2713} {title}"),
                            Style::default().fg(Color::Green),
                        )),
                        Some(e) => Line::from(Span::styled(
                            format!("\u{2717} {title}: {e}"),
                            Style::default().fg(Color::Red),
                        )),
                    }
                })
                .collect();
            frame.render_widget(
                Paragraph::new(results).wrap(Wrap { trim: true }),
                gauge_area[2],
            );
        }

        BulkImportStage::Done => {
            let failed = state.results.iter().filter(|r| r.is_some()).count();
            let total = state.results.len();
            let summary = format!("Done! {} succeeded, {} failed.", total - failed, failed);
            let style = if failed > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            };

            let mut lines = vec![Line::from(Span::styled(summary, style)), Line::from("")];
            for (i, r) in state.results.iter().enumerate() {
                if let Some(err) = r {
                    let title = state
                        .valid_requests
                        .get(i)
                        .and_then(|r| {
                            r.manual_title
                                .as_deref()
                                .or(r.external_metadata_id.as_deref())
                        })
                        .unwrap_or("?");
                    lines.push(Line::from(Span::styled(
                        format!("\u{2717} {title}: {err}"),
                        Style::default().fg(Color::Red),
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Esc: start over",
                Style::default().fg(Color::DarkGray),
            )));
            frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), rows[1]);
        }
    }
}

// ── Settings ──────────────────────────────────────────────────────────────────

fn draw_settings(frame: &mut Frame, area: Rect, state: &SettingsState) {
    let block = Block::default().title(" Settings ").borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(inner);

    let url_style = if state.focused == SettingsField::ApiUrl {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let url_display = if state.focused == SettingsField::ApiUrl {
        format!("{}_", state.api_url)
    } else {
        state.api_url.clone()
    };
    frame.render_widget(
        Paragraph::new(url_display).block(
            Block::default()
                .title("API URL")
                .borders(Borders::ALL)
                .border_style(url_style),
        ),
        rows[0],
    );

    let save_style = if state.focused == SettingsField::Save {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let logout_style = if state.focused == SettingsField::Logout {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let buttons = Line::from(vec![
        Span::styled("[ Save ]", save_style),
        Span::raw("   "),
        Span::styled("[ Logout ]", logout_style),
    ]);
    frame.render_widget(
        Paragraph::new(buttons).alignment(Alignment::Center),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new("Tab: next   Enter: activate").style(Style::default().fg(Color::DarkGray)),
        rows[2],
    );
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn draw_status_bar(frame: &mut Frame, area: Rect, status: Option<&StatusMsg>, loading: bool) {
    let (text, color) = if loading {
        ("Loading...", Color::Yellow)
    } else {
        match status {
            None => ("q: quit   Tab: next tab", Color::DarkGray),
            Some(s) if s.is_error => (s.text.as_str(), Color::Red),
            Some(s) => (s.text.as_str(), Color::Green),
        }
    };
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(color)), area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn stars(rating: u8) -> String {
    format!(
        "{}{}",
        "\u{2605}".repeat(rating as usize),
        "\u{2606}".repeat(5usize.saturating_sub(rating as usize))
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!(
            "{}\u{2026}",
            &s[..s
                .char_indices()
                .nth(max - 1)
                .map(|(i, _)| i)
                .unwrap_or(s.len())]
        )
    }
}

fn centered_rect(width_pct: u16, height: u16, area: Rect) -> Rect {
    let v_chunks = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height.min(area.height)),
        Constraint::Fill(1),
    ])
    .split(area);

    let h_chunks = Layout::horizontal([
        Constraint::Percentage((100 - width_pct) / 2),
        Constraint::Percentage(width_pct),
        Constraint::Percentage((100 - width_pct) / 2),
    ])
    .split(v_chunks[1]);

    h_chunks[1]
}
