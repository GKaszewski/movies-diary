use uuid::Uuid;
use crate::client::{DiaryEntryDto, LogReviewRequest, ReviewHistoryResponse};
use crate::config::Config;

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
pub enum LoginField { #[default] Email, Password }

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
            settings: SettingsState { api_url, focused: SettingsField::default() },
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab { #[default] Diary, AddReview, BulkImport, Settings }

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
    #[default] ExternalId, Title, Year, Rating, WatchedAt, Comment, Submit,
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
    #[default] EnterPath,
    Preview,
    Importing { done: usize },
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
pub enum SettingsField { #[default] ApiUrl, Save, Logout }

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
        let api_url = config.as_ref().map(|c| c.api_url.clone()).unwrap_or_default();
        let screen = match &config {
            None => Screen::Setup(SetupState::default()),
            Some(_) if token.is_none() => Screen::Login(LoginState::default()),
            Some(c) => Screen::Main(MainState::new(c.api_url.clone())),
        };
        Self { screen, token, loading: false, status: None, api_url }
    }
}

// ── Action & Command (stubs; logic added in Task 5) ───────────────────────────

#[derive(Debug)]
pub enum Action {
    Quit, Escape, TabSelect(Tab), TabNext, TabPrev,
    SetupSubmit,
    InputChar(char), Backspace, FocusNext, FocusPrev,
    LoginSubmit,
    ScrollDown, ScrollUp, OpenHistory, LoadMore,
    DeleteInit, DeleteConfirm, DeleteCancel,
    RatingUp, RatingDown, ReviewSubmit,
    BulkParseFile, BulkImportAll, BulkCancel,
    SettingsSave, SettingsLogout,
    // async results
    AuthOk(String), AuthFail(String),
    DiaryLoaded { entries: Vec<DiaryEntryDto>, total: u64 },
    DiaryLoadFailed(String),
    HistoryLoaded(ReviewHistoryResponse),
    HistoryLoadFailed(String),
    ReviewCreated, ReviewCreateFailed(String),
    ReviewDeleted(Uuid), ReviewDeleteFailed(String),
    BulkItemDone { index: usize, error: Option<String> },
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
}

pub fn parse_csv(content: &str) -> Vec<ParsedRow> {
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    let mut rows = Vec::new();

    for (i, result) in rdr.records().enumerate() {
        let row_num = i + 2; // 1-indexed, header is row 1
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                rows.push(ParsedRow { row: row_num, result: Err(e.to_string()) });
                continue;
            }
        };

        let title = record.get(0).unwrap_or("").trim().to_string();
        let year_str = record.get(1).unwrap_or("").trim().to_string();
        let external_id = record.get(2).unwrap_or("").trim().to_string();
        let rating_str = record.get(3).unwrap_or("").trim().to_string();
        let watched_at = record.get(4).unwrap_or("").trim().to_string();
        let comment = record.get(5).unwrap_or("").trim().to_string();

        if title.is_empty() && external_id.is_empty() {
            rows.push(ParsedRow { row: row_num, result: Err("title or external_id required".into()) });
            continue;
        }

        let rating: u8 = match rating_str.trim().parse::<u8>() {
            Ok(r) if r <= 5 => r,
            Ok(_) => { rows.push(ParsedRow { row: row_num, result: Err(format!("rating must be 0-5, got {rating_str}")) }); continue; }
            Err(_) => { rows.push(ParsedRow { row: row_num, result: Err(format!("invalid rating: {rating_str}")) }); continue; }
        };

        if watched_at.is_empty() {
            rows.push(ParsedRow { row: row_num, result: Err("watched_at required".into()) });
            continue;
        }

        let manual_release_year: Option<u16> = if year_str.is_empty() {
            None
        } else {
            match year_str.parse() {
                Ok(y) => Some(y),
                Err(_) => { rows.push(ParsedRow { row: row_num, result: Err(format!("invalid year: {year_str}")) }); continue; }
            }
        };

        rows.push(ParsedRow {
            row: row_num,
            result: Ok(LogReviewRequest {
                external_metadata_id: if external_id.is_empty() { None } else { Some(external_id) },
                manual_title: if title.is_empty() { None } else { Some(title) },
                manual_release_year,
                rating,
                comment: if comment.is_empty() { None } else { Some(comment) },
                watched_at,
            }),
        });
    }
    rows
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
                m.tab = match m.tab { Tab::Diary => Tab::AddReview, Tab::AddReview => Tab::BulkImport, Tab::BulkImport => Tab::Settings, Tab::Settings => Tab::Diary };
            }
            vec![]
        }

        Action::TabPrev => {
            if let Screen::Main(m) = &mut app.screen {
                m.tab = match m.tab { Tab::Diary => Tab::Settings, Tab::AddReview => Tab::Diary, Tab::BulkImport => Tab::AddReview, Tab::Settings => Tab::BulkImport };
            }
            vec![]
        }

        Action::Escape => {
            if let Screen::Main(m) = &mut app.screen {
                match m.tab {
                    Tab::Diary => {
                        if m.diary.delete_pending.is_some() { m.diary.delete_pending = None; }
                        else { m.diary.history = None; }
                    }
                    Tab::BulkImport => {
                        if matches!(m.bulk_import.stage, BulkImportStage::Preview | BulkImportStage::Done) {
                            m.bulk_import.stage = BulkImportStage::EnterPath;
                        }
                    }
                    _ => {}
                }
            }
            vec![]
        }

        // ── Shared text input ────────────────────────────────────────────────
        Action::InputChar(c) => {
            match &mut app.screen {
                Screen::Setup(s) => s.api_url.push(c),
                Screen::Login(s) => match s.focused {
                    LoginField::Email => s.email.push(c),
                    LoginField::Password => s.password.push(c),
                },
                Screen::Main(m) => match m.tab {
                    Tab::AddReview => match m.add_review.focused {
                        AddReviewField::ExternalId => m.add_review.external_id.push(c),
                        AddReviewField::Title => m.add_review.title.push(c),
                        AddReviewField::Year => m.add_review.year.push(c),
                        AddReviewField::WatchedAt => m.add_review.watched_at.push(c),
                        AddReviewField::Comment => m.add_review.comment.push(c),
                        _ => {}
                    },
                    Tab::BulkImport if matches!(m.bulk_import.stage, BulkImportStage::EnterPath) => {
                        m.bulk_import.file_path.push(c);
                    }
                    Tab::Settings if matches!(m.settings.focused, SettingsField::ApiUrl) => {
                        m.settings.api_url.push(c);
                    }
                    _ => {}
                },
            }
            vec![]
        }

        Action::Backspace => {
            match &mut app.screen {
                Screen::Setup(s) => { s.api_url.pop(); }
                Screen::Login(s) => match s.focused {
                    LoginField::Email => { s.email.pop(); }
                    LoginField::Password => { s.password.pop(); }
                },
                Screen::Main(m) => match m.tab {
                    Tab::AddReview => match m.add_review.focused {
                        AddReviewField::ExternalId => { m.add_review.external_id.pop(); }
                        AddReviewField::Title => { m.add_review.title.pop(); }
                        AddReviewField::Year => { m.add_review.year.pop(); }
                        AddReviewField::WatchedAt => { m.add_review.watched_at.pop(); }
                        AddReviewField::Comment => { m.add_review.comment.pop(); }
                        _ => {}
                    },
                    Tab::BulkImport if matches!(m.bulk_import.stage, BulkImportStage::EnterPath) => {
                        m.bulk_import.file_path.pop();
                    }
                    Tab::Settings if matches!(m.settings.focused, SettingsField::ApiUrl) => {
                        m.settings.api_url.pop();
                    }
                    _ => {}
                },
            }
            vec![]
        }

        Action::FocusNext => {
            match &mut app.screen {
                Screen::Login(s) => {
                    s.focused = if s.focused == LoginField::Email { LoginField::Password } else { LoginField::Email };
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
                    s.focused = if s.focused == LoginField::Password { LoginField::Email } else { LoginField::Password };
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
                app.screen = Screen::Login(LoginState::default());
                return vec![Command::SaveConfig(url)];
            }
            vec![]
        }

        // ── Login ─────────────────────────────────────────────────────────────
        Action::LoginSubmit => {
            if let Screen::Login(s) = &app.screen {
                if s.email.is_empty() || s.password.is_empty() {
                    app.status = Some(StatusMsg { text: "Email and password required".into(), is_error: true });
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
            let cmds = vec![Command::SaveToken(token.clone()), Command::LoadDiary { offset: 0 }];
            app.token = Some(token);
            cmds
        }

        Action::AuthFail(msg) => {
            app.loading = false;
            app.status = Some(StatusMsg { text: msg, is_error: true });
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
            if let Screen::Main(m) = &mut app.screen {
                if m.diary.selected > 0 {
                    m.diary.selected -= 1;
                    m.diary.history = None;
                }
            }
            vec![]
        }

        Action::OpenHistory => {
            if let Screen::Main(m) = &mut app.screen {
                if let Some(entry) = m.diary.entries.get(m.diary.selected) {
                    let movie_id = entry.movie.id;
                    app.loading = true;
                    return vec![Command::LoadHistory { movie_id }];
                }
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

        Action::DiaryLoaded { entries, total } => {
            app.loading = false;
            if let Screen::Main(m) = &mut app.screen {
                m.diary.total = total;
                m.diary.entries = entries;
                m.diary.selected = 0;
            }
            vec![]
        }

        Action::DiaryLoadFailed(msg) => {
            app.loading = false;
            if msg.contains("unauthorized") || msg.contains("Unauthorized") {
                app.token = None;
                app.screen = Screen::Login(LoginState::default());
                app.status = Some(StatusMsg { text: "Session expired. Please log in again.".into(), is_error: true });
                return vec![Command::ClearToken];
            }
            app.status = Some(StatusMsg { text: msg, is_error: true });
            vec![]
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
            app.status = Some(StatusMsg { text: msg, is_error: true });
            vec![]
        }

        Action::DeleteInit => {
            if let Screen::Main(m) = &mut app.screen {
                if let Some(entry) = m.diary.entries.get(m.diary.selected) {
                    m.diary.delete_pending = Some(entry.review.id);
                }
            }
            vec![]
        }

        Action::DeleteConfirm => {
            if let Screen::Main(m) = &mut app.screen {
                if let Some(review_id) = m.diary.delete_pending.take() {
                    return vec![Command::DeleteReview(review_id)];
                }
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
            app.status = Some(StatusMsg { text: "Review deleted".into(), is_error: false });
            vec![]
        }

        Action::ReviewDeleteFailed(msg) => {
            app.status = Some(StatusMsg { text: msg, is_error: true });
            vec![]
        }

        // ── Add Review ────────────────────────────────────────────────────────
        Action::RatingUp => {
            if let Screen::Main(m) = &mut app.screen {
                if m.add_review.rating < 5 { m.add_review.rating += 1; }
            }
            vec![]
        }

        Action::RatingDown => {
            if let Screen::Main(m) = &mut app.screen {
                if m.add_review.rating > 0 { m.add_review.rating -= 1; }
            }
            vec![]
        }

        Action::ReviewSubmit => {
            if let Screen::Main(m) = &app.screen {
                if m.tab == Tab::AddReview {
                    let f = &m.add_review;
                    let has_ext = !f.external_id.is_empty();
                    let has_title = !f.title.is_empty();
                    let has_watched = !f.watched_at.is_empty();
                    let ext_id = if has_ext { Some(f.external_id.clone()) } else { None };
                    let title = if has_title { Some(f.title.clone()) } else { None };
                    let year: Option<u16> = f.year.parse().ok();
                    let rating = f.rating;
                    let comment = if f.comment.is_empty() { None } else { Some(f.comment.clone()) };
                    let watched_at = f.watched_at.clone();

                    if !has_ext && !has_title {
                        app.status = Some(StatusMsg { text: "Title or external ID required".into(), is_error: true });
                        return vec![];
                    }
                    if !has_watched {
                        app.status = Some(StatusMsg { text: "Watched-at date required".into(), is_error: true });
                        return vec![];
                    }
                    let req = LogReviewRequest {
                        external_metadata_id: ext_id,
                        manual_title: title,
                        manual_release_year: year,
                        rating,
                        comment,
                        watched_at,
                    };
                    app.loading = true;
                    return vec![Command::CreateReview(req)];
                }
            }
            vec![]
        }

        Action::ReviewCreated => {
            app.loading = false;
            app.status = Some(StatusMsg { text: "Review added!".into(), is_error: false });
            if let Screen::Main(m) = &mut app.screen {
                m.add_review = AddReviewState::default();
            }
            vec![]
        }

        Action::ReviewCreateFailed(msg) => {
            app.loading = false;
            app.status = Some(StatusMsg { text: msg, is_error: true });
            vec![]
        }

        // ── Bulk Import ───────────────────────────────────────────────────────
        Action::BulkParseFile => {
            if let Screen::Main(m) = &mut app.screen {
                if m.tab == Tab::BulkImport && m.bulk_import.stage == BulkImportStage::EnterPath {
                    let path = m.bulk_import.file_path.trim().to_string();
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            m.bulk_import.parsed = parse_csv(&content);
                            m.bulk_import.stage = BulkImportStage::Preview;
                        }
                        Err(e) => {
                            app.status = Some(StatusMsg { text: format!("Cannot read file: {e}"), is_error: true });
                        }
                    }
                }
            }
            vec![]
        }

        Action::BulkImportAll => {
            if let Screen::Main(m) = &mut app.screen {
                if m.tab == Tab::BulkImport && m.bulk_import.stage == BulkImportStage::Preview {
                    let valid: Vec<LogReviewRequest> = m.bulk_import.parsed.iter()
                        .filter_map(|r| r.result.as_ref().ok().cloned())
                        .collect();
                    if valid.is_empty() {
                        app.status = Some(StatusMsg { text: "No valid rows to import".into(), is_error: true });
                        return vec![];
                    }
                    m.bulk_import.results = vec![None; valid.len()];
                    m.bulk_import.valid_requests = valid;
                    m.bulk_import.stage = BulkImportStage::Importing { done: 0 };
                    return vec![Command::ImportNext(0)];
                }
            }
            vec![]
        }

        Action::BulkCancel => {
            if let Screen::Main(m) = &mut app.screen {
                m.bulk_import = BulkImportState::default();
            }
            vec![]
        }

        Action::BulkItemDone { index, error } => {
            if let Screen::Main(m) = &mut app.screen {
                if index >= m.bulk_import.results.len() {
                    app.status = Some(StatusMsg { text: format!("Import error: unexpected index {index}"), is_error: true });
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
                    app.status = Some(StatusMsg { text: "URL required".into(), is_error: true });
                    return vec![];
                }
                app.status = Some(StatusMsg { text: "Settings saved".into(), is_error: false });
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
mod tests {
    use super::*;
    use crate::client::{DiaryEntryDto, MovieDto, ReviewDto};
    use uuid::Uuid;

    fn setup_app() -> App {
        App {
            screen: Screen::Setup(SetupState { api_url: String::new(), error: None }),
            token: None,
            loading: false,
            status: None,
            api_url: String::new(),
        }
    }

    fn login_app() -> App {
        App {
            screen: Screen::Login(LoginState::default()),
            token: None,
            loading: false,
            status: None,
            api_url: String::new(),
        }
    }

    fn main_app() -> App {
        App {
            screen: Screen::Main(MainState::new("http://localhost:3000".into())),
            token: Some("tok".into()),
            loading: false,
            status: None,
            api_url: "http://localhost:3000".into(),
        }
    }

    fn diary_entry() -> DiaryEntryDto {
        DiaryEntryDto {
            movie: MovieDto { id: Uuid::new_v4(), title: "The Matrix".into(), release_year: 1999, director: None },
            review: ReviewDto { id: Uuid::new_v4(), rating: 5, comment: None, watched_at: "1999-03-31T00:00:00".into() },
        }
    }

    // ── Setup screen ──────────────────────────────────────────────────────────

    #[test]
    fn setup_input_char_appends_to_api_url() {
        let mut app = setup_app();
        update(&mut app, Action::InputChar('h'));
        update(&mut app, Action::InputChar('i'));
        if let Screen::Setup(s) = &app.screen {
            assert_eq!(s.api_url, "hi");
        } else { panic!("expected Setup"); }
    }

    #[test]
    fn setup_submit_with_empty_url_sets_error() {
        let mut app = setup_app();
        let cmds = update(&mut app, Action::SetupSubmit);
        assert!(cmds.is_empty());
        if let Screen::Setup(s) = &app.screen {
            assert!(s.error.is_some());
        } else { panic!("expected Setup"); }
    }

    #[test]
    fn setup_submit_with_url_saves_config_and_transitions_to_login() {
        let mut app = setup_app();
        update(&mut app, Action::InputChar('h'));
        let cmds = update(&mut app, Action::SetupSubmit);
        assert!(cmds.iter().any(|c| matches!(c, Command::SaveConfig(_))));
        assert!(matches!(app.screen, Screen::Login(_)));
    }

    // ── Login screen ──────────────────────────────────────────────────────────

    #[test]
    fn login_input_char_goes_to_email_by_default() {
        let mut app = login_app();
        update(&mut app, Action::InputChar('a'));
        if let Screen::Login(s) = &app.screen {
            assert_eq!(s.email, "a");
            assert_eq!(s.password, "");
        } else { panic!(); }
    }

    #[test]
    fn login_focus_next_moves_to_password() {
        let mut app = login_app();
        update(&mut app, Action::FocusNext);
        if let Screen::Login(s) = &app.screen {
            assert_eq!(s.focused, LoginField::Password);
        } else { panic!(); }
    }

    #[test]
    fn login_input_after_focus_goes_to_password() {
        let mut app = login_app();
        update(&mut app, Action::FocusNext);
        update(&mut app, Action::InputChar('x'));
        if let Screen::Login(s) = &app.screen {
            assert_eq!(s.password, "x");
        } else { panic!(); }
    }

    #[test]
    fn login_submit_returns_login_command_and_sets_loading() {
        let mut app = login_app();
        for c in "user@example.com".chars() { update(&mut app, Action::InputChar(c)); }
        update(&mut app, Action::FocusNext);
        for c in "pass123".chars() { update(&mut app, Action::InputChar(c)); }
        let cmds = update(&mut app, Action::LoginSubmit);
        assert!(cmds.iter().any(|c| matches!(c, Command::Login { .. })));
        assert!(app.loading);
    }

    #[test]
    fn login_submit_with_empty_fields_sets_error_status() {
        let mut app = login_app();
        let cmds = update(&mut app, Action::LoginSubmit);
        assert!(cmds.is_empty());
        assert!(app.status.as_ref().map_or(false, |s| s.is_error));
    }

    #[test]
    fn auth_ok_sets_token_and_transitions_to_main() {
        let mut app = login_app();
        let cmds = update(&mut app, Action::AuthOk("jwt-token".into()));
        assert_eq!(app.token, Some("jwt-token".into()));
        assert!(matches!(app.screen, Screen::Main(_)));
        assert!(!app.loading);
        assert!(cmds.iter().any(|c| matches!(c, Command::SaveToken(_))));
        assert!(cmds.iter().any(|c| matches!(c, Command::LoadDiary { .. })));
    }

    #[test]
    fn auth_fail_sets_error_status_and_clears_loading() {
        let mut app = login_app();
        app.loading = true;
        update(&mut app, Action::AuthFail("bad creds".into()));
        assert!(!app.loading);
        assert!(app.status.as_ref().map_or(false, |s| s.is_error));
    }

    // ── Diary ─────────────────────────────────────────────────────────────────

    #[test]
    fn diary_scroll_down_increments_selected() {
        let mut app = main_app();
        update(&mut app, Action::DiaryLoaded {
            entries: vec![diary_entry(), diary_entry(), diary_entry()],
            total: 3,
        });
        update(&mut app, Action::ScrollDown);
        if let Screen::Main(m) = &app.screen {
            assert_eq!(m.diary.selected, 1);
        } else { panic!(); }
    }

    #[test]
    fn diary_scroll_up_clamps_at_zero() {
        let mut app = main_app();
        update(&mut app, Action::DiaryLoaded { entries: vec![diary_entry()], total: 1 });
        update(&mut app, Action::ScrollUp);
        if let Screen::Main(m) = &app.screen {
            assert_eq!(m.diary.selected, 0);
        } else { panic!(); }
    }

    #[test]
    fn diary_scroll_down_clamps_at_last_entry() {
        let mut app = main_app();
        update(&mut app, Action::DiaryLoaded { entries: vec![diary_entry()], total: 1 });
        update(&mut app, Action::ScrollDown);
        if let Screen::Main(m) = &app.screen {
            assert_eq!(m.diary.selected, 0);
        } else { panic!(); }
    }

    #[test]
    fn delete_init_sets_delete_pending() {
        let mut app = main_app();
        let entry = diary_entry();
        let review_id = entry.review.id;
        update(&mut app, Action::DiaryLoaded { entries: vec![entry], total: 1 });
        update(&mut app, Action::DeleteInit);
        if let Screen::Main(m) = &app.screen {
            assert_eq!(m.diary.delete_pending, Some(review_id));
        } else { panic!(); }
    }

    #[test]
    fn delete_confirm_returns_delete_command() {
        let mut app = main_app();
        let entry = diary_entry();
        let review_id = entry.review.id;
        update(&mut app, Action::DiaryLoaded { entries: vec![entry], total: 1 });
        update(&mut app, Action::DeleteInit);
        let cmds = update(&mut app, Action::DeleteConfirm);
        assert!(cmds.iter().any(|c| matches!(c, Command::DeleteReview(id) if *id == review_id)));
    }

    #[test]
    fn delete_cancel_clears_pending() {
        let mut app = main_app();
        let entry = diary_entry();
        update(&mut app, Action::DiaryLoaded { entries: vec![entry], total: 1 });
        update(&mut app, Action::DeleteInit);
        update(&mut app, Action::DeleteCancel);
        if let Screen::Main(m) = &app.screen {
            assert!(m.diary.delete_pending.is_none());
        } else { panic!(); }
    }

    #[test]
    fn review_deleted_removes_entry_from_list() {
        let mut app = main_app();
        let entry = diary_entry();
        let review_id = entry.review.id;
        update(&mut app, Action::DiaryLoaded { entries: vec![entry], total: 1 });
        update(&mut app, Action::ReviewDeleted(review_id));
        if let Screen::Main(m) = &app.screen {
            assert!(m.diary.entries.is_empty());
            assert_eq!(m.diary.total, 0);
        } else { panic!(); }
    }

    // ── Add Review ────────────────────────────────────────────────────────────

    #[test]
    fn rating_up_increments_rating() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen { m.tab = Tab::AddReview; m.add_review.rating = 3; }
        update(&mut app, Action::RatingUp);
        if let Screen::Main(m) = &app.screen { assert_eq!(m.add_review.rating, 4); }
    }

    #[test]
    fn rating_clamps_at_5() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen { m.tab = Tab::AddReview; m.add_review.rating = 5; }
        update(&mut app, Action::RatingUp);
        if let Screen::Main(m) = &app.screen { assert_eq!(m.add_review.rating, 5); }
    }

    #[test]
    fn review_submit_returns_create_review_command() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen {
            m.tab = Tab::AddReview;
            m.add_review.title = "The Matrix".into();
            m.add_review.watched_at = "1999-03-31T00:00:00".into();
            m.add_review.rating = 5;
        }
        let cmds = update(&mut app, Action::ReviewSubmit);
        assert!(cmds.iter().any(|c| matches!(c, Command::CreateReview(_))));
    }

    #[test]
    fn review_submit_with_missing_title_and_id_sets_error() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen {
            m.tab = Tab::AddReview;
            m.add_review.watched_at = "1999-03-31T00:00:00".into();
        }
        let cmds = update(&mut app, Action::ReviewSubmit);
        assert!(cmds.is_empty());
        assert!(app.status.as_ref().map_or(false, |s| s.is_error));
    }

    // ── Bulk Import ───────────────────────────────────────────────────────────

    #[test]
    fn bulk_import_all_with_valid_rows_returns_import_next_command() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen {
            m.tab = Tab::BulkImport;
            m.bulk_import.stage = BulkImportStage::Preview;
            m.bulk_import.parsed = vec![
                ParsedRow {
                    row: 2,
                    result: Ok(LogReviewRequest {
                        external_metadata_id: None,
                        manual_title: Some("The Matrix".into()),
                        manual_release_year: None,
                        rating: 5,
                        comment: None,
                        watched_at: "1999-03-31T00:00:00".into(),
                    }),
                },
            ];
        }
        let cmds = update(&mut app, Action::BulkImportAll);
        assert!(cmds.iter().any(|c| matches!(c, Command::ImportNext(0))));
    }

    #[test]
    fn bulk_item_done_advances_stage_and_returns_next_command() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen {
            m.tab = Tab::BulkImport;
            m.bulk_import.stage = BulkImportStage::Importing { done: 0 };
            m.bulk_import.valid_requests = vec![
                LogReviewRequest { external_metadata_id: None, manual_title: Some("A".into()), manual_release_year: None, rating: 5, comment: None, watched_at: "2024-01-01T00:00:00".into() },
                LogReviewRequest { external_metadata_id: None, manual_title: Some("B".into()), manual_release_year: None, rating: 4, comment: None, watched_at: "2024-01-02T00:00:00".into() },
            ];
            m.bulk_import.results = vec![None, None];
        }
        let cmds = update(&mut app, Action::BulkItemDone { index: 0, error: None });
        assert!(cmds.iter().any(|c| matches!(c, Command::ImportNext(1))));
    }

    #[test]
    fn bulk_item_done_last_item_transitions_to_done() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen {
            m.tab = Tab::BulkImport;
            m.bulk_import.stage = BulkImportStage::Importing { done: 0 };
            m.bulk_import.valid_requests = vec![
                LogReviewRequest { external_metadata_id: None, manual_title: Some("A".into()), manual_release_year: None, rating: 5, comment: None, watched_at: "2024-01-01T00:00:00".into() },
            ];
            m.bulk_import.results = vec![None];
        }
        let cmds = update(&mut app, Action::BulkItemDone { index: 0, error: None });
        assert!(cmds.is_empty());
        if let Screen::Main(m) = &app.screen {
            assert!(matches!(m.bulk_import.stage, BulkImportStage::Done));
        }
        assert!(app.status.is_some());
    }

    // ── Settings ──────────────────────────────────────────────────────────────

    #[test]
    fn settings_save_returns_save_config_command() {
        let mut app = main_app();
        if let Screen::Main(m) = &mut app.screen {
            m.tab = Tab::Settings;
            m.settings.api_url = "http://new-server:8080".into();
        }
        let cmds = update(&mut app, Action::SettingsSave);
        assert!(cmds.iter().any(|c| matches!(c, Command::SaveConfig(url) if url.contains("8080"))));
    }

    #[test]
    fn settings_logout_clears_token_and_goes_to_login() {
        let mut app = main_app();
        let cmds = update(&mut app, Action::SettingsLogout);
        assert!(app.token.is_none());
        assert!(matches!(app.screen, Screen::Login(_)));
        assert!(cmds.iter().any(|c| matches!(c, Command::ClearToken)));
    }

    #[test]
    fn auth_ok_uses_app_api_url_for_main_state() {
        let mut app = login_app();
        app.api_url = "http://test-server:9000".into();
        update(&mut app, Action::AuthOk("tok".into()));
        if let Screen::Main(m) = &app.screen {
            assert_eq!(m.settings.api_url, "http://test-server:9000");
        } else { panic!("expected Main"); }
    }

    // ── parse_csv ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_csv_valid_row_with_title() {
        let csv = "title,year,external_id,rating,watched_at,comment\nThe Matrix,1999,,5,1999-03-31T00:00:00,\n";
        let rows = parse_csv(csv);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].result.is_ok());
        let req = rows[0].result.as_ref().unwrap();
        assert_eq!(req.manual_title.as_deref(), Some("The Matrix"));
        assert_eq!(req.rating, 5);
    }

    #[test]
    fn parse_csv_row_missing_title_and_id_is_error() {
        let csv = "title,year,external_id,rating,watched_at,comment\n,,,5,2024-01-01T00:00:00,\n";
        let rows = parse_csv(csv);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].result.is_err());
    }

    #[test]
    fn parse_csv_invalid_rating_is_error() {
        let csv = "title,year,external_id,rating,watched_at,comment\nThe Matrix,,,9,2024-01-01T00:00:00,\n";
        let rows = parse_csv(csv);
        assert!(rows[0].result.is_err());
    }

    #[test]
    fn parse_csv_with_external_id_only() {
        let csv = "title,year,external_id,rating,watched_at,comment\n,,tt0133093,5,1999-03-31T00:00:00,\n";
        let rows = parse_csv(csv);
        assert!(rows[0].result.is_ok());
        let req = rows[0].result.as_ref().unwrap();
        assert_eq!(req.external_metadata_id.as_deref(), Some("tt0133093"));
        assert!(req.manual_title.is_none());
    }

    #[test]
    fn parse_csv_rating_zero_is_valid() {
        let csv = "title,year,external_id,rating,watched_at,comment\nThe Matrix,,,0,2024-01-01T00:00:00,\n";
        let rows = parse_csv(csv);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].result.is_ok());
        let req = rows[0].result.as_ref().unwrap();
        assert_eq!(req.rating, 0);
    }
}
