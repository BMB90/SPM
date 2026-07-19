/// Pagination parameters shared by every repository list query. `limit`
/// clamps to `[1, 1000]`, defaulting to 100 when unset (0).
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub limit: u32,
    pub offset: u32,
}

impl Default for Pagination {
    fn default() -> Self {
        Self { limit: 100, offset: 0 }
    }
}

impl Pagination {
    pub fn new(limit: u32, offset: u32) -> Self {
        Self { limit, offset }
    }

    pub fn effective_limit(&self) -> i64 {
        if self.limit == 0 {
            100
        } else {
            self.limit.min(1000) as i64
        }
    }

    pub fn effective_offset(&self) -> i64 {
        self.offset as i64
    }
}

/// A page of results plus the total row count matching the (unpaginated)
/// filter, so the UI can render "showing 1-100 of 4213".
#[derive(Debug, Clone, serde::Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}
