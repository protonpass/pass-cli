#[derive(Clone, Copy, Debug)]
pub struct Pagination {
    page_size: usize,
    page: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Pagination {
            page_size: Pagination::default_size(),
            page: 1,
        }
    }
}

impl Pagination {
    pub fn default_size() -> usize {
        100
    }

    pub fn default_paginated() -> Pagination {
        Pagination {
            page: 0,
            page_size: Self::default_size(),
        }
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn page_size(&self) -> usize {
        self.page_size
    }

    pub fn next(self) -> Self {
        Self {
            page: self.page + 1,
            page_size: self.page_size,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SincePagination {
    pub since: Option<String>,
    pub page_size: usize,
}

impl Default for SincePagination {
    fn default() -> Self {
        Self {
            since: None,
            page_size: Pagination::default_size(),
        }
    }
}

impl SincePagination {
    pub fn next(self, token: Option<String>) -> Option<Self> {
        match token {
            Some(t) => Some(Self {
                since: Some(t),
                page_size: self.page_size,
            }),
            None => None,
        }
    }
}
