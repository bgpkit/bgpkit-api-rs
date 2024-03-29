use crate::api::error::ApiError;
use crate::api::Pagination;
use crate::db::{execute, BgpkitDatabase};
use axum::extract::Query;
use axum::{Extension, Json};
use chrono::prelude::*;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use utoipa::{IntoParams, ToSchema};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct BrokerEntry {
    ts_start: String,
    ts_end: String,

    project: String,
    collector: String,

    data_type: String,
    url: String,
    size: u32,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct BrokerRawEntry {
    ts_start: String,
    ts_end: String,
    collector_id: String,
    data_type: String,
    url: String,
    rough_size: u32,
    exact_size: u32,
}

impl BrokerRawEntry {
    fn to_entry(self) -> BrokerEntry {
        let project = match self.collector_id.contains("rrc") {
            true => "riperis".to_string(),
            false => "route-views".to_string(),
        };
        BrokerEntry {
            ts_start: self.ts_start,
            ts_end: self.ts_end,
            project,
            collector: self.collector_id,
            data_type: self.data_type,
            url: self.url,
            size: self.rough_size,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct BrokerResponse {
    page: usize,
    page_size: usize,

    /// count of items returned in current query
    count: usize,

    data: Vec<BrokerEntry>,
}

#[derive(Deserialize, IntoParams, Debug)]
pub struct BrokerSearchQuery {
    ts_start: Option<String>,
    ts_end: Option<String>,

    /// duration before `ts_end` or after `ts_start`
    duration: Option<String>,

    /// filter by route collector projects, i.e. `route-views` or `riperis`
    project: Option<String>,

    /// filter by collector IDs, e.g. 'rrc00', 'route-views2. use comma to separate multiple collectors
    collectors: Option<String>,
    data_type: Option<String>,
}

/// Search for information regarding autonomous systems.
///
/// **NOTE**: only valid prefix match will be returned, i.e. the prefix must be contained within
/// (or equals to) a prefix of a ROA entry and the length of the prefix must be equal or smaller
/// than the max_length specified by the ROA.
#[utoipa::path(
    get,
    tag = "bgp",
    path = "/broker",
    responses(
        (status = 200, description = "public MRT files found", body = BrokerResponse),
    ),
    params(
        BrokerSearchQuery,
        Pagination
    )
)]
pub async fn search_broker(
    Extension(db): Extension<Arc<BgpkitDatabase>>,
    query: Query<BrokerSearchQuery>,
    pagination: Query<Pagination>,
) -> Result<Json<BrokerResponse>, ApiError> {
    let mut db_query = db.client.from("items").select("*");

    //////////////////
    // TIME FILTERS //
    //////////////////
    let mut ts_start = None;
    let mut ts_end = None;
    if let Some(ts_end_str) = &query.ts_end {
        ts_end = if let Ok(ts_end) = ts_end_str.parse::<i64>() {
            // it's unix timestamp
            Some(NaiveDateTime::from_timestamp(ts_end, 0))
        } else {
            match NaiveDateTime::from_str(ts_end_str) {
                Ok(t) => Some(t),
                Err(_) => {
                    return Err(ApiError::new_bad_request(format!(
                        "cannot parse time string: {}",
                        ts_end_str
                    )))
                }
            }
        };
    }

    if let Some(ts_start_str) = &query.ts_start {
        ts_start = if let Ok(ts_start) = ts_start_str.parse::<i64>() {
            // it's unix timestamp
            Some(NaiveDateTime::from_timestamp(ts_start, 0))
        } else {
            match NaiveDateTime::from_str(ts_start_str) {
                Ok(t) => Some(t),
                Err(_) => {
                    return Err(ApiError::new_bad_request(format!(
                        "cannot parse time string: {}",
                        ts_start_str
                    )))
                }
            }
        };
    }

    match (ts_start, ts_end) {
        (Some(start), None) => {
            if let Some(duration_str) = &query.duration {
                match humantime::parse_duration(duration_str.as_str()) {
                    Ok(d) => {
                        ts_end = Some(start + Duration::from_std(d).unwrap());
                    }
                    Err(_) => {
                        return Err(ApiError::new_bad_request(format!(
                            "cannot parse time duration string: {}",
                            duration_str
                        )))
                    }
                }
            }
        }
        (None, Some(end)) => {
            if let Some(duration_str) = &query.duration {
                match humantime::parse_duration(duration_str.as_str()) {
                    Ok(d) => {
                        ts_start = Some(end - Duration::from_std(d).unwrap());
                    }
                    Err(_) => {
                        return Err(ApiError::new_bad_request(format!(
                            "cannot parse time duration string: {}",
                            duration_str
                        )))
                    }
                }
            }
        }
        _ => {}
    };

    if let Some(ts_end) = ts_end {
        let ts_str = ts_end.format("%Y-%m-%dT%X").to_string();
        db_query = db_query.lte("ts_start", ts_str);
    }

    if let Some(ts_start) = ts_start {
        let ts_str = ts_start.format("%Y-%m-%dT%X").to_string();
        db_query = db_query.gte("ts_end", ts_str);
    }

    ///////////////////////
    // COLLECTOR FILTERS //
    ///////////////////////

    if let Some(project) = &query.project {
        match project.as_str() {
            "route-views" | "routeviews" | "rv" => {
                db_query = db_query.ilike("collector_id", "route-views%");
            }
            "ripe" | "ripencc" | "riperis" | "ris" => {
                db_query = db_query.ilike("collector_id", "rrc%");
            }
            _ => {
                // TODO: handle unrecognized cases
            }
        }
    }

    if let Some(collectors_str) = &query.collectors {
        let collectors: Vec<&str> = collectors_str.split(",").map(|c| c.trim()).collect();
        info!("{:?}", &collectors);
        db_query = db_query.in_("collector_id", collectors);
    }

    ////////////
    // OTHERS //
    ////////////

    if let Some(data_type) = &query.data_type {
        match data_type.to_lowercase().as_str() {
            "update" | "updates" | "u" => {
                db_query = db_query.eq("data_type", "update");
            }
            "rib" | "ribs" | "r" => {
                db_query = db_query.eq("data_type", "rib");
            }
            _ => {}
        }
    }

    db_query = db_query.order("ts_start.asc");

    let (page, page_size) = pagination.extract(1000);
    let low = page * page_size;
    let high = (page + 1) * page_size - 1;
    db_query = db_query.range(low, high);

    let response = execute(db_query).await?;

    let data: Vec<BrokerEntry> = serde_json::from_str::<Vec<BrokerRawEntry>>(response.as_str())
        .unwrap()
        .into_iter()
        .map(|entry| entry.to_entry())
        .collect();
    let count = data.len();
    let response = BrokerResponse {
        page,
        page_size,
        data,
        count,
    };

    Ok(Json(response))
}
