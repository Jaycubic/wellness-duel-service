mod activities;
mod db;
mod error;
mod feedback;
mod models;
mod rooms;
mod state;
mod util;
mod ws;

use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use state::AppState;

async fn health(state: web::Data<AppState>) -> HttpResponse {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })),
        Err(e) => {
            tracing::error!(error = %e, "health check DB ping failed");
            HttpResponse::ServiceUnavailable().json(serde_json::json!({ "status": "db_error" }))
        }
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let pool = db::create_pool().await?;
    db::run_migrations(&pool).await?;
    tracing::info!("migrations applied");

    let uploads_dir = std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "./uploads".to_string());
    std::fs::create_dir_all(&uploads_dir)?;

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8090);

    let app_state = web::Data::new(AppState::new(pool, uploads_dir.clone()));

    tracing::info!(bind_addr = %bind_addr, port, "wellness-duel-service starting");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/health", web::get().to(health))
            .route("/api/rooms", web::post().to(rooms::create_room))
            .route("/api/rooms/{code}/join", web::post().to(rooms::join_room))
            .route("/api/rooms/{code}/state", web::get().to(rooms::get_state))
            .route("/api/rooms/{code}/checkin", web::post().to(rooms::submit_checkin))
            .route("/api/feedback", web::get().to(feedback::list_feedback))
            .route("/api/feedback", web::post().to(feedback::submit_feedback))
            .route("/api/recover", web::post().to(rooms::recover_player))
            .route("/ws/{code}", web::get().to(ws::ws_route))
            .service(Files::new("/uploads", &uploads_dir))
            // Serves the game itself (public/index.html) from the same
            // origin as the API, so the browser needs no CORS setup at all.
            // Registered last so it never shadows the routes above.
            .service(Files::new("/", "./public").index_file("index.html"))
    })
    .bind((bind_addr.as_str(), port))?
    .run()
    .await?;

    Ok(())
}
