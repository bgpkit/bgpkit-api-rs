mod asninfo;
mod roas;

pub(crate) use asninfo::*;
pub(crate) use roas::*;

use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
pub struct Pagination {
    /// page number, starting from 0
    page: Option<usize>,

    /// page size, default to 10
    page_size: Option<usize>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self { page: Some(1), page_size: Some(10) }
    }
}

