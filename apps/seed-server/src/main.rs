//! Minimal single-seed server for Modrinthium.
//!
//! One running instance hosts exactly one "seed" (one modpack line): it serves
//! a manifest at `/manifest.json` — the URL users paste into the launcher — and
//! the `.mrpack` files it points at, and it accepts authenticated uploads at
//! `POST /publish` to release a new version.
//!
//! Configuration is entirely through environment variables:
//!
//! | Variable          | Required | Default          | Purpose                                             |
//! | ----------------- | -------- | ---------------- | --------------------------------------------------- |
//! | `SEED_KEY`        | yes      | —                | Secret bearer token required to publish.            |
//! | `SEED_PUBLIC_URL` | yes      | —                | Public base URL, used to build absolute file links. |
//! | `SEED_DATA_DIR`   | no       | `./data`         | Where the manifest and files are stored.            |
//! | `SEED_BIND`       | no       | `0.0.0.0:8000`   | Address to listen on.                               |
//! | `SEED_NAME`       | no       | `Seed`           | Fallback pack name before anything is published.    |

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Multipart, Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};

struct Config {
    key: String,
    data_dir: PathBuf,
    public_url: String,
    default_name: String,
}

impl Config {
    fn files_dir(&self) -> PathBuf {
        self.data_dir.join("files")
    }

    fn manifest_path(&self) -> PathBuf {
        self.data_dir.join("manifest.json")
    }

    fn file_url(&self, name: &str) -> String {
        format!("{}/files/{}", self.public_url, name)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct SeedServer {
    address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Manifest {
    name: String,
    version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    changelog: Option<String>,
    mrpack: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon: Option<String>,
    #[serde(default)]
    servers: Vec<SeedServer>,
}

struct AppError(StatusCode, String);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

fn bad_request(msg: impl Into<String>) -> AppError {
    AppError(StatusCode::BAD_REQUEST, msg.into())
}

#[tokio::main]
async fn main() {
    let key = require_env("SEED_KEY");
    let public_url = require_env("SEED_PUBLIC_URL")
        .trim_end_matches('/')
        .to_string();
    let data_dir = PathBuf::from(
        std::env::var("SEED_DATA_DIR").unwrap_or_else(|_| "./data".to_string()),
    );
    let bind = std::env::var("SEED_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8000".to_string());
    let default_name =
        std::env::var("SEED_NAME").unwrap_or_else(|_| "Seed".to_string());

    let config = Arc::new(Config {
        key,
        data_dir,
        public_url,
        default_name,
    });

    if let Err(err) = tokio::fs::create_dir_all(config.files_dir()).await {
        eprintln!("Failed to create data directory: {err}");
        std::process::exit(1);
    }

    let app = Router::new()
        .route("/", get(health))
        .route("/manifest.json", get(get_manifest))
        .route("/files/{name}", get(get_file))
        .route(
            "/publish",
            post(publish).layer(DefaultBodyLimit::disable()),
        )
        .with_state(config.clone());

    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("Failed to bind {bind}: {err}");
            std::process::exit(1);
        }
    };

    println!(
        "seed-server listening on {bind} (public: {})",
        config.public_url
    );
    if let Err(err) = axum::serve(listener, app).await {
        eprintln!("Server error: {err}");
        std::process::exit(1);
    }
}

fn require_env(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => value,
        _ => {
            eprintln!("Missing required environment variable {name}");
            std::process::exit(1);
        }
    }
}

async fn health() -> &'static str {
    "seed-server: ok. Manifest at /manifest.json"
}

async fn get_manifest(
    State(config): State<Arc<Config>>,
) -> Result<Response, AppError> {
    let bytes = tokio::fs::read(config.manifest_path()).await.map_err(|_| {
        AppError(
            StatusCode::NOT_FOUND,
            "No version has been published yet".to_string(),
        )
    })?;
    Ok((
        [(header::CONTENT_TYPE, "application/json")],
        bytes,
    )
        .into_response())
}

async fn get_file(
    State(config): State<Arc<Config>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Response, AppError> {
    if !is_safe_filename(&name) {
        return Err(bad_request("Invalid file name"));
    }
    let path = config.files_dir().join(&name);
    let bytes = tokio::fs::read(&path).await.map_err(|_| {
        AppError(StatusCode::NOT_FOUND, "File not found".to_string())
    })?;
    Ok((
        [(header::CONTENT_TYPE, content_type_for(&name))],
        bytes,
    )
        .into_response())
}

async fn publish(
    State(config): State<Arc<Config>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    if !authorized(&config, &headers) {
        return Err(AppError(
            StatusCode::UNAUTHORIZED,
            "Invalid or missing seed key".to_string(),
        ));
    }

    let mut mrpack: Option<Bytes> = None;
    let mut version: Option<String> = None;
    let mut name: Option<String> = None;
    let mut changelog: Option<String> = None;
    let mut servers: Vec<SeedServer> = Vec::new();
    let mut icon: Option<(Bytes, String)> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| bad_request(format!("Malformed upload: {err}")))?
    {
        match field.name().unwrap_or_default() {
            "mrpack" => {
                mrpack = Some(read_bytes(field).await?);
            }
            "icon" => {
                let ext = field
                    .file_name()
                    .and_then(|f| f.rsplit('.').next())
                    .filter(|ext| is_safe_ext(ext))
                    .unwrap_or("png")
                    .to_lowercase();
                icon = Some((read_bytes(field).await?, ext));
            }
            "version" => version = Some(read_text(field).await?),
            "name" => name = Some(read_text(field).await?),
            "changelog" => changelog = Some(read_text(field).await?),
            "servers" => {
                let raw = read_text(field).await?;
                if !raw.trim().is_empty() {
                    servers = serde_json::from_str(&raw).map_err(|err| {
                        bad_request(format!("Invalid servers JSON: {err}"))
                    })?;
                }
            }
            _ => {
                // Ignore unknown fields so the protocol can grow.
                let _ = read_bytes(field).await;
            }
        }
    }

    let mrpack = mrpack.ok_or_else(|| bad_request("Missing mrpack file"))?;
    let version = version
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad_request("Missing version"))?;

    let existing = read_manifest(&config).await;
    let resolved_name = name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| existing.as_ref().map(|m| m.name.clone()))
        .unwrap_or_else(|| config.default_name.clone());
    let changelog = changelog
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());

    let mrpack_name = format!("{}.mrpack", sanitize_filename(&version));
    let hash = sha1_smol::Sha1::from(mrpack.as_ref()).digest().to_string();

    tokio::fs::write(config.files_dir().join(&mrpack_name), &mrpack)
        .await
        .map_err(|err| {
            AppError(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to store mrpack: {err}"),
            )
        })?;

    let icon_url = match icon {
        Some((bytes, ext)) => {
            let icon_name = format!("icon.{ext}");
            tokio::fs::write(config.files_dir().join(&icon_name), &bytes)
                .await
                .map_err(|err| {
                    AppError(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to store icon: {err}"),
                    )
                })?;
            Some(config.file_url(&icon_name))
        }
        None => existing.and_then(|m| m.icon),
    };

    let manifest = Manifest {
        name: resolved_name,
        version,
        changelog,
        mrpack: config.file_url(&mrpack_name),
        hash: Some(format!("sha1:{hash}")),
        icon: icon_url,
        servers,
    };

    let serialized = serde_json::to_vec_pretty(&manifest).map_err(|err| {
        AppError(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize manifest: {err}"),
        )
    })?;
    tokio::fs::write(config.manifest_path(), &serialized)
        .await
        .map_err(|err| {
            AppError(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to write manifest: {err}"),
            )
        })?;

    println!(
        "Published {} v{} ({} bytes)",
        manifest.name,
        manifest.version,
        mrpack.len()
    );

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serialized,
    )
        .into_response())
}

async fn read_manifest(config: &Config) -> Option<Manifest> {
    let bytes = tokio::fs::read(config.manifest_path()).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn authorized(config: &Config, headers: &HeaderMap) -> bool {
    let Some(value) = headers.get(header::AUTHORIZATION) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    value
        .strip_prefix("Bearer ")
        .is_some_and(|token| token == config.key)
}

async fn read_bytes(
    field: axum::extract::multipart::Field<'_>,
) -> Result<Bytes, AppError> {
    field
        .bytes()
        .await
        .map_err(|err| bad_request(format!("Failed to read field: {err}")))
}

async fn read_text(
    field: axum::extract::multipart::Field<'_>,
) -> Result<String, AppError> {
    field
        .text()
        .await
        .map_err(|err| bad_request(format!("Failed to read field: {err}")))
}

fn is_safe_filename(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && !name.contains("..")
}

fn is_safe_ext(ext: &str) -> bool {
    !ext.is_empty()
        && ext.len() <= 8
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

fn sanitize_filename(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    if cleaned.is_empty() {
        "version".to_string()
    } else {
        cleaned
    }
}

fn content_type_for(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_lowercase).as_deref() {
        Some("mrpack" | "zip") => "application/zip",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}
