use backend::{AppState, app};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8000".into());
    let origins = std::env::var("CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .expect("bind");
    tracing::info!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app(AppState::with_fixtures(), origins))
        .with_graceful_shutdown(shutdown())
        .await
        .unwrap();
}

async fn shutdown() {
    let mut term =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}
