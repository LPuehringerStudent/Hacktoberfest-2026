use backend::{AppState, app, db::EventLog};
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

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "hacktober.db".into());
    let mut state = AppState::with_fixtures();
    match EventLog::open(std::path::Path::new(&db_path)) {
        Ok(log) => {
            tracing::info!(%db_path, "event log open");
            state = state.with_event_log(log);
        }
        Err(e) => tracing::error!(%db_path, error = %e, "event log disabled"),
    }

    let router = app(state, origins);
    if std::env::var("SERVER").as_deref() == Ok("uring") {
        let addr = format!("{host}:{port}").parse().expect("HOST:PORT");
        let workers = std::env::var("URING_WORKERS")
            .ok()
            .and_then(|w| w.parse().ok())
            .unwrap_or(4);
        backend::uring::serve(router, addr, workers, tokio::runtime::Handle::current())
            .expect("io_uring server");
        shutdown().await;
        return;
    }

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .expect("bind");
    tracing::info!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, router)
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
