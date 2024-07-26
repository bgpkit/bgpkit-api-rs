use crate::api::ApiError;
use postgrest::Builder;
use postgrest::Postgrest;

pub struct BgpkitDatabase {
    pub client: Postgrest,
}

impl BgpkitDatabase {
    pub fn new() -> Self {
        dotenvy::dotenv().ok();
        let api_key = std::env::var("POSTGREST_API_KEY")
            .expect("required environment variable POSTGREST_API_KEY not set");
        let endpoint = std::env::var("POSTGREST_ENDPOINT")
            .expect("required environment variable POSTGREST_ENDPOINT not set");
        let client = Postgrest::new(endpoint).insert_header("apikey", api_key);
        Self { client }
    }
}

pub async fn execute(builder: Builder) -> Result<String, ApiError> {
    let response = match builder.execute().await {
        Ok(r) => r,
        Err(_) => return Err(ApiError::new_internal("database request failed")),
    };
    let text = match response.text().await {
        Ok(t) => t,
        Err(_) => {
            return Err(ApiError::new_internal(
                "extracting text from response failed",
            ))
        }
    };
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::AsnInfo;

    #[tokio::test]
    async fn test_connection() {
        let db = BgpkitDatabase::new();
        let data = db
            .client
            .from("asn_view")
            .select("*")
            .limit(10)
            .execute()
            .await
            .unwrap();
        let objects: Vec<AsnInfo> =
            serde_json::from_str(data.text().await.unwrap().as_str()).unwrap();
        dbg!(objects);
    }
}
