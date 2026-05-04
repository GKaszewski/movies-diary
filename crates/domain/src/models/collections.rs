use crate::errors::DomainError;

#[derive(Clone, Debug)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total_count: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Clone, Debug)]
pub struct PageParams {
    pub limit: u32,
    pub offset: u32,
}

impl PageParams {
    const MAX_LIMIT: u32 = 100;
    const DEFAULT_LIMIT: u32 = 5;

    pub fn new(limit: Option<u32>, offset: Option<u32>) -> Result<Self, DomainError> {
        let l = limit.unwrap_or(Self::DEFAULT_LIMIT);
        if l == 0 || l > Self::MAX_LIMIT {
            return Err(DomainError::ValidationError(format!(
                "Limit must be between 1 and {}",
                Self::MAX_LIMIT
            )));
        }
        Ok(Self {
            limit: l,
            offset: offset.unwrap_or(0),
        })
    }
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            limit: Self::DEFAULT_LIMIT,
            offset: 0,
        }
    }
}
