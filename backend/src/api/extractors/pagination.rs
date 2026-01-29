use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_PAGE_SIZE: i64 = 100;

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}

#[derive(Debug, Clone)]
pub struct Pagination {
    pub limit: i64,
    pub offset: i64,
    pub page: i64,
    pub page_size: i64,
}

impl Pagination {
    pub fn new(page: i64, page_size: i64) -> Self {
        let page = page.max(1);
        let page_size = page_size.clamp(1, MAX_PAGE_SIZE);
        let offset = (page - 1) * page_size;

        Self {
            limit: page_size,
            offset,
            page,
            page_size,
        }
    }
}

impl<S> FromRequestParts<S> for Pagination
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<PaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::Validation(format!("Invalid pagination parameters: {}", e)))?;

        Ok(Pagination::new(query.page, query.page_size))
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub page_size: i64,
    pub total_items: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationMeta {
    pub fn new(page: i64, page_size: i64, total_items: i64) -> Self {
        let total_pages = (total_items as f64 / page_size as f64).ceil() as i64;

        Self {
            page,
            page_size,
            total_items,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, pagination: &Pagination, total_items: i64) -> Self {
        Self {
            data,
            pagination: PaginationMeta::new(pagination.page, pagination.page_size, total_items),
        }
    }
}
