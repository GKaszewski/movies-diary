use api_types::{
    CreateGoalRequest, GoalDto, GoalsResponse, UpdateGoalRequest, UpdateUserSettingsRequest,
    UserSettingsDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::goals::list_goals,
        crate::api::goals::create_goal,
        crate::api::goals::update_goal,
        crate::api::goals::delete_goal,
        crate::api::goals::get_user_goals,
        crate::api::goals::get_settings,
        crate::api::goals::update_settings,
    ),
    components(schemas(
        GoalDto,
        GoalsResponse,
        CreateGoalRequest,
        UpdateGoalRequest,
        UserSettingsDto,
        UpdateUserSettingsRequest,
    ))
)]
pub struct GoalsDoc;
