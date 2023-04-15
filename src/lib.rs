use crate::api::{search_asninfo, search_broker, search_roas, search_peer_stats};
use crate::db::BgpkitDatabase;
use axum::http::{Method, StatusCode};
use axum::{routing, Extension, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use utoipa::openapi::{ContactBuilder, LicenseBuilder};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

pub mod api;
pub mod db;

async fn health_check() -> StatusCode {
    return StatusCode::OK;
}

pub async fn start_service() {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            api::search_asninfo,
            api::search_roas,
            api::search_broker,
            api::search_peer_stats,
        ),
    components(
        schemas(api::AsnInfo, api::AsninfoResponse),
        schemas(api::BrokerEntry, api::BrokerResponse),
        schemas(api::RoasEntry, api::RoasResponse),
        schemas(api::PeerStats, api::PeerStatsResponse)
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
            openapi.info.license = Some(
                LicenseBuilder::new()
                    .name("BGPKIT Public Dataset License")
                    .url(Some("https://bgpkit.com/aua"))
                    .build(),
            );
            openapi.info.title = "BGPKIT Data API".to_string();
            openapi.info.contact = Some(
                ContactBuilder::new()
                    .name(Some("About BGPKIT"))
                    .url(Some("https://bgpkit.com/about"))
                    .build(),
            );
        }
    }

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    let db = Arc::new(BgpkitDatabase::new());
    let app = Router::new()
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .route("/asninfo", routing::get(search_asninfo))
        .route("/roas", routing::get(search_roas))
        .route("/broker", routing::get(search_broker))
        .route("/peers", routing::get(search_peer_stats))
        .route("/health_check", routing::get(health_check))
        .layer(Extension(db))
        .layer(cors);

    dotenvy::dotenv().ok();
    let port_str = std::env::var("BGPKIT_API_PORT").unwrap_or("3000".to_string());
    let addr_str = format!("0.0.0.0:{}", port_str);
    let addr = addr_str.parse::<std::net::SocketAddr>().unwrap();

    info!("start listening to address http://{}", addr.to_string());
    info!("docs available at http://{}/docs", addr.to_string());
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
