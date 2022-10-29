use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AsnInfo {
    asn: u32,
    as_name: Option<String>,
    org_id: Option<String>,
    org_name: Option<String>,
    country_code: Option<String>,
    country_name: Option<String>,
    data_source: Option<String>,
}