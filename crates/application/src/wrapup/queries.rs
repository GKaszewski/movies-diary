use domain::models::wrapup::{DateRange, WrapUpScope};

pub struct ComputeWrapUpQuery {
    pub scope: WrapUpScope,
    pub date_range: DateRange,
}
