use crate::GoalDto;

pub fn goal_with_progress_to_dto(g: &domain::models::GoalWithProgress) -> GoalDto {
    GoalDto {
        year: g.goal.year(),
        target_count: g.goal.target_count(),
        current_count: g.current_count,
        percentage: g.percentage(),
        is_complete: g.is_complete(),
        goal_type: g.goal.goal_type().clone(),
    }
}
