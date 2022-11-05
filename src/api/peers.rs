use std::str::FromStr;
use crate::api::{ApiError, Pagination};
use crate::db::BgpkitDatabase;
use axum::extract::Query;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::NaiveDate;
use utoipa::{IntoParams, ToSchema};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct PeerStats {
    /// Date of the query
    pub date: String,

    /// Route collector ID
    pub collector: String,

    /// Route collector peer IP address
    pub ip: String,

    /// Peer's AS number
    pub asn: i64,

    /// Number of unique IPv4 prefixes this peer receives
    pub num_v4_pfxs: i64,

    /// Number of unique IPv6 prefixes this peer receives
    pub num_v6_pfxs: i64,

    /// Number of connected ASes
    pub num_connected_asns: i64,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PeerStatsResponse {
    page: usize,
    page_size: usize,
    count: usize,
    data: Vec<PeerStats>,
}

#[derive(Deserialize, IntoParams, Debug)]
pub struct PeerStatsSearchQuery {
    /// filter results by peer ASN exact match
    ip: Option<String>,

    /// filter results by peer ASN exact match
    asn: Option<u32>,

    /// filter results by date, only applicable if `latest=false` is set
    date: Option<String>,

    /// filter by collector ID, e.g. rrc00
    collector: Option<String>,

    /// filter by minimum number of IPv4 prefixes
    min_v4: Option<u32>,

    /// filter by minimum number of IPv6 prefixes
    min_v6: Option<u32>,

    /// filter by minimum number of connected ASNs
    min_connected: Option<u32>,

    /// show latest information, default true
    latest: Option<bool>
}

/// Public route collector peers information.
#[utoipa::path(
    get,
    tag = "meta",
    path = "/peers",
    responses(
        (status = 200, description = "Route collector peers information", body = PeerStatsResponse),
    ),
    params(
        PeerStatsSearchQuery,
        Pagination
    )
)]
pub async fn search_peer_stats(
    Extension(db): Extension<Arc<BgpkitDatabase>>,
    query: Query<PeerStatsSearchQuery>,
    pagination: Query<Pagination>,
) -> Result<Json<PeerStatsResponse>, ApiError> {

    let mut is_latest = false;
    let table = match &query.latest{
        None => {
            is_latest = true;
            "peer_stats_latest"
        }
        Some(v) => {
            if *v {
                is_latest = true;
                "peer_stats_latest"
            } else {
                // only search historical one when explicitly specified
                "peer_stats"
            }
        }
    };
    let mut db_query = db.client.from(table).select("*");

    if let Some(asn) = &query.asn {
        db_query = db_query.eq("asn", asn.to_string());
    }

    if let Some(collector) = &query.collector {
        db_query = db_query.ilike("collector", collector);
    }

    if let Some(ip) = &query.ip {
        db_query = db_query.eq("ip", ip);
    }

    if !is_latest {
        if let Some(date) = &query.date {
            match NaiveDate::from_str(date) {
                Ok(d) => {
                    db_query = db_query.eq("date", d.to_string());
                }
                Err(_) => {
                    return Err(ApiError::new_bad_request(format!(
                        "cannot parse date string: {}",
                        date
                    )));
                }
            };
        }
    }

    if let Some(min_v4) = &query.min_v4 {
        db_query = db_query.gte("num_v4_pfxs", min_v4.to_string());
    }

    if let Some(min_v6) = &query.min_v6 {
        db_query = db_query.gte("num_v6_pfxs", min_v6.to_string());
    }

    if let Some(min_connected) = &query.min_connected {
        db_query = db_query.gte("num_connected_asns", min_connected.to_string());
    }

    let (page, page_size) = match is_latest {
        true => {
            (0, 10000)
        }
        false => {
            pagination.extract(1000)
        }
    };

    let low = page * page_size;
    let high = (page + 1) * page_size - 1;
    db_query = db_query.range(low, high);

    let response = db_query.execute().await.unwrap();
    let response_text = response.text().await.unwrap();
    let data: Vec<PeerStats> = serde_json::from_str(response_text.as_str()).unwrap();
    let count = data.len();
    let response = PeerStatsResponse {
        page,
        page_size,
        count,
        data,
    };
    Ok(Json(response))
}
