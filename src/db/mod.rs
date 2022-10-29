use postgrest::Postgrest;

pub struct BgpkitDatabase {
    pub client: Postgrest,
}

impl BgpkitDatabase {
    pub fn new() -> Self {
        dotenvy::dotenv().unwrap();
        let api_key = dotenvy::var("POSTGREST_API_KEY").expect("required environment variable POSTGREST_API_KEY not set");
        let endpoint = dotenvy::var("POSTGREST_ENDPOINT").expect("required environment variable POSTGREST_ENDPOINT not set");
        let client = Postgrest::new(endpoint)
            .insert_header("apikey", api_key);
        Self{client}
    }
}

#[cfg(test)]
mod tests {
    use crate::api::AsnInfo;
    use super::*;

    #[tokio::test]
    async fn test_connection() {
        let db = BgpkitDatabase::new();
        let data = db.client.from("asn_view")
            .select("*")
            .limit(10)
            .execute().await.unwrap();
        let objects: Vec<AsnInfo> = serde_json::from_str(data.text().await.unwrap().as_str()).unwrap();
        dbg!(objects);
    }
}