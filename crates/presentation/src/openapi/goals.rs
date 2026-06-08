use api_types::{
    CreateGoalRequest, GoalDto, GoalsResponse, UpdateGoalRequest, UpdateUserSettingsRequest,
    UserSettingsDto,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::goals::list_goals,
        crate::handlers::goals::create_goal,
        crate::handlers::goals::update_goal,
        crate::handlers::goals::delete_goal,
        crate::handlers::goals::get_user_goals,
        crate::handlers::goals::get_settings,
        crate::handlers::goals::update_settings,
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
