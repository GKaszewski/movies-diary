use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};

use tui::app::{self, Action, App, BulkImportStage, Command, Screen, SettingsField, Tab};
use tui::client::{ApiClient, ApiError};
use tui::config::Config;

/// Convert an API error into either `AuthExpired` (for 401s) or a `ShowError`.
fn api_err_action(e: ApiError) -> Action {
    match e {
        ApiError::Unauthorized => Action::AuthExpired,
        other => Action::ShowError(other.to_string()),
    }
}

fn main() -> anyhow::Result<()> {
    Config::init_keyring()?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

async fn run() -> anyhow::Result<()> {
    let mut config = Config::load();

    // env var override
    if let Ok(url) = std::env::var("MOVIES_API_URL") {
        match &mut config {
            Some(c) => c.api_url = url,
            None => config = Some(Config { api_url: url }),
        }
    }

    let initial_url = config
        .as_ref()
        .map(|c| c.api_url.as_str())
        .unwrap_or("http://localhost:3000");
    let client = Arc::new(ApiClient::new(initial_url));
    let saved_token = tokio::task::spawn_blocking(Config::load_token)
        .await
        .unwrap_or(None);
    let mut app = App::new(config, saved_token.clone());

    let (tx, mut rx) = mpsc::channel::<Action>(64);
    let mut terminal = ratatui::init();

    // If we start directly in Main (saved token), trigger an initial diary load
    if matches!(app.screen, Screen::Main(_))
        && let Some(token) = &saved_token
    {
        let c = client.clone();
        let t = token.clone();
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let action = match c.get_diary(&t, 0, 20).await {
                Ok(r) => Action::DiaryLoaded {
                    entries: r.items,
                    total: r.total_count,
                },
                Err(e) => api_err_action(e),
            };
            let _ = tx2.send(action).await;
        });
    }

    let result = async {
        loop {
            terminal.draw(|f| tui::ui::render(f, &app))?;

            // Poll keyboard — non-blocking with short timeout
            if event::poll(Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
            {
                if key.kind != ratatui::crossterm::event::KeyEventKind::Press {
                    continue;
                }
                if let Some(action) = key_to_action(&app, key) {
                    if matches!(action, Action::Quit) {
                        break;
                    }
                    let cmds = app::update(&mut app, action);
                    for cmd in cmds {
                        handle_command(cmd, &app, &client, &tx);
                    }
                }
            }

            // Drain async results
            while let Ok(action) = rx.try_recv() {
                let cmds = app::update(&mut app, action);
                for cmd in cmds {
                    handle_command(cmd, &app, &client, &tx);
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;

    ratatui::restore();
    result
}

// ── Command executor ──────────────────────────────────────────────────────────

fn handle_command(cmd: Command, app: &App, client: &Arc<ApiClient>, tx: &mpsc::Sender<Action>) {
    match cmd {
        Command::SaveConfig(url) => {
            let config = Config {
                api_url: url.clone(),
            };
            if let Err(e) = config.save() {
                let tx2 = tx.clone();
                let msg = format!("Failed to save config: {e}");
                tokio::spawn(async move {
                    let _ = tx2.send(Action::ShowError(msg)).await;
                });
            }
            client.update_url(&url);
        }

        Command::SaveToken(token) => {
            let tx2 = tx.clone();
            tokio::spawn(async move {
                if let Err(e) = tokio::task::spawn_blocking(move || Config::save_token(&token))
                    .await
                    .unwrap_or_else(|e| Err(anyhow::anyhow!(e)))
                {
                    let msg = format!("Token not saved to keychain: {e}");
                    let _ = tx2.send(Action::ShowError(msg)).await;
                }
            });
        }

        Command::ClearToken => {
            tokio::spawn(async {
                let _ = tokio::task::spawn_blocking(Config::clear_token).await;
            });
        }

        Command::Login { email, password } => {
            let c = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let action = match c.login(&email, &password).await {
                    Ok(r) => Action::AuthOk(r.token),
                    Err(e) => Action::AuthFail(e.to_string()),
                };
                let _ = tx.send(action).await;
            });
        }

        Command::LoadDiary { offset } => {
            let Some(token) = app.token.clone() else {
                return;
            };
            let c = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let action = match c.get_diary(&token, offset, 20).await {
                    Ok(r) => Action::DiaryLoaded {
                        entries: r.items,
                        total: r.total_count,
                    },
                    Err(e) => api_err_action(e),
                };
                let _ = tx.send(action).await;
            });
        }

        Command::LoadHistory { movie_id } => {
            let Some(token) = app.token.clone() else {
                return;
            };
            let c = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let action = match c.get_movie_history(&token, movie_id).await {
                    Ok(r) => Action::HistoryLoaded(r),
                    Err(ApiError::Unauthorized) => Action::AuthExpired,
                    Err(e) => Action::HistoryLoadFailed(e.to_string()),
                };
                let _ = tx.send(action).await;
            });
        }

        Command::CreateReview(req) => {
            let Some(token) = app.token.clone() else {
                return;
            };
            let c = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let action = match c.create_review(&token, &req).await {
                    Ok(()) => Action::ReviewCreated,
                    Err(ApiError::Unauthorized) => Action::AuthExpired,
                    Err(e) => Action::ReviewCreateFailed(e.to_string()),
                };
                let _ = tx.send(action).await;
            });
        }

        Command::DeleteReview(id) => {
            let Some(token) = app.token.clone() else {
                return;
            };
            let c = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let action = match c.delete_review(&token, id).await {
                    Ok(()) => Action::ReviewDeleted(id),
                    Err(ApiError::Unauthorized) => Action::AuthExpired,
                    Err(e) => Action::ReviewDeleteFailed(e.to_string()),
                };
                let _ = tx.send(action).await;
            });
        }

        Command::ReadFile { path } => {
            let tx = tx.clone();
            tokio::spawn(async move {
                let action =
                    match tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
                        .await
                        .unwrap_or_else(|e| Err(std::io::Error::other(e)))
                    {
                        Ok(content) => Action::FileRead(content),
                        Err(e) => Action::FileReadFailed(e.to_string()),
                    };
                let _ = tx.send(action).await;
            });
        }

        Command::ImportNext(index) => {
            let Some(token) = app.token.clone() else {
                return;
            };
            let req = match &app.screen {
                Screen::Main(m) => match m.bulk_import.valid_requests.get(index) {
                    Some(r) => r.clone(),
                    None => return,
                },
                _ => return,
            };
            let c = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let error = c
                    .create_review(&token, &req)
                    .await
                    .err()
                    .map(|e| e.to_string());
                let _ = tx.send(Action::BulkItemDone { index, error }).await;
            });
        }
    }
}

// ── Key → Action ──────────────────────────────────────────────────────────────

fn tab_shortcut(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char('1') => Some(Action::TabSelect(Tab::Diary)),
        KeyCode::Char('2') => Some(Action::TabSelect(Tab::AddReview)),
        KeyCode::Char('3') => Some(Action::TabSelect(Tab::BulkImport)),
        KeyCode::Char('4') => Some(Action::TabSelect(Tab::Settings)),
        _ => None,
    }
}

fn key_to_action(app: &App, key: ratatui::crossterm::event::KeyEvent) -> Option<Action> {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }

    match &app.screen {
        Screen::Setup(_) => match key.code {
            KeyCode::Char(c) => Some(Action::InputChar(c)),
            KeyCode::Backspace => Some(Action::Backspace),
            KeyCode::Enter => Some(Action::SetupSubmit),
            KeyCode::Esc => Some(Action::Escape),
            _ => None,
        },

        Screen::Login(_) => match key.code {
            KeyCode::Char(c) => Some(Action::InputChar(c)),
            KeyCode::Backspace => Some(Action::Backspace),
            KeyCode::Tab => Some(Action::FocusNext),
            KeyCode::BackTab => Some(Action::FocusPrev),
            KeyCode::Enter => Some(Action::LoginSubmit),
            _ => None,
        },

        Screen::Main(m) => match m.tab {
            Tab::Diary => match key.code {
                KeyCode::Up | KeyCode::Char('k') => Some(Action::ScrollUp),
                KeyCode::Down | KeyCode::Char('j') => Some(Action::ScrollDown),
                KeyCode::Enter => Some(Action::OpenHistory),
                KeyCode::Char('d') => Some(Action::DeleteInit),
                KeyCode::Char('y') if m.diary.delete_pending.is_some() => {
                    Some(Action::DeleteConfirm)
                }
                KeyCode::Char('n') if m.diary.delete_pending.is_some() => {
                    Some(Action::DeleteCancel)
                }
                KeyCode::Esc => Some(Action::Escape),
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Tab => Some(Action::TabNext),
                KeyCode::BackTab => Some(Action::TabPrev),
                KeyCode::Char('>') | KeyCode::Char('m') => Some(Action::LoadMore),
                KeyCode::Char('<') | KeyCode::Char('b') => Some(Action::LoadPrev),
                _ => tab_shortcut(key.code),
            },

            Tab::AddReview => match key.code {
                KeyCode::Char(c) => Some(Action::InputChar(c)),
                KeyCode::Backspace => Some(Action::Backspace),
                KeyCode::Tab => Some(Action::FocusNext),
                KeyCode::BackTab => Some(Action::FocusPrev),
                KeyCode::Left => Some(Action::RatingDown),
                KeyCode::Right => Some(Action::RatingUp),
                KeyCode::Enter => Some(Action::ReviewSubmit),
                KeyCode::Esc => Some(Action::Escape),
                _ => None,
            },

            Tab::BulkImport => {
                let in_path = m.bulk_import.stage == BulkImportStage::EnterPath;
                match key.code {
                    KeyCode::Char(c) if in_path => Some(Action::InputChar(c)),
                    KeyCode::Backspace if in_path => Some(Action::Backspace),
                    KeyCode::Enter => match m.bulk_import.stage {
                        BulkImportStage::EnterPath => Some(Action::BulkParseFile),
                        BulkImportStage::Preview => Some(Action::BulkImportAll),
                        _ => None,
                    },
                    KeyCode::Esc => Some(Action::BulkCancel),
                    KeyCode::Tab if !in_path => Some(Action::TabNext),
                    KeyCode::BackTab if !in_path => Some(Action::TabPrev),
                    KeyCode::Char('q') if !in_path => Some(Action::Quit),
                    _ if !in_path => tab_shortcut(key.code),
                    _ => None,
                }
            }

            Tab::Settings => {
                let on_url = m.settings.focused == SettingsField::ApiUrl;
                match key.code {
                    KeyCode::Char(c) if on_url => Some(Action::InputChar(c)),
                    KeyCode::Backspace if on_url => Some(Action::Backspace),
                    KeyCode::Tab => Some(Action::FocusNext),
                    KeyCode::BackTab => Some(Action::FocusPrev),
                    KeyCode::Enter => match m.settings.focused {
                        SettingsField::Save | SettingsField::ApiUrl => Some(Action::SettingsSave),
                        SettingsField::Logout => Some(Action::SettingsLogout),
                    },
                    KeyCode::Esc => Some(Action::Escape),
                    KeyCode::Char('q') => Some(Action::Quit),
                    _ if !on_url => tab_shortcut(key.code),
                    _ => None,
                }
            }
        },
    }
}
