#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    bgpkit_api_rs::start_service().await;
}
