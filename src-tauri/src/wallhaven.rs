use crate::downloads::ByteDownloadProgress;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};

const WALLHAVEN_API_BASE: &str = "https://wallhaven.cc/api/v1";
const USER_AGENT: &str =
    "Wallscape/0.1 (Windows desktop app; user-initiated Wallhaven integration)";
const MAX_DOWNLOAD_BYTES: u64 = 80 * 1024 * 1024;
const WALLHAVEN_MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(1_350);
pub const WALLHAVEN_SOURCE_NAME: &str = "wallhaven";

static WALLHAVEN_NEXT_REQUEST_AT: OnceLock<Mutex<Instant>> = OnceLock::new();

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallhavenSearchRequest {
    pub query: Option<String>,
    pub categories: Option<String>,
    pub purity: Option<String>,
    pub sorting: Option<String>,
    pub order: Option<String>,
    pub atleast: Option<String>,
    pub ratios: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub struct WallhavenSearchResponse {
    pub data: Vec<WallhavenWallpaper>,
    pub meta: WallhavenMeta,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub struct WallhavenWallpaper {
    pub id: String,
    pub url: String,
    pub short_url: String,
    pub views: i64,
    pub favorites: i64,
    pub purity: String,
    pub category: String,
    pub dimension_x: i32,
    pub dimension_y: i32,
    pub resolution: String,
    pub ratio: String,
    pub file_size: i64,
    pub file_type: String,
    pub created_at: String,
    pub colors: Vec<String>,
    pub path: String,
    pub thumbs: WallhavenThumbs,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub struct WallhavenThumbs {
    pub large: String,
    pub original: String,
    pub small: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub struct WallhavenMeta {
    pub current_page: u32,
    pub last_page: u32,
    pub per_page: u32,
    pub total: u32,
    pub seed: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiSearchResponse {
    data: Vec<WallhavenWallpaper>,
    meta: ApiMeta,
}

#[derive(Debug, Deserialize)]
struct ApiMeta {
    current_page: u32,
    last_page: u32,
    per_page: u32,
    total: u32,
    seed: Option<String>,
}

#[derive(Debug)]
pub struct DownloadedImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}

pub struct WallhavenClient {
    http: reqwest::Client,
}

impl WallhavenClient {
    pub fn new() -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| format!("Failed to create Wallhaven client: {}", e))?;

        Ok(Self { http })
    }

    pub async fn search(
        &self,
        request: WallhavenSearchRequest,
    ) -> Result<WallhavenSearchResponse, String> {
        wait_for_wallhaven_request_slot().await;

        let mut url = reqwest::Url::parse(&format!("{WALLHAVEN_API_BASE}/search"))
            .map_err(|e| format!("Failed to build Wallhaven URL: {}", e))?;
        let search_term = clean_query_value(request.query);
        let sorting = {
            let sorting = sanitize_sorting(request.sorting);
            if sorting == "relevance" && search_term.is_none() {
                "date_added".to_string()
            } else {
                sorting
            }
        };

        {
            let mut params = url.query_pairs_mut();
            params.append_pair(
                "categories",
                sanitize_categories(request.categories).as_str(),
            );
            params.append_pair("purity", sanitize_purity(request.purity).as_str());
            params.append_pair("sorting", &sorting);
            params.append_pair("order", sanitize_order(request.order).as_str());
            params.append_pair("page", &request.page.unwrap_or(1).clamp(1, 100).to_string());

            if let Some(value) = search_term {
                params.append_pair("q", &value);
            }

            if let Some(value) = clean_resolution(request.atleast) {
                params.append_pair("atleast", &value);
            }

            if let Some(value) = clean_ratios(request.ratios) {
                params.append_pair("ratios", &value);
            }
        }

        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Wallhaven search failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Wallhaven search returned HTTP {}",
                response.status()
            ));
        }

        let payload = response
            .json::<ApiSearchResponse>()
            .await
            .map_err(|e| format!("Failed to read Wallhaven search response: {}", e))?;

        Ok(WallhavenSearchResponse {
            data: payload.data,
            meta: WallhavenMeta {
                current_page: payload.meta.current_page,
                last_page: payload.meta.last_page,
                per_page: payload.meta.per_page,
                total: payload.meta.total,
                seed: payload.meta.seed,
            },
        })
    }

    pub async fn download_image_with_progress<F>(
        &self,
        url: &str,
        mut on_progress: F,
    ) -> Result<DownloadedImage, String>
    where
        F: FnMut(ByteDownloadProgress),
    {
        if !is_wallhaven_image_url(url) {
            return Err("Only Wallhaven image URLs can be downloaded".to_string());
        }

        wait_for_wallhaven_request_slot().await;

        let mut response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Wallhaven download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Wallhaven download returned HTTP {}",
                response.status()
            ));
        }

        let total_bytes = response.content_length();

        if let Some(length) = total_bytes {
            if length > MAX_DOWNLOAD_BYTES {
                return Err("Wallhaven image is too large to download".to_string());
            }
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let extension = extension_from_content_type(&content_type)
            .or_else(|| extension_from_url(url))
            .ok_or_else(|| "Wallhaven returned an unsupported image format".to_string())?;

        let mut received_bytes = 0_u64;
        let mut bytes =
            Vec::with_capacity(total_bytes.unwrap_or(0).min(MAX_DOWNLOAD_BYTES) as usize);
        on_progress(ByteDownloadProgress {
            received_bytes,
            total_bytes,
        });

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Failed to read Wallhaven image: {}", e))?
        {
            received_bytes += chunk.len() as u64;

            if received_bytes > MAX_DOWNLOAD_BYTES {
                return Err("Wallhaven image is too large to download".to_string());
            }

            bytes.extend_from_slice(&chunk);
            on_progress(ByteDownloadProgress {
                received_bytes,
                total_bytes,
            });
        }

        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err("Wallhaven image is too large to download".to_string());
        }

        Ok(DownloadedImage { bytes, extension })
    }
}

async fn wait_for_wallhaven_request_slot() {
    let limiter = WALLHAVEN_NEXT_REQUEST_AT.get_or_init(|| Mutex::new(Instant::now()));
    let mut next_request_at = limiter.lock().await;
    let now = Instant::now();

    if *next_request_at > now {
        sleep(*next_request_at - now).await;
    }

    *next_request_at = Instant::now() + WALLHAVEN_MIN_REQUEST_INTERVAL;
}

fn sanitize_categories(value: Option<String>) -> String {
    match value.as_deref() {
        Some("100") | Some("010") | Some("001") | Some("110") | Some("101") | Some("011")
        | Some("111") => value.unwrap_or_default(),
        _ => "111".to_string(),
    }
}

fn sanitize_purity(value: Option<String>) -> String {
    match value.as_deref() {
        Some("100") | Some("010") | Some("110") => value.unwrap_or_default(),
        _ => "100".to_string(),
    }
}

fn sanitize_sorting(value: Option<String>) -> String {
    match value.as_deref() {
        Some("relevance") | Some("random") | Some("date_added") | Some("views")
        | Some("favorites") | Some("toplist") | Some("hot") => value.unwrap_or_default(),
        _ => "date_added".to_string(),
    }
}

fn sanitize_order(value: Option<String>) -> String {
    match value.as_deref() {
        Some("asc") => "asc".to_string(),
        _ => "desc".to_string(),
    }
}

fn clean_query_value(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn clean_resolution(value: Option<String>) -> Option<String> {
    let value = clean_query_value(value)?;
    let (width, height) = value.split_once('x')?;
    if valid_dimension(width) && valid_dimension(height) {
        Some(value)
    } else {
        None
    }
}

fn clean_ratios(value: Option<String>) -> Option<String> {
    let value = clean_query_value(value)?;
    if value
        .split(',')
        .all(|ratio| matches!(ratio, "16x9" | "16x10" | "21x9" | "32x9" | "48x9"))
    {
        Some(value)
    } else {
        None
    }
}

fn valid_dimension(value: &str) -> bool {
    value
        .parse::<u32>()
        .map(|n| (1..=16_384).contains(&n))
        .unwrap_or(false)
}

fn is_wallhaven_image_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| {
            let host = url.host_str()?.to_ascii_lowercase();
            Some(url.scheme() == "https" && host.ends_with("wallhaven.cc"))
        })
        .unwrap_or(false)
}

fn extension_from_content_type(content_type: &str) -> Option<&'static str> {
    if content_type.contains("image/jpeg") {
        Some("jpg")
    } else if content_type.contains("image/png") {
        Some("png")
    } else {
        None
    }
}

fn extension_from_url(url: &str) -> Option<&'static str> {
    let path = reqwest::Url::parse(url).ok()?.path().to_ascii_lowercase();
    if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        Some("jpg")
    } else if path.ends_with(".png") {
        Some("png")
    } else {
        None
    }
}
