use crate::Cluster;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Serialize;

pub fn app(cluster: Cluster) -> Router {
    Router::new()
        .route(
            "/cache/:key",
            put(put_cache).get(get_cache).delete(delete_cache),
        )
        .route("/stats", get(stats))
        .route("/health", get(health))
        .with_state(cluster)
}

async fn put_cache(
    State(cluster): State<Cluster>,
    Path(key): Path<String>,
    body: Bytes,
) -> Response {
    if let Some(response) = redirect_if_not_local_owner(&cluster, &key) {
        return response;
    }

    let mut cache = cluster.local_cache.write().await;
    cache.put(key, body.to_vec());
    StatusCode::NO_CONTENT.into_response()
}

async fn get_cache(State(cluster): State<Cluster>, Path(key): Path<String>) -> Response {
    if let Some(response) = redirect_if_not_local_owner(&cluster, &key) {
        return response;
    }

    let mut cache = cluster.local_cache.write().await;
    match cache.get(&key) {
        Some(value) => (StatusCode::OK, value).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn delete_cache(State(cluster): State<Cluster>, Path(key): Path<String>) -> Response {
    if let Some(response) = redirect_if_not_local_owner(&cluster, &key) {
        return response;
    }

    let mut cache = cluster.local_cache.write().await;
    if cache.remove(&key) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn stats(State(cluster): State<Cluster>) -> Json<StatsResponse> {
    let cache = cluster.local_cache.read().await;
    let stats = cache.stats();

    Json(StatsResponse {
        len: cache.len(),
        hits: stats.hits,
        misses: stats.misses,
        puts: stats.puts,
        updates: stats.updates,
        removes: stats.removes,
        evictions: stats.evictions,
        expirations: stats.expirations,
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

fn redirect_if_not_local_owner(cluster: &Cluster, key: &str) -> Option<Response> {
    if cluster.is_local_owner(key) {
        return None;
    }

    let owner = cluster.primary_owner(key)?;
    let location = format!("{}/cache/{}", owner.base_url.trim_end_matches('/'), key);
    let mut headers = HeaderMap::new();
    let Ok(location) = HeaderValue::from_str(&location) else {
        return Some(StatusCode::BAD_GATEWAY.into_response());
    };
    headers.insert(axum::http::header::LOCATION, location);
    Some((StatusCode::TEMPORARY_REDIRECT, headers).into_response())
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    len: usize,
    hits: u64,
    misses: u64,
    puts: u64,
    updates: u64,
    removes: u64,
    evictions: u64,
    expirations: u64,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}
