mod api;
mod assets;
mod config;
mod error;
mod ffmpeg;
mod jobs;
mod song;
mod source;
mod state;
mod types;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = config::Config::from_env();
    ffmpeg::check_ffmpeg()?;
    let bind_addr = config.bind_addr.clone();
    let state = state::AppState::new(config)?;
    let app = api::router(state).fallback(assets::serve);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
