use crate::config::Config;
use api_types::{DiaryEntryDto, LogReviewRequest, ReviewHistoryResponse};
use uuid::Uuid;

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum Screen {
    Setup(SetupState),
    Login(LoginState),
    Main(MainState),
}

#[derive(Debug, Default)]
pub struct SetupState {
    pub api_url: String,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct LoginState {
    pub email: String,
    pub password: String,
    pub focused: LoginField,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    #[default]
    Email,
    Password,
}

// ── Main (4 tabs) ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MainState {
    pub tab: Tab,
    pub diary: DiaryState,
    pub add_review: AddReviewState,
    pub bulk_import: BulkImportState,
    pub settings: SettingsState,
}

impl MainState {
    pub fn new(api_url: String) -> Self {
        Self {
            tab: Tab::Diary,
            diary: DiaryState::default(),
            add_review: AddReviewState::default(),
            bulk_import: BulkImportState::default(),
            settings: SettingsState {
                api_url,
                focused: SettingsField::default(),
            },
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Diary,
    AddReview,
    BulkImport,
    Settings,
}

// ── Diary ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct DiaryState {
    pub entries: Vec<DiaryEntryDto>,
    pub selected: usize,
    pub offset: u32,
    pub total: u64,
    pub history: Option<ReviewHistoryResponse>,
    pub delete_pending: Option<Uuid>,
}

// ── Add Review ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AddReviewState {
    pub external_id: String,
    pub title: String,
    pub year: String,
    pub rating: u8,
    pub watched_at: String,
    pub comment: String,
    pub focused: AddReviewField,
}

impl Default for AddReviewState {
    fn default() -> Self {
        Self {
            external_id: String::new(),
            title: String::new(),
            year: String::new(),
            rating: 5,
            watched_at: String::new(),
            comment: String::new(),
            focused: AddReviewField::ExternalId,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AddReviewField {
    #[default]
    ExternalId,
    Title,
    Year,
    Rating,
    WatchedAt,
    Comment,
    Submit,
}

// ── Bulk Import ───────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct BulkImportState {
    pub file_path: String,
    pub stage: BulkImportStage,
    pub parsed: Vec<ParsedRow>,
    pub valid_requests: Vec<LogReviewRequest>,
    // None = succeeded, Some(msg) = failed
    pub results: Vec<Option<String>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum BulkImportStage {
    #[default]
    EnterPath,
    Preview,
    Importing {
        done: usize,
    },
    Done,
}

#[derive(Debug, Clone)]
pub struct ParsedRow {
    pub row: usize,
    pub result: Result<LogReviewRequest, String>,
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct SettingsState {
    pub api_url: String,
    pub focused: SettingsField,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    #[default]
    ApiUrl,
    Save,
    Logout,
}

// ── Status bar ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StatusMsg {
    pub text: String,
    pub is_error: bool,
}

// ── Top-level App ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct App {
    pub screen: Screen,
    pub token: Option<String>,
    pub loading: bool,
    pub status: Option<StatusMsg>,
    pub api_url: String,
}

impl App {
    pub fn new(config: Option<Config>, token: Option<String>) -> Self {
        let api_url = config
            .as_ref()
            .map(|c| c.api_url.clone())
            .unwrap_or_default();
        let screen = match &config {
            None => Screen::Setup(SetupState::default()),
            Some(_) if token.is_none() => Screen::Login(LoginState::default()),
            Some(c) => Screen::Main(MainState::new(c.api_url.clone())),
        };
        Self {
            screen,
            token,
            loading: false,
            status: None,
            api_url,
        }
    }
}

// ── Action & Command (stubs; logic added in Task 5) ───────────────────────────

#[derive(Debug)]
pub enum Action {
    Quit,
    Escape,
    TabSelect(Tab),
    TabNext,
    TabPrev,
    SetupSubmit,
    InputChar(char),
    Backspace,
    FocusNext,
    FocusPrev,
    LoginSubmit,
    ScrollDown,
    ScrollUp,
    OpenHistory,
    LoadMore,
    LoadPrev,
    DeleteInit,
    DeleteConfirm,
    DeleteCancel,
    RatingUp,
    RatingDown,
    ReviewSubmit,
    BulkParseFile,
    BulkImportAll,
    BulkCancel,
    SettingsSave,
    SettingsLogout,
    // async results
    AuthOk(String),
    AuthFail(String),
    DiaryLoaded {
        entries: Vec<DiaryEntryDto>,
        total: u64,
    },
    ShowError(String),
    AuthExpired,
    HistoryLoaded(ReviewHistoryResponse),
    HistoryLoadFailed(String),
    ReviewCreated,
    ReviewCreateFailed(String),
    ReviewDeleted(Uuid),
    ReviewDeleteFailed(String),
    FileRead(String),
    FileReadFailed(String),
    BulkItemDone {
        index: usize,
        error: Option<String>,
    },
}

#[derive(Debug)]
pub enum Command {
    Login { email: String, password: String },
    LoadDiary { offset: u32 },
    LoadHistory { movie_id: Uuid },
    CreateReview(LogReviewRequest),
    DeleteReview(Uuid),
    ImportNext(usize),
    SaveConfig(String),
    SaveToken(String),
    ClearToken,
    ReadFile { path: String },
}

// Matches the export CSV column order:
// title,year,director,rating,comment,watched_at,external_metadata_id
pub fn parse_csv(content: &str) -> Vec<ParsedRow> {
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    let mut rows = Vec::new();

    for (i, result) in rdr.records().enumerate() {
        let row_num = i + 2; // 1-indexed, header is row 1
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                rows.push(ParsedRow {
                    row: row_num,
                    result: Err(e.to_string()),
                });
                continue;
            }
        };

        let title = record.get(0).unwrap_or("").trim().to_string();
        let year_str = record.get(1).unwrap_or("").trim().to_string();
        let director = record.get(2).unwrap_or("").trim().to_string();
        let rating_str = record.get(3).unwrap_or("").trim().to_string();
        let comment = record.get(4).unwrap_or("").trim().to_string();
        let watched_at = record.get(5).unwrap_or("").trim().to_string();
        let external_id = record.get(6).unwrap_or("").trim().to_string();

        if title.is_empty() && external_id.is_empty() {
            rows.push(ParsedRow {
                row: row_num,
                result: Err("title or external_id required".into()),
            });
            continue;
        }

        let rating: u8 = match rating_str.trim().parse::<u8>() {
            Ok(r) if r <= 5 => r,
            Ok(_) => {
                rows.push(ParsedRow {
                    row: row_num,
                    result: Err(format!("rating must be 0-5, got {rating_str}")),
                });
                continue;
            }
            Err(_) => {
                rows.push(ParsedRow {
                    row: row_num,
                    result: Err(format!("invalid rating: {rating_str}")),
                });
                continue;
            }
        };

        if watched_at.is_empty() {
            rows.push(ParsedRow {
                row: row_num,
                result: Err("watched_at required".into()),
            });
            continue;
        }

        let manual_release_year: Option<u16> = if year_str.is_empty() {
            None
        } else {
            match year_str.parse() {
                Ok(y) => Some(y),
                Err(_) => {
                    rows.push(ParsedRow {
                        row: row_num,
                        result: Err(format!("invalid year: {year_str}")),
                    });
                    continue;
                }
            }
        };

        rows.push(ParsedRow {
            row: row_num,
            result: Ok(LogReviewRequest {
                external_metadata_id: if external_id.is_empty() {
                    None
                } else {
                    Some(external_id)
                },
                manual_title: if title.is_empty() { None } else { Some(title) },
                manual_release_year,
                manual_director: if director.is_empty() {
                    None
                } else {
                    Some(director)
                },
                rating,
                comment: if comment.is_empty() {
                    None
                } else {
                    Some(comment)
                },
                watched_at,
                watch_medium: None,
            }),
        });
    }
    rows
}

/// Returns a mutable reference to whichever text field currently has focus,
/// or `None` if the active widget is non-textual (e.g. a rating spinner).
fn focused_input(app: &mut App) -> Option<&mut String> {
    match &mut app.screen {
        Screen::Setup(s) => Some(&mut s.api_url),
        Screen::Login(s) => match s.focused {
            LoginField::Email => Some(&mut s.email),
            LoginField::Password => Some(&mut s.password),
        },
        Screen::Main(m) => match m.tab {
            Tab::AddReview => match m.add_review.focused {
                AddReviewField::ExternalId => Some(&mut m.add_review.external_id),
                AddReviewField::Title => Some(&mut m.add_review.title),
                AddReviewField::Year => Some(&mut m.add_review.year),
                AddReviewField::WatchedAt => Some(&mut m.add_review.watched_at),
                AddReviewField::Comment => Some(&mut m.add_review.comment),
                _ => None,
            },
            Tab::BulkImport if matches!(m.bulk_import.stage, BulkImportStage::EnterPath) => {
                Some(&mut m.bulk_import.file_path)
            }
            Tab::Settings if matches!(m.settings.focused, SettingsField::ApiUrl) => {
                Some(&mut m.settings.api_url)
            }
            _ => None,
        },
    }
}

pub fn update(app: &mut App, action: Action) -> Vec<Command> {
    match action {
        // ── Global ───────────────────────────────────────────────────────────
        Action::Quit => vec![],

        Action::TabSelect(tab) => {
            if let Screen::Main(m) = &mut app.screen {
                m.tab = tab;
            }
            vec![]
        }

        Action::TabNext => {
            if let Screen::Main(m) = &mut app.screen {
                m.tab = match m.tab {
                    Tab::Diary => Tab::AddReview,
                    Tab::AddReview => Tab::BulkImport,
                    Tab::BulkImport => Tab::Settings,
                    Tab::Settings => Tab::Diary,
                };
            }
            vec![]
        }

        Action::TabPrev => {
            if let Screen::Main(m) = &mut app.screen {
                m.tab = match m.tab {
                    Tab::Diary => Tab::Settings,
                    Tab::AddReview => Tab::Diary,
                    Tab::BulkImport => Tab::AddReview,
                    Tab::Settings => Tab::BulkImport,
                };
            }
            vec![]
        }

        Action::Escape => {
            if let Screen::Main(m) = &mut app.screen {
                match m.tab {
                    Tab::Diary => {
                        if m.diary.delete_pending.is_some() {
                            m.diary.delete_pending = None;
                        } else {
                            m.diary.history = None;
                        }
                    }
                    Tab::BulkImport => {
                        if matches!(
                            m.bulk_import.stage,
                            BulkImportStage::Preview | BulkImportStage::Done
                        ) {
                            m.bulk_import.stage = BulkImportStage::EnterPath;
                        }
                    }
                    Tab::AddReview | Tab::Settings => {
                        m.tab = Tab::Diary;
                    }
                }
            }
            vec![]
        }

        // ── Shared text input ────────────────────────────────────────────────
        Action::InputChar(c) => {
            if let Some(field) = focused_input(app) {
                field.push(c);
            }
            vec![]
        }

        Action::Backspace => {
            if let Some(field) = focused_input(app) {
                field.pop();
            }
            vec![]
        }

        Action::FocusNext => {
            match &mut app.screen {
                Screen::Login(s) => {
                    s.focused = if s.focused == LoginField::Email {
                        LoginField::Password
                    } else {
                        LoginField::Email
                    };
                }
                Screen::Main(m) => match m.tab {
                    Tab::AddReview => {
                        m.add_review.focused = match m.add_review.focused {
                            AddReviewField::ExternalId => AddReviewField::Title,
                            AddReviewField::Title => AddReviewField::Year,
                            AddReviewField::Year => AddReviewField::Rating,
                            AddReviewField::Rating => AddReviewField::WatchedAt,
                            AddReviewField::WatchedAt => AddReviewField::Comment,
                            AddReviewField::Comment => AddReviewField::Submit,
                            AddReviewField::Submit => AddReviewField::ExternalId,
                        };
                    }
                    Tab::Settings => {
                        m.settings.focused = match m.settings.focused {
                            SettingsField::ApiUrl => SettingsField::Save,
                            SettingsField::Save => SettingsField::Logout,
                            SettingsField::Logout => SettingsField::ApiUrl,
                        };
                    }
                    _ => {}
                },
                _ => {}
            }
            vec![]
        }

        Action::FocusPrev => {
            match &mut app.screen {
                Screen::Login(s) => {
                    s.focused = if s.focused == LoginField::Password {
                        LoginField::Email
                    } else {
                        LoginField::Password
                    };
                }
                Screen::Main(m) => match m.tab {
                    Tab::AddReview => {
                        m.add_review.focused = match m.add_review.focused {
                            AddReviewField::ExternalId => AddReviewField::Submit,
                            AddReviewField::Title => AddReviewField::ExternalId,
                            AddReviewField::Year => AddReviewField::Title,
                            AddReviewField::Rating => AddReviewField::Year,
                            AddReviewField::WatchedAt => AddReviewField::Rating,
                            AddReviewField::Comment => AddReviewField::WatchedAt,
                            AddReviewField::Submit => AddReviewField::Comment,
                        };
                    }
                    Tab::Settings => {
                        m.settings.focused = match m.settings.focused {
                            SettingsField::ApiUrl => SettingsField::Logout,
                            SettingsField::Save => SettingsField::ApiUrl,
                            SettingsField::Logout => SettingsField::Save,
                        };
                    }
                    _ => {}
                },
                _ => {}
            }
            vec![]
        }

        // ── Setup ─────────────────────────────────────────────────────────────
        Action::SetupSubmit => {
            if let Screen::Setup(s) = &mut app.screen {
                let url = s.api_url.trim().to_string();
                if url.is_empty() {
                    s.error = Some("URL required".into());
                    return vec![];
                }
                app.api_url = url.clone();
                let cmds = if app.token.is_some() {
                    app.screen = Screen::Main(MainState::new(url.clone()));
                    vec![Command::SaveConfig(url), Command::LoadDiary { offset: 0 }]
                } else {
                    app.screen = Screen::Login(LoginState::default());
                    vec![Command::SaveConfig(url)]
                };
                return cmds;
            }
            vec![]
        }

        // ── Login ─────────────────────────────────────────────────────────────
        Action::LoginSubmit => {
            if let Screen::Login(s) = &app.screen {
                if s.email.is_empty() || s.password.is_empty() {
                    app.status = Some(StatusMsg {
                        text: "Email and password required".into(),
                        is_error: true,
                    });
                    return vec![];
                }
                let email = s.email.clone();
                let password = s.password.clone();
                app.loading = true;
                return vec![Command::Login { email, password }];
            }
            vec![]
        }

        Action::AuthOk(token) => {
            app.loading = false;
            app.status = None;
            app.screen = Screen::Main(MainState::new(app.api_url.clone()));
            let cmds = vec![
                Command::SaveToken(token.clone()),
                Command::LoadDiary { offset: 0 },
            ];
            app.token = Some(token);
            cmds
        }

        Action::AuthFail(msg) => {
            app.loading = false;
            app.status = Some(StatusMsg {
                text: msg,
                is_error: true,
            });
            vec![]
        }

        // ── Diary ─────────────────────────────────────────────────────────────
        Action::ScrollDown => {
            if let Screen::Main(m) = &mut app.screen {
                let len = m.diary.entries.len();
                if len > 0 && m.diary.selected < len - 1 {
                    m.diary.selected += 1;
                    m.diary.history = None;
                }
            }
            vec![]
        }

        Action::ScrollUp => {
            if let Screen::Main(m) = &mut app.screen
                && m.diary.selected > 0
            {
                m.diary.selected -= 1;
                m.diary.history = None;
            }
            vec![]
        }

        Action::OpenHistory => {
            if let Screen::Main(m) = &mut app.screen
                && let Some(entry) = m.diary.entries.get(m.diary.selected)
            {
                let movie_id = entry.movie.id;
                app.loading = true;
                return vec![Command::LoadHistory { movie_id }];
            }
            vec![]
        }

        Action::LoadMore => {
            if let Screen::Main(m) = &mut app.screen {
                let next = m.diary.offset + 20;
                if (next as u64) < m.diary.total {
                    m.diary.offset = next;
                    return vec![Command::LoadDiary { offset: next }];
                }
            }
            vec![]
        }

        Action::LoadPrev => {
            if let Screen::Main(m) = &mut app.screen
                && m.diary.offset > 0
            {
                let prev = m.diary.offset.saturating_sub(20);
                m.diary.offset = prev;
                return vec![Command::LoadDiary { offset: prev }];
            }
            vec![]
        }

        Action::DiaryLoaded { entries, total } => {
            app.loading = false;
            if let Screen::Main(m) = &mut app.screen {
                m.diary.total = total;
                m.diary.entries = entries;
                m.diary.selected = 0;
            }
            vec![]
        }

        Action::ShowError(msg) => {
            app.loading = false;
            app.status = Some(StatusMsg {
                text: msg,
                is_error: true,
            });
            vec![]
        }

        Action::AuthExpired => {
            app.loading = false;
            app.token = None;
            app.screen = Screen::Login(LoginState::default());
            app.status = Some(StatusMsg {
                text: "Session expired. Please log in again.".into(),
                is_error: true,
            });
            vec![Command::ClearToken]
        }

        Action::HistoryLoaded(h) => {
            app.loading = false;
            if let Screen::Main(m) = &mut app.screen {
                m.diary.history = Some(h);
            }
            vec![]
        }

        Action::HistoryLoadFailed(msg) => {
            app.loading = false;
            app.status = Some(StatusMsg {
                text: msg,
                is_error: true,
            });
            vec![]
        }

        Action::DeleteInit => {
            if let Screen::Main(m) = &mut app.screen
                && let Some(entry) = m.diary.entries.get(m.diary.selected)
            {
                m.diary.delete_pending = Some(entry.review.id);
            }
            vec![]
        }

        Action::DeleteConfirm => {
            if let Screen::Main(m) = &mut app.screen
                && let Some(review_id) = m.diary.delete_pending.take()
            {
                return vec![Command::DeleteReview(review_id)];
            }
            vec![]
        }

        Action::DeleteCancel => {
            if let Screen::Main(m) = &mut app.screen {
                m.diary.delete_pending = None;
            }
            vec![]
        }

        Action::ReviewDeleted(id) => {
            if let Screen::Main(m) = &mut app.screen {
                m.diary.entries.retain(|e| e.review.id != id);
                m.diary.total = m.diary.total.saturating_sub(1);
                if m.diary.selected >= m.diary.entries.len() {
                    m.diary.selected = m.diary.entries.len().saturating_sub(1);
                }
                m.diary.history = None;
            }
            app.status = Some(StatusMsg {
                text: "Review deleted".into(),
                is_error: false,
            });
            vec![]
        }

        Action::ReviewDeleteFailed(msg) => {
            app.status = Some(StatusMsg {
                text: msg,
                is_error: true,
            });
            vec![]
        }

        // ── Add Review ────────────────────────────────────────────────────────
        Action::RatingUp => {
            if let Screen::Main(m) = &mut app.screen
                && m.add_review.rating < 5
            {
                m.add_review.rating += 1;
            }
            vec![]
        }

        Action::RatingDown => {
            if let Screen::Main(m) = &mut app.screen
                && m.add_review.rating > 0
            {
                m.add_review.rating -= 1;
            }
            vec![]
        }

        Action::ReviewSubmit => {
            if let Screen::Main(m) = &app.screen
                && m.tab == Tab::AddReview
            {
                let f = &m.add_review;
                let has_ext = !f.external_id.is_empty();
                let has_title = !f.title.is_empty();
                let has_watched = !f.watched_at.is_empty();
                let ext_id = if has_ext {
                    Some(f.external_id.clone())
                } else {
                    None
                };
                let title = if has_title {
                    Some(f.title.clone())
                } else {
                    None
                };
                let year: Option<u16> = f.year.parse().ok();
                let rating = f.rating;
                let comment = if f.comment.is_empty() {
                    None
                } else {
                    Some(f.comment.clone())
                };
                let watched_at = f.watched_at.clone();

                if !has_ext && !has_title {
                    app.status = Some(StatusMsg {
                        text: "Title or external ID required".into(),
                        is_error: true,
                    });
                    return vec![];
                }
                if !has_watched {
                    app.status = Some(StatusMsg {
                        text: "Watched-at date required".into(),
                        is_error: true,
                    });
                    return vec![];
                }
                let req = LogReviewRequest {
                    external_metadata_id: ext_id,
                    manual_title: title,
                    manual_release_year: year,
                    manual_director: None,
                    rating,
                    comment,
                    watched_at,
                    watch_medium: None,
                };
                app.loading = true;
                return vec![Command::CreateReview(req)];
            }
            vec![]
        }

        Action::ReviewCreated => {
            app.loading = false;
            app.status = Some(StatusMsg {
                text: "Review added!".into(),
                is_error: false,
            });
            if let Screen::Main(m) = &mut app.screen {
                m.add_review = AddReviewState::default();
            }
            vec![]
        }

        Action::ReviewCreateFailed(msg) => {
            app.loading = false;
            app.status = Some(StatusMsg {
                text: msg,
                is_error: true,
            });
            vec![]
        }

        // ── Bulk Import ───────────────────────────────────────────────────────
        Action::BulkParseFile => {
            if let Screen::Main(m) = &mut app.screen
                && m.tab == Tab::BulkImport
                && m.bulk_import.stage == BulkImportStage::EnterPath
            {
                let path = m.bulk_import.file_path.trim().to_string();
                if path.is_empty() {
                    app.status = Some(StatusMsg {
                        text: "File path required".into(),
                        is_error: true,
                    });
                    return vec![];
                }
                return vec![Command::ReadFile { path }];
            }
            vec![]
        }

        Action::FileRead(content) => {
            if let Screen::Main(m) = &mut app.screen {
                m.bulk_import.parsed = parse_csv(&content);
                m.bulk_import.stage = BulkImportStage::Preview;
            }
            vec![]
        }

        Action::FileReadFailed(msg) => {
            app.status = Some(StatusMsg {
                text: format!("Cannot read file: {msg}"),
                is_error: true,
            });
            vec![]
        }

        Action::BulkImportAll => {
            if let Screen::Main(m) = &mut app.screen
                && m.tab == Tab::BulkImport
                && m.bulk_import.stage == BulkImportStage::Preview
            {
                let valid: Vec<LogReviewRequest> = m
                    .bulk_import
                    .parsed
                    .iter()
                    .filter_map(|r| r.result.as_ref().ok().cloned())
                    .collect();
                if valid.is_empty() {
                    app.status = Some(StatusMsg {
                        text: "No valid rows to import".into(),
                        is_error: true,
                    });
                    return vec![];
                }
                m.bulk_import.results = vec![None; valid.len()];
                m.bulk_import.valid_requests = valid;
                m.bulk_import.stage = BulkImportStage::Importing { done: 0 };
                return vec![Command::ImportNext(0)];
            }
            vec![]
        }

        Action::BulkCancel => {
            if let Screen::Main(m) = &mut app.screen {
                if m.bulk_import.stage == BulkImportStage::EnterPath {
                    m.tab = Tab::Diary;
                } else {
                    m.bulk_import = BulkImportState::default();
                }
            }
            vec![]
        }

        Action::BulkItemDone { index, error } => {
            if let Screen::Main(m) = &mut app.screen {
                if index >= m.bulk_import.results.len() {
                    app.status = Some(StatusMsg {
                        text: format!("Import error: unexpected index {index}"),
                        is_error: true,
                    });
                    m.bulk_import.stage = BulkImportStage::Done;
                    return vec![];
                }
                m.bulk_import.results[index] = error;
                let done = index + 1;
                let total = m.bulk_import.valid_requests.len();
                if done < total {
                    m.bulk_import.stage = BulkImportStage::Importing { done };
                    return vec![Command::ImportNext(done)];
                } else {
                    let failed = m.bulk_import.results.iter().filter(|r| r.is_some()).count();
                    m.bulk_import.stage = BulkImportStage::Done;
                    app.status = Some(StatusMsg {
                        text: format!("Import done: {} ok, {} failed", total - failed, failed),
                        is_error: failed > 0,
                    });
                }
            }
            vec![]
        }

        // ── Settings ──────────────────────────────────────────────────────────
        Action::SettingsSave => {
            if let Screen::Main(m) = &app.screen {
                let url = m.settings.api_url.trim().to_string();
                if url.is_empty() {
                    app.status = Some(StatusMsg {
                        text: "URL required".into(),
                        is_error: true,
                    });
                    return vec![];
                }
                app.status = Some(StatusMsg {
                    text: "Settings saved".into(),
                    is_error: false,
                });
                app.api_url = url.clone();
                return vec![Command::SaveConfig(url)];
            }
            vec![]
        }

        Action::SettingsLogout => {
            app.token = None;
            app.screen = Screen::Login(LoginState::default());
            app.status = None;
            vec![Command::ClearToken]
        }
    }
}

#[cfg(test)]
#[path = "tests/app.rs"]
mod tests;
