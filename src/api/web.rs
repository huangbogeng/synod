use axum::{
    body::Body,
    http::{Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};

const INDEX: &str = include_str!("../../web/dist/index.html");
const JAVASCRIPT: &[u8] = include_bytes!("../../web/dist/assets/app.js");
const STYLESHEET: &[u8] = include_bytes!("../../web/dist/assets/index.css");

pub async fn index() -> Response {
    index_response()
}

pub async fn javascript() -> Response {
    asset("text/javascript; charset=utf-8", JAVASCRIPT)
}

pub async fn stylesheet() -> Response {
    asset("text/css; charset=utf-8", STYLESHEET)
}

pub async fn fallback(method: Method, uri: Uri) -> Response {
    if method != Method::GET {
        StatusCode::METHOD_NOT_ALLOWED.into_response()
    } else if uri.path().starts_with("/api/") {
        StatusCode::NOT_FOUND.into_response()
    } else {
        index_response()
    }
}

fn index_response() -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; connect-src 'self'; img-src 'self' data:; \
             script-src 'self'; style-src 'self'; object-src 'none'; \
             base-uri 'none'; frame-ancestors 'none'",
        )
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(INDEX))
        .expect("static index response must be valid")
}

fn asset(content_type: &'static str, content: &'static [u8]) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(content))
        .expect("static asset response must be valid")
}
