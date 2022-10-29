use std::sync::Arc;
use axum::{Extension, Router, routing};
use axum::http::StatusCode;
use tracing::info;
use crate::api::search_asninfo;
use crate::db::BgpkitDatabase;

use utoipa::{
    OpenApi,
};
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
        ),
    components(
        schemas(api::AsnInfo, api::AsninfoResponse)
    ),
    modifiers(),
    tags(
        (name = "meta", description = "Meta information for Internet entities")
    )
    )]
    struct ApiDoc;


    let db = Arc::new(BgpkitDatabase::new());
    let app = Router::new()
        .merge(SwaggerUi::new("/docs/*tail").url("/openapi.json", ApiDoc::openapi()))
        .route("/asninfo", routing::get(search_asninfo))
        .route("/health_check", routing::get(health_check))
        .layer(Extension(db))
        ;

    let addr = "[::]:3000".parse::<std::net::SocketAddr>().unwrap();

    info!("start listening to address {}", addr.to_string());
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
