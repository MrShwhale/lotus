use axum::{routing::get, Router};
use lotus_web::{server, SERVER_HEADING};

#[tokio::main]
async fn main() {
    eprintln!("{}Starting web server...", SERVER_HEADING);

    let app = Router::new()
        .route("/", get(server::root))
        .route("/rec", get(server::get_rec));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Error starting listener");
    eprintln!("{}Web server up!", SERVER_HEADING);

    // Wait until after the listener is actually up to set up the recommender
    lazy_static::initialize(&server::RECOMMENDER);
    axum::serve(listener, app).await.expect("Error while serving pages");
}
