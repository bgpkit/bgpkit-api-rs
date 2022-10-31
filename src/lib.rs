use std::sync::Arc;
use axum::{Extension, Router, routing};
use axum::http::StatusCode;
use tracing::info;
use crate::api::{search_asninfo, search_roas};
use crate::db::BgpkitDatabase;

use utoipa::{Modify, OpenApi};
use utoipa::openapi::{ContactBuilder, LicenseBuilder};
use utoipa_swagger_ui::SwaggerUi;

pub mod api;
pub mod db;

async fn health_check() -> StatusCode {
    return StatusCode::OK
}


pub async fn start_service() {

    #[derive(OpenApi)]
    #[openapi(
        paths(
            api::search_asninfo,
            api::search_roas,
        ),
    components(
        schemas(api::AsnInfo, api::AsninfoResponse),
        schemas(api::RoasEntry, api::RoasResponse)
    ),
    modifiers( &Intro ),
    tags(
        (name = "meta", description = "Meta information for Internet entities"),
        (name = "bgp", description = "BGP data")
    )
    )]
    struct ApiDoc;

    struct Intro;

    impl Modify for Intro {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            openapi.info.license = Some(LicenseBuilder::new().name("BGPKIT Public Dataset License").url(Some("https://bgpkit.com/aua")).build());
            openapi.info.title = "BGPKIT Data API".to_string();
            openapi.info.contact = Some(
                ContactBuilder::new()
                    .name(Some("About BGPKIT"))
                    .url(Some("https://bgpkit.com/about"))
                    .build());
        }
    }


    let db = Arc::new(BgpkitDatabase::new());
    let app = Router::new()
        .merge(SwaggerUi::new("/docs/*tail").url("/openapi.json", ApiDoc::openapi()))
        .route("/asninfo", routing::get(search_asninfo))
        .route("/roas", routing::get(search_roas))
        .route("/health_check", routing::get(health_check))
        .layer(Extension(db));

    dotenvy::dotenv().ok();
    let port_str = std::env::var("BGPKIT_API_PORT").unwrap_or("3000".to_string());
    let addr_str = format!("[::]:{}", port_str);
    let addr = addr_str.parse::<std::net::SocketAddr>().unwrap();

    info!("start listening to address {}", addr.to_string());
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
