mod routes;
mod scraping;
mod structs;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};

#[tokio::main]
async fn main() {
    // Configuración CORS para permitir cualquier origen
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::create_routes()
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3030));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("🚀 Servidor API iniciado en http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}