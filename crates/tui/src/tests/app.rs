use super::*;
use api_types::{DiaryEntryDto, MovieDto, ReviewDto};
use uuid::Uuid;

fn setup_app() -> App {
    App {
        screen: Screen::Setup(SetupState {
            api_url: String::new(),
            error: None,
        }),
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
        movie: MovieDto {
            id: Uuid::new_v4(),
            title: "The Matrix".into(),
            release_year: 1999,
            director: None,
            poster_path: None,
        },
        review: ReviewDto {
            id: Uuid::new_v4(),
            rating: 5,
            comment: None,
            watched_at: "1999-03-31T00:00:00".into(),
        },
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
    } else {
        panic!("expected Setup");
    }
}

#[test]
fn setup_submit_with_empty_url_sets_error() {
    let mut app = setup_app();
    let cmds = update(&mut app, Action::SetupSubmit);
    assert!(cmds.is_empty());
    if let Screen::Setup(s) = &app.screen {
        assert!(s.error.is_some());
    } else {
        panic!("expected Setup");
    }
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
    } else {
        panic!();
    }
}

#[test]
fn login_focus_next_moves_to_password() {
    let mut app = login_app();
    update(&mut app, Action::FocusNext);
    if let Screen::Login(s) = &app.screen {
        assert_eq!(s.focused, LoginField::Password);
    } else {
        panic!();
    }
}

#[test]
fn login_input_after_focus_goes_to_password() {
    let mut app = login_app();
    update(&mut app, Action::FocusNext);
    update(&mut app, Action::InputChar('x'));
    if let Screen::Login(s) = &app.screen {
        assert_eq!(s.password, "x");
    } else {
        panic!();
    }
}

#[test]
fn login_submit_returns_login_command_and_sets_loading() {
    let mut app = login_app();
    for c in "user@example.com".chars() {
        update(&mut app, Action::InputChar(c));
    }
    update(&mut app, Action::FocusNext);
    for c in "pass123".chars() {
        update(&mut app, Action::InputChar(c));
    }
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
    update(
        &mut app,
        Action::DiaryLoaded {
            entries: vec![diary_entry(), diary_entry(), diary_entry()],
            total: 3,
        },
    );
    update(&mut app, Action::ScrollDown);
    if let Screen::Main(m) = &app.screen {
        assert_eq!(m.diary.selected, 1);
    } else {
        panic!();
    }
}

#[test]
fn diary_scroll_up_clamps_at_zero() {
    let mut app = main_app();
    update(
        &mut app,
        Action::DiaryLoaded {
            entries: vec![diary_entry()],
            total: 1,
        },
    );
    update(&mut app, Action::ScrollUp);
    if let Screen::Main(m) = &app.screen {
        assert_eq!(m.diary.selected, 0);
    } else {
        panic!();
    }
}

#[test]
fn diary_scroll_down_clamps_at_last_entry() {
    let mut app = main_app();
    update(
        &mut app,
        Action::DiaryLoaded {
            entries: vec![diary_entry()],
            total: 1,
        },
    );
    update(&mut app, Action::ScrollDown);
    if let Screen::Main(m) = &app.screen {
        assert_eq!(m.diary.selected, 0);
    } else {
        panic!();
    }
}

#[test]
fn delete_init_sets_delete_pending() {
    let mut app = main_app();
    let entry = diary_entry();
    let review_id = entry.review.id;
    update(
        &mut app,
        Action::DiaryLoaded {
            entries: vec![entry],
            total: 1,
        },
    );
    update(&mut app, Action::DeleteInit);
    if let Screen::Main(m) = &app.screen {
        assert_eq!(m.diary.delete_pending, Some(review_id));
    } else {
        panic!();
    }
}

#[test]
fn delete_confirm_returns_delete_command() {
    let mut app = main_app();
    let entry = diary_entry();
    let review_id = entry.review.id;
    update(
        &mut app,
        Action::DiaryLoaded {
            entries: vec![entry],
            total: 1,
        },
    );
    update(&mut app, Action::DeleteInit);
    let cmds = update(&mut app, Action::DeleteConfirm);
    assert!(
        cmds.iter()
            .any(|c| matches!(c, Command::DeleteReview(id) if *id == review_id))
    );
}

#[test]
fn delete_cancel_clears_pending() {
    let mut app = main_app();
    let entry = diary_entry();
    update(
        &mut app,
        Action::DiaryLoaded {
            entries: vec![entry],
            total: 1,
        },
    );
    update(&mut app, Action::DeleteInit);
    update(&mut app, Action::DeleteCancel);
    if let Screen::Main(m) = &app.screen {
        assert!(m.diary.delete_pending.is_none());
    } else {
        panic!();
    }
}

#[test]
fn review_deleted_removes_entry_from_list() {
    let mut app = main_app();
    let entry = diary_entry();
    let review_id = entry.review.id;
    update(
        &mut app,
        Action::DiaryLoaded {
            entries: vec![entry],
            total: 1,
        },
    );
    update(&mut app, Action::ReviewDeleted(review_id));
    if let Screen::Main(m) = &app.screen {
        assert!(m.diary.entries.is_empty());
        assert_eq!(m.diary.total, 0);
    } else {
        panic!();
    }
}

// ── Add Review ────────────────────────────────────────────────────────────

#[test]
fn rating_up_increments_rating() {
    let mut app = main_app();
    if let Screen::Main(m) = &mut app.screen {
        m.tab = Tab::AddReview;
        m.add_review.rating = 3;
    }
    update(&mut app, Action::RatingUp);
    if let Screen::Main(m) = &app.screen {
        assert_eq!(m.add_review.rating, 4);
    }
}

#[test]
fn rating_clamps_at_5() {
    let mut app = main_app();
    if let Screen::Main(m) = &mut app.screen {
        m.tab = Tab::AddReview;
        m.add_review.rating = 5;
    }
    update(&mut app, Action::RatingUp);
    if let Screen::Main(m) = &app.screen {
        assert_eq!(m.add_review.rating, 5);
    }
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
        m.bulk_import.parsed = vec![ParsedRow {
            row: 2,
            result: Ok(LogReviewRequest {
                external_metadata_id: None,
                manual_title: Some("The Matrix".into()),
                manual_release_year: None,
                manual_director: None,
                rating: 5,
                comment: None,
                watched_at: "1999-03-31T00:00:00".into(),
            }),
        }];
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
            LogReviewRequest {
                external_metadata_id: None,
                manual_title: Some("A".into()),
                manual_release_year: None,
                manual_director: None,
                rating: 5,
                comment: None,
                watched_at: "2024-01-01T00:00:00".into(),
            },
            LogReviewRequest {
                external_metadata_id: None,
                manual_title: Some("B".into()),
                manual_release_year: None,
                manual_director: None,
                rating: 4,
                comment: None,
                watched_at: "2024-01-02T00:00:00".into(),
            },
        ];
        m.bulk_import.results = vec![None, None];
    }
    let cmds = update(
        &mut app,
        Action::BulkItemDone {
            index: 0,
            error: None,
        },
    );
    assert!(cmds.iter().any(|c| matches!(c, Command::ImportNext(1))));
}

#[test]
fn bulk_item_done_last_item_transitions_to_done() {
    let mut app = main_app();
    if let Screen::Main(m) = &mut app.screen {
        m.tab = Tab::BulkImport;
        m.bulk_import.stage = BulkImportStage::Importing { done: 0 };
        m.bulk_import.valid_requests = vec![LogReviewRequest {
            external_metadata_id: None,
            manual_title: Some("A".into()),
            manual_release_year: None,
            manual_director: None,
            rating: 5,
            comment: None,
            watched_at: "2024-01-01T00:00:00".into(),
        }];
        m.bulk_import.results = vec![None];
    }
    let cmds = update(
        &mut app,
        Action::BulkItemDone {
            index: 0,
            error: None,
        },
    );
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
    assert!(
        cmds.iter()
            .any(|c| matches!(c, Command::SaveConfig(url) if url.contains("8080")))
    );
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
    } else {
        panic!("expected Main");
    }
}

// ── parse_csv ─────────────────────────────────────────────────────────────

// CSV column order matches the export format:
// title,year,director,rating,comment,watched_at,external_metadata_id

#[test]
fn parse_csv_valid_row_with_title() {
    let csv = "title,year,director,rating,comment,watched_at,external_metadata_id\nThe Matrix,1999,Wachowski,5,,1999-03-31T00:00:00,\n";
    let rows = parse_csv(csv);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].result.is_ok());
    let req = rows[0].result.as_ref().unwrap();
    assert_eq!(req.manual_title.as_deref(), Some("The Matrix"));
    assert_eq!(req.manual_director.as_deref(), Some("Wachowski"));
    assert_eq!(req.rating, 5);
}

#[test]
fn parse_csv_row_missing_title_and_id_is_error() {
    let csv = "title,year,director,rating,comment,watched_at,external_metadata_id\n,,,5,,2024-01-01T00:00:00,\n";
    let rows = parse_csv(csv);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].result.is_err());
}

#[test]
fn parse_csv_invalid_rating_is_error() {
    let csv = "title,year,director,rating,comment,watched_at,external_metadata_id\nThe Matrix,,,9,,2024-01-01T00:00:00,\n";
    let rows = parse_csv(csv);
    assert!(rows[0].result.is_err());
}

#[test]
fn parse_csv_with_external_id_only() {
    let csv = "title,year,director,rating,comment,watched_at,external_metadata_id\n,,,5,,1999-03-31T00:00:00,tt0133093\n";
    let rows = parse_csv(csv);
    assert!(rows[0].result.is_ok());
    let req = rows[0].result.as_ref().unwrap();
    assert_eq!(req.external_metadata_id.as_deref(), Some("tt0133093"));
    assert!(req.manual_title.is_none());
}

#[test]
fn parse_csv_rating_zero_is_valid() {
    let csv = "title,year,director,rating,comment,watched_at,external_metadata_id\nThe Matrix,,,0,,2024-01-01T00:00:00,\n";
    let rows = parse_csv(csv);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].result.is_ok());
    let req = rows[0].result.as_ref().unwrap();
    assert_eq!(req.rating, 0);
}
