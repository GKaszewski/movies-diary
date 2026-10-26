use chrono::NaiveDate;
use uuid::Uuid;

use domain::models::{FileFormat, UserRole, WatchEventSource};
use domain::testing::test_watched_at;

use application::auth::commands::RegisterCommand;
use application::auth::queries::LoginCommand;
use application::diary::commands::{
    EditReviewCommand, LogReviewCommand, MovieInput, SyncPosterCommand,
};
use application::goals::commands::{CreateGoalCommand, UpdateGoalCommand};
use application::import::commands::{
    ApplyImportMappingCommand, ApplyImportProfileCommand, CreateImportSessionCommand,
    ExecuteImportCommand,
};
use application::integrations::commands::{
    ConfirmWatchEventsCommand, DismissWatchEventsCommand, GenerateWebhookTokenCommand,
    WatchEventConfirmation,
};
use application::users::commands::UpdateProfileCommand;
use application::users::update_settings::UpdateUserSettingsCommand;
use application::watchlist::commands::AddToWatchlistCommand;
use application::wrapup::commands::RequestWrapUpCommand;

pub fn movie_input_by_id(movie_id: Uuid) -> MovieInput {
    MovieInput {
        movie_id: Some(movie_id),
        external_metadata_id: None,
        manual_title: None,
        manual_release_year: None,
        manual_director: None,
    }
}

pub fn movie_input_manual(title: &str, year: u16) -> MovieInput {
    MovieInput {
        movie_id: None,
        external_metadata_id: None,
        manual_title: Some(title.to_string()),
        manual_release_year: Some(year),
        manual_director: None,
    }
}

pub fn log_review_command(user_id: Uuid, movie_id: Uuid) -> LogReviewCommand {
    LogReviewCommand {
        user_id,
        input: movie_input_by_id(movie_id),
        rating: 4,
        comment: None,
        watched_at: test_watched_at(),
        watch_medium: None,
        spoilers: false,
    }
}

pub fn log_review_command_manual(user_id: Uuid, title: &str, year: u16) -> LogReviewCommand {
    LogReviewCommand {
        input: movie_input_manual(title, year),
        ..log_review_command(user_id, Uuid::nil())
    }
}

pub fn edit_review_command(rating: u8) -> EditReviewCommand {
    EditReviewCommand {
        review_id: Uuid::new_v4(),
        requesting_user_id: Uuid::new_v4(),
        rating: Some(rating),
        comment: None,
        watched_at: None,
        watch_medium: None,
        spoilers: None,
    }
}

pub fn sync_poster_command(movie_id: Uuid) -> SyncPosterCommand {
    SyncPosterCommand { movie_id }
}

pub fn register_command(email: impl Into<String>, username: impl Into<String>) -> RegisterCommand {
    RegisterCommand {
        email: email.into(),
        username: username.into(),
        password: "password123".to_string(),
        role: UserRole::Standard,
    }
}

pub fn login_command(email: impl Into<String>, password: impl Into<String>) -> LoginCommand {
    LoginCommand {
        email: email.into(),
        password: password.into(),
    }
}

pub fn create_goal_command(year: u16, target_count: u32) -> CreateGoalCommand {
    CreateGoalCommand {
        user_id: Uuid::nil(),
        year,
        target_count,
    }
}

pub fn update_goal_command(year: u16, target_count: u32) -> UpdateGoalCommand {
    UpdateGoalCommand {
        user_id: Uuid::nil(),
        year,
        target_count,
    }
}

pub fn execute_import_command(user_id: Uuid, session_id: Uuid) -> ExecuteImportCommand {
    ExecuteImportCommand {
        user_id,
        session_id,
        confirmed_indices: vec![0],
    }
}

pub fn create_import_session_command(user_id: Uuid, csv: &[u8]) -> CreateImportSessionCommand {
    CreateImportSessionCommand {
        user_id,
        bytes: csv.to_vec(),
        format: FileFormat::Csv,
    }
}

pub fn apply_import_mapping_command(user_id: Uuid, session_id: Uuid) -> ApplyImportMappingCommand {
    ApplyImportMappingCommand {
        user_id,
        session_id,
        mappings: vec![],
    }
}

pub fn apply_import_profile_command(
    user_id: Uuid,
    session_id: Uuid,
    profile_id: Uuid,
) -> ApplyImportProfileCommand {
    ApplyImportProfileCommand {
        user_id,
        session_id,
        profile_id,
    }
}

pub fn confirm_watch_events_command(
    user_id: Uuid,
    watch_event_id: Uuid,
    rating: u8,
) -> ConfirmWatchEventsCommand {
    ConfirmWatchEventsCommand {
        user_id,
        confirmations: vec![WatchEventConfirmation {
            watch_event_id,
            rating,
            comment: None,
        }],
    }
}

pub fn dismiss_watch_events_command(
    user_id: Uuid,
    event_ids: Vec<Uuid>,
) -> DismissWatchEventsCommand {
    DismissWatchEventsCommand { user_id, event_ids }
}

pub fn generate_webhook_token_command(user_id: Uuid) -> GenerateWebhookTokenCommand {
    GenerateWebhookTokenCommand {
        user_id,
        provider: WatchEventSource::Jellyfin,
        label: None,
    }
}

pub fn update_profile_command(user_id: Uuid) -> UpdateProfileCommand {
    UpdateProfileCommand {
        user_id,
        display_name: None,
        bio: None,
        avatar_bytes: None,
        avatar_content_type: None,
        banner_bytes: None,
        banner_content_type: None,
        also_known_as: None,
    }
}

pub fn update_user_settings_command(user_id: Uuid) -> UpdateUserSettingsCommand {
    UpdateUserSettingsCommand {
        user_id,
        federate_goals: true,
        federate_reviews: true,
        federate_watchlist: true,
    }
}

pub fn add_to_watchlist_command(user_id: Uuid, movie_id: Uuid) -> AddToWatchlistCommand {
    AddToWatchlistCommand {
        user_id,
        input: movie_input_by_id(movie_id),
    }
}

pub fn request_wrap_up_command(user_id: Option<Uuid>, year: i32) -> RequestWrapUpCommand {
    RequestWrapUpCommand {
        user_id,
        start_date: NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap(),
    }
}
