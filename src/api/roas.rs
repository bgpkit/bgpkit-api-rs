use std::sync::Arc;
use axum::extract::Query;
use axum::{Extension, Json};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use crate::api::Pagination;
use crate::db::BgpkitDatabase;
use chrono::prelude::*;
use tracing::info;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct RoasEntry {
    /// Autonomous system (AS) number
    asn: u32,

    /// maximum prefix length for this ROA
    max_len: u32,

    /// prefix
    prefix: String,

    /// trust anchor locator
    tal: String,

    /// the ROA is still valid at least on previous day UTC.
    current: bool,

    /// ROA valid date ranges
    date_ranges: Vec<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoasRawEntry {
    /// Autonomous system (AS) number
    asn: u32,

    /// maximum prefix length for this ROA
    max_len: u32,

    /// prefix
    prefix: String,

    /// trust anchor locator
    tal: String,

    /// ROA valid date ranges
    date_ranges: Vec<String>,
}

impl RoasRawEntry {

    /// process raw ROAs database query results and fix single-day gaps if there is any
    fn to_roas_entry(self, fix_gaps: bool) -> RoasEntry {
        let mut current = false;
        let mut date_ranges: Vec<Vec<Date<Utc>>> = self.date_ranges.into_iter().map(|date_range|{

            let start_exclusive = date_range.starts_with('(');
            let end_exclusive = date_range.ends_with(')');

            let dates: Vec<&str> = (&date_range).trim_matches(|c|char::is_ascii_punctuation(&c)).split(",").collect();
            let mut date_0 = Utc.datetime_from_str(format!("{}T00:00:00Z", dates[0]).as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap().date();
            let mut date_1 = Utc.datetime_from_str(format!("{}T00:00:00Z", dates[1]).as_str(), "%Y-%m-%dT%H:%M:%SZ").unwrap().date();

            if start_exclusive {
                date_0 = date_0 + Duration::days(1);
            }
            if end_exclusive {
                date_1 = date_1 - Duration::days(1);
            }

            if date_1 >= (Utc::now() - Duration::days(1)).date() {
                // The last valid day is at least one day before now
                current = true;
            }

            vec![
                date_0,
                date_1
            ]
        }).collect();

        if fix_gaps {
            info!("fixing gaps");
            let mut cur_start = date_ranges[0][0];
            let mut cur_end = date_ranges[0][1];
            let mut new_ranges = vec![];

            for i in 1..(date_ranges.len()) {
                let date_0 = date_ranges[i][0];
                let date_1 = date_ranges[i][1];
                if cur_end == date_0-Duration::days(2) {
                    cur_end = date_1;
                } else {
                    new_ranges.push(vec![cur_start, cur_end]);
                    cur_start = date_0;
                    cur_end = date_1;
                }
            }
            new_ranges.push(vec![cur_start, cur_end]);

            date_ranges = new_ranges
        }


        let date_ranges_strs: Vec<Vec<String>> = date_ranges.into_iter().map(|range|{
            vec![
                range[0].format("%Y-%m-%d").to_string(),
                range[1].format("%Y-%m-%d").to_string(),
            ]
        }).collect();

        RoasEntry{
            asn: self.asn,
            max_len: self.max_len,
            prefix: self.prefix,
            tal: self.tal,
            current,
            date_ranges: date_ranges_strs
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RoasResponse {
    page: usize,
    page_size: usize,
    data: Vec<RoasEntry>
}

#[derive(Deserialize, IntoParams, Debug)]
pub struct RoasSearchQuery {
    /// filter results by ASN exact match
    asn: Option<u32>,

    /// IP prefix to search ROAs for, e.g. `?prefix=1.1.1.0/24`.
    prefix: Option<String>,

    /// filer results by trusted anchor, supported values are `apnic`, `afrinic`, `lacnic`, `ripencc`, `arin`
    tal: Option<String>,

    /// limit the date of the ROAs, format: YYYY-MM-DD, e.g. `?date=2022-01-01`
    date: Option<String>,

    /// filter results to whether ROA is still current
    current: Option<bool>,

    /// filter results by the max_len value
    max_len: Option<u32>,
}

/// Search for information regarding autonomous systems.
///
/// **NOTE**: only valid prefix match will be returned, i.e. the prefix must be contained within
/// (or equals to) a prefix of a ROA entry and the length of the prefix must be equal or smaller
/// than the max_length specified by the ROA.
#[utoipa::path(
    get,
    tag = "bgp",
    path = "/roas",
    responses(
        (status = 200, description = "ROV information found", body = RoasResponse),
    ),
    params(
        RoasSearchQuery,
        Pagination
    )
)]
pub async fn search_roas(
    Extension(db): Extension<Arc<BgpkitDatabase>>,
    query: Query<RoasSearchQuery>,
    pagination: Query<Pagination>,
) -> Json<RoasResponse> {

    // parse pagination parameters
    let page = match pagination.page {
        None => 0 as usize,
        Some(p) => p
    };
    let page_size = match &pagination.page_size {
        None => { 100 as usize }
        Some(p) => {
            match *p > 1000 {
                true => 1000 as usize,
                false => *p
            }
        }
    };
    let offset = page * page_size;

    let mut query_str_array = vec![
        format!(r#""res_limit": {}"#, page_size),
        format!(r#""res_offset": {}"#, offset),
    ];
    query_str_array.push(
        format!(r#""prefix": {}"#,
                match &query.prefix {
                    None => {"\"\"".to_string()}
                    Some(v) => {format!("\"{}\"", v)}
                }
        )
    );
    query_str_array.push(
        format!(r#""asn": {}"#,
                match &query.asn {
                    None => {"-1".to_string()}
                    Some(v) => {format!("{}", v)}
                }
        )
    );
    query_str_array.push(
        format!(r#""max_len": {}"#,
                match &query.max_len {
                    None => {"-1".to_string()}
                    Some(v) => {format!("{}", v)}
                }
        )
    );
    query_str_array.push(
        format!(r#""nic": {}"#,
                match &query.tal {
                    None => {"\"\"".to_string()}
                    Some(v) => {format!("\"{}\"", v)}
                }
        )
    );

    match &query.current {
        None => {
            query_str_array.push(
            format!(r#""date": {}"#,
                    match &query.date {
                        None => {"\"\"".to_string()}
                        Some(v) => {format!("\"{}\"", v)}
                    }
            ));
            query_str_array.push( format!(r#""not_date": """#));
        }
        Some(current) => {
            match current {
                true => {
                    let date = (Utc::today() - Duration::days(1)).format("%Y-%m-%d").to_string();
                    query_str_array.push( format!(r#""date": "{}""#, date));
                    query_str_array.push( format!(r#""not_date": """#));
                },
                false => {
                    let date = (Utc::today() - Duration::days(1)).format("%Y-%m-%d").to_string();
                    query_str_array.push( format!(r#""not_date": "{}""#, date));
                    query_str_array.push( format!(r#""date": """#));
                }
            }
        }
    }

    // construct final RPC query string
    let query_string = format!("{{ {} }}", query_str_array.join(","));
    info!("{}",&query_string);

    // execute RPC call
    let response = db.client.rpc("query_history", query_string).execute().await.unwrap();

    // gather response json text
    let resp_text = response.text().await.unwrap();

    // convert date ranges to tuples
    let raw_data: Vec<RoasRawEntry> = serde_json::from_str(resp_text.as_str()).unwrap();
    let data: Vec<RoasEntry> = raw_data.into_iter().map(|entry|{
        entry.to_roas_entry(true)
    }).collect();

    let response = RoasResponse{
        page,
        page_size,
        data
    };

    Json(response)
}
