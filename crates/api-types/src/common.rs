use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PaginationQueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
