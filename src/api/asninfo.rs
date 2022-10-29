use std::sync::Arc;
use axum::extract::Query;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use crate::db::BgpkitDatabase;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AsnInfo {
    /// Autonomous system (AS) number
    asn: u32,

    /// AS name
    as_name: Option<String>,

    /// Organization ID based on CAIDA's as2org dataset
    org_id: Option<String>,

    /// Organization name based on CAIDA's as2org dataset
    org_name: Option<String>,

    /// Registration country in two-letter code format
    country_code: Option<String>,

    /// Registration country full name
    country_name: Option<String>,

    /// RIR source
    data_source: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AsninfoResponse {
    page: usize,
    page_size: usize,
    data: Vec<AsnInfo>
}

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

#[derive(Deserialize, IntoParams, Debug)]
pub struct AsninfoSearchQuery {
    /// filter results by ASN exact match
    asn: Option<u32>,

    /// filter results that has asn in the specified array, formatted as ','-separated string
    asns: Option<String>,

    /// filter results by AS name or organization name
    name: Option<String>,

    /// filter by two-letter country code or country name
    country: Option<String>,
}

/// Search for information regarding autonomous systems.
#[utoipa::path(
    get,
    tag = "meta",
    path = "/asninfo",
    responses(
        (status = 200, description = "ASN information found", body = AsninfoResponse),
    ),
    params(
        AsninfoSearchQuery,
        Pagination
    )
)]
pub async fn search_asninfo(
    Extension(db): Extension<Arc<BgpkitDatabase>>,
    query: Query<AsninfoSearchQuery>,
    pagination: Query<Pagination>,
) -> Json<AsninfoResponse> {
    let mut db_query = db.client.from("asn_view").select("*");

    if let Some(asn) = &query.asn {
        db_query = db_query.eq("asn", asn.to_string());
    }

    if let Some(asns_str) = &query.asns {
        let asns: Vec<&str> = asns_str.split(",").collect();
        db_query = db_query.in_("asn", asns);
    }

    if let Some(country) = &query.country {
        db_query = db_query.or(format!(r#"country_code.ilike."{}", country_name.ilike."*{}*""#, country, country));
    }

    if let Some(name) = &query.name {
        db_query = db_query.or(format!(r#"as_name.ilike."*{}*", org_name.ilike."*{}*""#, name, name));
    }


    let page_size = match &pagination.page_size {
        None => { 50 as usize }
        Some(p) => {
            match *p > 1000 {
                true => 1000 as usize,
                false => *p
            }
        }
    };


    let page = match pagination.page {
        None => 0 as usize,
        Some(p) => p
    };

    let low = page * page_size;
    let high = (page+1) * page_size - 1;
    db_query = db_query.range(low, high);

    let response = db_query.execute().await.unwrap();
    let data: Vec<AsnInfo> = serde_json::from_str(response.text().await.unwrap().as_str()).unwrap();
    let response = AsninfoResponse {
        page,
        page_size,
        data
    };
    Json(
        response
    )
}
