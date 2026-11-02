use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../web/build/"]
struct Assets;

pub async fn serve(req: Request<Body>) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    if path == "api" || path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let key = if path.is_empty() { "index.html" } else { path };
    let asset = Assets::get(key).or_else(|| {
        if path
            .rsplit('/')
            .next()
            .is_some_and(|part| !part.contains('.'))
        {
            Assets::get("index.html")
        } else {
            None
        }
    });
    match asset {
        Some(asset) => {
            let mime = if Assets::get(key).is_some() {
                mime_guess::from_path(key).first_or_octet_stream()
            } else {
                mime_guess::mime::TEXT_HTML
            };
            (
                [(header::CONTENT_TYPE, mime.as_ref())],
                asset.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
