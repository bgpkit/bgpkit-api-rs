mod asninfo;
mod broker;
mod error;
mod roas;

pub(crate) use asninfo::*;
pub(crate) use broker::*;
pub(crate) use error::*;
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

impl Pagination {
    pub fn extract(&self, max_page_size: usize) -> (usize, usize) {
        (
            match self.page {
                None => 0,
                Some(p) => p,
            },
            match self.page_size {
                None => 10,
                Some(p) => match p > max_page_size {
                    true => max_page_size,
                    false => p,
                },
            },
        )
    }
}

// TODO: error handling https://github.com/tokio-rs/axum/blob/main/examples/customize-extractor-error/src/with_rejection.rs
