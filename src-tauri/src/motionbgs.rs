use crate::downloads::ByteDownloadProgress;
use reqwest::header::CONTENT_TYPE;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};

const MOTIONBGS_BASE: &str = "https://motionbgs.com";
const USER_AGENT: &str =
    "Wallscape/0.1 (Windows desktop app; user-initiated MotionBGS integration)";
const MAX_DOWNLOAD_BYTES: u64 = 300 * 1024 * 1024;
const MOTIONBGS_MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(1_500);
const MOTIONBGS_PAGE_TIMEOUT: Duration = Duration::from_secs(30);
const MOTIONBGS_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MOTIONBGS_READ_TIMEOUT: Duration = Duration::from_secs(45);
pub const MOTIONBGS_SOURCE_NAME: &str = "motionbgs";

static MOTIONBGS_NEXT_REQUEST_AT: OnceLock<Mutex<Instant>> = OnceLock::new();

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionBgsSearchRequest {
    pub query: Option<String>,
    pub category: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MotionBgsSearchResponse {
    pub data: Vec<MotionBgsWallpaper>,
    pub meta: MotionBgsMeta,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MotionBgsWallpaper {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub url: String,
    pub thumbnail_url: String,
    pub preview_video_url: Option<String>,
    pub quality: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub tags: Vec<String>,
    pub downloads: Vec<MotionBgsDownloadOption>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MotionBgsDownloadOption {
    pub quality: String,
    pub url: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct MotionBgsMeta {
    pub current_page: u32,
    pub has_next_page: bool,
}

#[derive(Debug)]
pub struct DownloadedMotionBgsVideo {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
    pub wallpaper: MotionBgsWallpaper,
    pub download: MotionBgsDownloadOption,
}

pub struct MotionBgsClient {
    http: reqwest::Client,
}

impl MotionBgsClient {
    pub fn new() -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(MOTIONBGS_CONNECT_TIMEOUT)
            .read_timeout(MOTIONBGS_READ_TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create MotionBGS client: {}", e))?;

        Ok(Self { http })
    }

    pub async fn search(
        &self,
        request: MotionBgsSearchRequest,
    ) -> Result<MotionBgsSearchResponse, String> {
        wait_for_motionbgs_request_slot().await;

        let page = request.page.unwrap_or(1).clamp(1, 100);
        let is_feed_request = request
            .query
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty();
        let url = search_url(request.query, request.category, page)?;
        let response = self
            .http
            .get(url)
            .timeout(MOTIONBGS_PAGE_TIMEOUT)
            .send()
            .await
            .map_err(|e| format!("MotionBGS search failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "MotionBGS search returned HTTP {}",
                response.status()
            ));
        }

        let html = response
            .text()
            .await
            .map_err(|e| format!("Failed to read MotionBGS search response: {}", e))?;

        let document = Html::parse_document(&html);
        let data = parse_listing_wallpapers(&document);
        let has_next_page = has_next_page(&document) || (page == 1 && is_feed_request);

        Ok(MotionBgsSearchResponse {
            meta: MotionBgsMeta {
                current_page: page,
                has_next_page: has_next_page && !data.is_empty(),
            },
            data,
        })
    }

    pub async fn detail(
        &self,
        wallpaper: &MotionBgsWallpaper,
    ) -> Result<MotionBgsWallpaper, String> {
        wait_for_motionbgs_request_slot().await;

        let url = validate_motionbgs_page_url(&wallpaper.url)?;
        let response = self
            .http
            .get(url)
            .timeout(MOTIONBGS_PAGE_TIMEOUT)
            .send()
            .await
            .map_err(|e| format!("MotionBGS detail request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "MotionBGS detail returned HTTP {}",
                response.status()
            ));
        }

        let html = response
            .text()
            .await
            .map_err(|e| format!("Failed to read MotionBGS detail response: {}", e))?;
        let document = Html::parse_document(&html);
        parse_detail_wallpaper(&document, wallpaper)
    }

    pub async fn download_video_with_progress<F>(
        &self,
        wallpaper: &MotionBgsWallpaper,
        mut on_progress: F,
    ) -> Result<DownloadedMotionBgsVideo, String>
    where
        F: FnMut(ByteDownloadProgress),
    {
        let detail = self.detail(wallpaper).await?;
        let download = preferred_download(&detail)
            .ok_or_else(|| "MotionBGS detail did not expose a downloadable MP4".to_string())?;
        let url = validate_motionbgs_download_url(&download.url)?;

        wait_for_motionbgs_request_slot().await;

        let mut response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| format!("MotionBGS download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "MotionBGS download returned HTTP {}",
                response.status()
            ));
        }

        let total_bytes = response.content_length();

        if let Some(length) = total_bytes {
            if length > MAX_DOWNLOAD_BYTES {
                return Err("MotionBGS video is too large to download".to_string());
            }
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !content_type.contains("video/mp4") {
            return Err("MotionBGS returned an unsupported video format".to_string());
        }

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
            .map_err(|e| format!("Failed to read MotionBGS video: {}", e))?
        {
            received_bytes += chunk.len() as u64;

            if received_bytes > MAX_DOWNLOAD_BYTES {
                return Err("MotionBGS video is too large to download".to_string());
            }

            bytes.extend_from_slice(&chunk);
            on_progress(ByteDownloadProgress {
                received_bytes,
                total_bytes,
            });
        }

        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err("MotionBGS video is too large to download".to_string());
        }

        if let Some(total_bytes) = total_bytes {
            if bytes.len() as u64 != total_bytes {
                return Err(format!(
                    "MotionBGS video download ended early: received {} of {} bytes",
                    bytes.len(),
                    total_bytes
                ));
            }
        }

        Ok(DownloadedMotionBgsVideo {
            bytes,
            extension: "mp4",
            wallpaper: detail,
            download,
        })
    }
}

async fn wait_for_motionbgs_request_slot() {
    let limiter = MOTIONBGS_NEXT_REQUEST_AT.get_or_init(|| Mutex::new(Instant::now()));
    let mut next_request_at = limiter.lock().await;
    let now = Instant::now();

    if *next_request_at > now {
        sleep(*next_request_at - now).await;
    }

    *next_request_at = Instant::now() + MOTIONBGS_MIN_REQUEST_INTERVAL;
}

fn search_url(
    query: Option<String>,
    category: Option<String>,
    page: u32,
) -> Result<reqwest::Url, String> {
    let base = reqwest::Url::parse(MOTIONBGS_BASE)
        .map_err(|e| format!("Failed to build MotionBGS URL: {}", e))?;

    let query = clean_query_value(query);
    if let Some(query) = query {
        let mut url = base
            .join("/search")
            .map_err(|e| format!("Failed to build MotionBGS search URL: {}", e))?;
        {
            let mut params = url.query_pairs_mut();
            params.append_pair("q", &query);
            if page > 1 {
                params.append_pair("page", &page.to_string());
            }
        }
        return Ok(url);
    }

    let path = motionbgs_feed_path(category.as_deref(), page);
    let url = base
        .join(&path)
        .map_err(|e| format!("Failed to build MotionBGS category URL: {}", e))?;
    Ok(url)
}

fn motionbgs_feed_path(category: Option<&str>, page: u32) -> String {
    let root = match category {
        Some("4k") => "/4k",
        Some("mobile") => "/mobile",
        Some("gifs") => "/gifs",
        _ => "",
    };

    if page <= 1 {
        return format!("{root}/");
    }

    format!("{root}/{page}/")
}

fn parse_listing_wallpapers(document: &Html) -> Vec<MotionBgsWallpaper> {
    let card_selector = selector("div.tmb > a");
    let title_selector = selector(".ttl");
    let frame_selector = selector(".frm");
    let image_selector = selector("img");

    document
        .select(&card_selector)
        .filter_map(|card| {
            let href = card.value().attr("href")?;
            let url = absolute_url(href)?;
            if !is_motionbgs_page_url(url.as_str()) {
                return None;
            }

            let slug = slug_from_page_url(url.as_str())?;
            let raw_thumbnail_url = card
                .select(&image_selector)
                .next()
                .and_then(|image| image.value().attr("src"))
                .and_then(absolute_url)?;
            let id = media_id_from_url(&raw_thumbnail_url).unwrap_or_else(|| slug.clone());
            let thumbnail_url =
                high_resolution_image_url(&raw_thumbnail_url).unwrap_or(raw_thumbnail_url);
            let title = card
                .select(&title_selector)
                .next()
                .map(|node| node.text().collect::<Vec<_>>().join(" "))
                .or_else(|| card.value().attr("title").map(str::to_string))
                .map(clean_title)
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| title_from_slug(&slug));
            let quality = card
                .select(&frame_selector)
                .next()
                .map(|node| node.text().collect::<Vec<_>>().join(" "))
                .map(|value| value.trim().to_ascii_uppercase())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "HD".to_string());
            let (width, height) = dimensions_for_quality(&quality);
            let downloads = inferred_listing_downloads(&id, &quality);
            let preview_video_url = inferred_preview_video_url(&id, &slug);

            Some(MotionBgsWallpaper {
                id,
                slug,
                title,
                url: url.to_string(),
                thumbnail_url,
                preview_video_url,
                quality,
                width,
                height,
                file_size: 0,
                tags: Vec::new(),
                downloads,
            })
        })
        .collect()
}

fn parse_detail_wallpaper(
    document: &Html,
    fallback: &MotionBgsWallpaper,
) -> Result<MotionBgsWallpaper, String> {
    let title = meta_content(document, "meta[property=\"og:title\"]")
        .map(clean_title)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.title.clone());
    let url = meta_content(document, "meta[property=\"og:url\"]")
        .and_then(|value| absolute_url(&value))
        .unwrap_or_else(|| fallback.url.clone());
    let slug = slug_from_page_url(&url).unwrap_or_else(|| fallback.slug.clone());
    let thumbnail_url = meta_content(document, "meta[property=\"og:image\"]")
        .and_then(|value| absolute_url(&value))
        .unwrap_or_else(|| fallback.thumbnail_url.clone());
    let preview_video_url = meta_content(document, "meta[property=\"og:video\"]")
        .and_then(|value| absolute_url(&value))
        .or_else(|| video_source_url(document))
        .or_else(|| fallback.preview_video_url.clone());
    let downloads = parse_downloads(document);
    let preferred_quality = fallback.quality.to_ascii_uppercase();
    let quality = downloads
        .iter()
        .find(|download| download.quality.eq_ignore_ascii_case(&preferred_quality))
        .or_else(|| downloads.iter().find(|download| download.quality == "4K"))
        .or_else(|| downloads.first())
        .map(|download| download.quality.clone())
        .unwrap_or_else(|| fallback.quality.clone());
    let (width, height, file_size) = downloads
        .iter()
        .find(|download| download.quality == quality)
        .map(|download| (download.width, download.height, download.file_size))
        .unwrap_or_else(|| (fallback.width, fallback.height, fallback.file_size));
    let id = downloads
        .first()
        .and_then(|download| id_from_download_url(&download.url))
        .or_else(|| media_id_from_url(&thumbnail_url))
        .unwrap_or_else(|| fallback.id.clone());

    Ok(MotionBgsWallpaper {
        id,
        slug,
        title,
        url,
        thumbnail_url,
        preview_video_url,
        quality,
        width,
        height,
        file_size,
        tags: parse_tags(document),
        downloads,
    })
}

fn parse_downloads(document: &Html) -> Vec<MotionBgsDownloadOption> {
    let selector = selector("section.dl a[href^=\"/dl/\"]");
    document
        .select(&selector)
        .filter_map(|link| {
            let href = link.value().attr("href")?;
            let url = absolute_url(href)?;
            let quality = download_quality_from_url(&url)?;
            let text = link.text().collect::<Vec<_>>().join(" ");
            let (fallback_width, fallback_height) = dimensions_for_quality(&quality);
            let (width, height) =
                parse_dimensions(&text).unwrap_or((fallback_width, fallback_height));
            let file_size = parse_size_mb(&text).unwrap_or(0);

            Some(MotionBgsDownloadOption {
                quality,
                url,
                width,
                height,
                file_size,
            })
        })
        .collect()
}

fn inferred_listing_downloads(id: &str, quality: &str) -> Vec<MotionBgsDownloadOption> {
    if !id.chars().all(|ch| ch.is_ascii_digit()) {
        return Vec::new();
    }

    let qualities = if quality.eq_ignore_ascii_case("4K") {
        vec!["4K", "HD"]
    } else {
        vec!["HD"]
    };

    qualities
        .into_iter()
        .filter_map(|quality| inferred_download_option(id, quality))
        .collect()
}

fn inferred_download_option(id: &str, quality: &str) -> Option<MotionBgsDownloadOption> {
    let (width, height) = dimensions_for_quality(quality);
    let route_quality = match quality {
        "4K" => "4k",
        "HD" => "hd",
        _ => return None,
    };

    Some(MotionBgsDownloadOption {
        quality: quality.to_string(),
        url: absolute_url(&format!("/dl/{route_quality}/{id}/"))?,
        width,
        height,
        file_size: 0,
    })
}

fn parse_tags(document: &Html) -> Vec<String> {
    let selector = selector("ul.subtags a span, nav.crumb a[href^=\"/tag:\"]");
    document
        .select(&selector)
        .map(|node| node.text().collect::<Vec<_>>().join(" "))
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .fold(Vec::new(), |mut tags, tag| {
            if !tags
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&tag))
            {
                tags.push(tag);
            }
            tags
        })
}

fn has_next_page(document: &Html) -> bool {
    let rel_next_selector = selector("link[rel=\"next\"]");
    if document.select(&rel_next_selector).next().is_some() {
        return true;
    }

    let selector = selector("section.pag a");
    document.select(&selector).any(|link| {
        link.text()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
            .contains("next")
    })
}

fn preferred_download(wallpaper: &MotionBgsWallpaper) -> Option<MotionBgsDownloadOption> {
    wallpaper
        .downloads
        .iter()
        .find(|download| download.quality == wallpaper.quality)
        .or_else(|| {
            wallpaper
                .downloads
                .iter()
                .find(|download| download.quality == "4K")
        })
        .or_else(|| wallpaper.downloads.first())
        .cloned()
}

fn video_source_url(document: &Html) -> Option<String> {
    let selector = selector("video source[src]");
    document
        .select(&selector)
        .next()
        .and_then(|source| source.value().attr("src"))
        .and_then(absolute_url)
}

fn meta_content(document: &Html, selector_text: &str) -> Option<String> {
    let selector = selector(selector_text);
    document
        .select(&selector)
        .next()
        .and_then(|node| node.value().attr("content"))
        .map(str::to_string)
}

fn clean_query_value(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn clean_title(title: String) -> String {
    let title = title.trim().to_string();
    let lower = title.to_ascii_lowercase();
    if let Some(index) = lower.rfind(" live wallpaper") {
        title[..index].trim().to_string()
    } else {
        title
    }
}

fn title_from_slug(slug: &str) -> String {
    slug.replace('-', " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn absolute_url(value: &str) -> Option<String> {
    reqwest::Url::parse(MOTIONBGS_BASE)
        .ok()?
        .join(value.trim())
        .ok()
        .map(|url| url.to_string())
}

fn high_resolution_image_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let path = parsed.path();
    if !(path.starts_with("/i/c/364x205/media/") || path.starts_with("/i/c/546x308/media/")) {
        return None;
    }

    let high_res_path = path
        .replace("/i/c/364x205/media/", "/i/c/960x540/media/")
        .replace("/i/c/546x308/media/", "/i/c/960x540/media/");
    let mut high_res_url = parsed;
    high_res_url.set_path(&high_res_path);
    Some(high_res_url.to_string())
}

fn inferred_preview_video_url(id: &str, slug: &str) -> Option<String> {
    if !id.chars().all(|ch| ch.is_ascii_digit()) || slug.is_empty() {
        return None;
    }

    absolute_url(&format!("/media/{id}/{slug}.960x540.mp4"))
}

fn validate_motionbgs_page_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url.trim())
        .map_err(|_| "Invalid MotionBGS wallpaper URL".to_string())?;
    if is_motionbgs_page_url(parsed.as_str()) {
        Ok(parsed)
    } else {
        Err("Only public MotionBGS wallpaper pages can be fetched".to_string())
    }
}

fn validate_motionbgs_download_url(url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(url.trim())
        .map_err(|_| "Invalid MotionBGS download URL".to_string())?;
    if is_motionbgs_download_url(parsed.as_str()) {
        Ok(parsed)
    } else {
        Err("Only public MotionBGS MP4 download URLs can be downloaded".to_string())
    }
}

fn is_motionbgs_page_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| {
            let host = url.host_str()?.to_ascii_lowercase();
            let path = url.path();
            Some(
                url.scheme() == "https"
                    && host == "motionbgs.com"
                    && path.starts_with('/')
                    && !path.starts_with("/admin/")
                    && !path.starts_with("/ajax/")
                    && !path.starts_with("/a2/")
                    && !path.starts_with("/dl/")
                    && !path.starts_with("/media/")
                    && !path.starts_with("/static/")
                    && !path.starts_with("/i/")
                    && !path.starts_with("/tag:")
                    && !path.starts_with("/page/")
                    && path.trim_matches('/').split('/').count() <= 1,
            )
        })
        .unwrap_or(false)
}

fn is_motionbgs_download_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| {
            let host = url.host_str()?.to_ascii_lowercase();
            let parts = url
                .path_segments()
                .map(|segments| segments.collect::<Vec<_>>())
                .unwrap_or_default();
            Some(
                url.scheme() == "https"
                    && host == "motionbgs.com"
                    && parts.len() >= 3
                    && parts[0] == "dl"
                    && matches!(parts[1], "hd" | "4k")
                    && parts[2].chars().all(|ch| ch.is_ascii_digit()),
            )
        })
        .unwrap_or(false)
}

fn slug_from_page_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    parsed
        .path_segments()?
        .find(|segment| !segment.is_empty())
        .map(str::to_string)
}

fn media_id_from_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let segments = parsed.path_segments()?.collect::<Vec<_>>();
    segments
        .windows(2)
        .find(|window| window[0] == "media" && window[1].chars().all(|ch| ch.is_ascii_digit()))
        .map(|window| window[1].to_string())
}

fn id_from_download_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let segments = parsed.path_segments()?.collect::<Vec<_>>();
    if segments.len() >= 3 && segments[0] == "dl" {
        Some(segments[2].to_string())
    } else {
        None
    }
}

fn download_quality_from_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let segments = parsed.path_segments()?.collect::<Vec<_>>();
    match segments.get(1).copied() {
        Some("4k") => Some("4K".to_string()),
        Some("hd") => Some("HD".to_string()),
        _ => None,
    }
}

fn dimensions_for_quality(quality: &str) -> (i32, i32) {
    match quality.to_ascii_uppercase().as_str() {
        "4K" => (3840, 2160),
        _ => (1920, 1080),
    }
}

fn parse_dimensions(text: &str) -> Option<(i32, i32)> {
    text.split_whitespace().find_map(|part| {
        let cleaned = part.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        let (width, height) = cleaned.split_once('x')?;
        let width = width.parse::<i32>().ok()?;
        let height = height.parse::<i32>().ok()?;
        if (1..=16_384).contains(&width) && (1..=16_384).contains(&height) {
            Some((width, height))
        } else {
            None
        }
    })
}

fn parse_size_mb(text: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let mb_index = lower.find("mb")?;
    let before = &lower[..mb_index];
    let start = before
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_digit() || *ch == '.')
        .last()
        .map(|(index, _)| index)?;
    let value = before[start..].parse::<f64>().ok()?;
    Some((value * 1024.0 * 1024.0).round() as i64)
}

pub(crate) fn motionbgs_download_source_id(wallpaper: &MotionBgsWallpaper) -> String {
    let quality = wallpaper
        .quality
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    let quality = if quality.is_empty() {
        "hd".to_string()
    } else {
        quality
    };

    format!("{}-{quality}", wallpaper.id)
}

pub(crate) fn mp4_duration_ms(bytes: &[u8]) -> Option<i64> {
    let moov = find_mp4_child_box(bytes, 0, bytes.len(), b"moov")?;
    let mvhd = find_mp4_child_box(bytes, moov.payload_start, moov.end, b"mvhd")?;
    parse_mvhd_duration_ms(&bytes[mvhd.payload_start..mvhd.end])
}

fn parse_mvhd_duration_ms(payload: &[u8]) -> Option<i64> {
    let version = *payload.first()?;

    let (timescale, duration) = match version {
        0 => {
            let timescale = read_u32(payload, 12)? as u64;
            let duration = read_u32(payload, 16)? as u64;
            (timescale, duration)
        }
        1 => {
            let timescale = read_u32(payload, 20)? as u64;
            let duration = read_u64(payload, 24)?;
            (timescale, duration)
        }
        _ => return None,
    };

    if timescale == 0 {
        return None;
    }

    let duration_ms = duration.checked_mul(1000)?.checked_div(timescale)?;
    i64::try_from(duration_ms).ok().filter(|value| *value >= 0)
}

#[derive(Debug)]
struct Mp4Box {
    payload_start: usize,
    end: usize,
}

fn find_mp4_child_box(bytes: &[u8], start: usize, end: usize, name: &[u8; 4]) -> Option<Mp4Box> {
    let mut cursor = start;

    while cursor.checked_add(8)? <= end {
        let size = read_u32(bytes, cursor)? as u64;
        let box_type = bytes.get(cursor + 4..cursor + 8)?;
        let mut header_size = 8_u64;

        let box_size = if size == 1 {
            header_size = 16;
            read_u64(bytes, cursor + 8)?
        } else if size == 0 {
            (end - cursor) as u64
        } else {
            size
        };

        if box_size < header_size {
            return None;
        }

        let box_end = cursor.checked_add(usize::try_from(box_size).ok()?)?;
        if box_end > end {
            return None;
        }

        let payload_start = cursor.checked_add(usize::try_from(header_size).ok()?)?;
        if box_type == name {
            return Some(Mp4Box {
                payload_start,
                end: box_end,
            });
        }

        cursor = box_end;
    }

    None
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_be_bytes(slice.try_into().ok()?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let slice = bytes.get(offset..offset.checked_add(8)?)?;
    Some(u64::from_be_bytes(slice.try_into().ok()?))
}

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("static CSS selector should parse")
}

#[cfg(test)]
fn content_disposition_filename(
    disposition: Option<&reqwest::header::HeaderValue>,
) -> Option<String> {
    let disposition = disposition?.to_str().ok()?;
    disposition
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("filename="))
        .map(|value| value.trim_matches('"').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING_HTML: &str = r#"
        <div class=tmb>
          <a title="Nature in Minecraft live wallpaper" href=/nature-in-minecraft>
            <figure><picture><img alt="nature in minecraft live wallpaper" src=/i/c/364x205/media/1964/nature-in-minecraft.jpg></picture></figure>
            <span class=ttl>Nature in Minecraft</span>
            <span class=frm> 4K </span>
          </a>
        </div>
        <section class=pag><a href=/search?q=minecraft&page=2>Next</a></section>
    "#;

    const DETAIL_HTML: &str = r#"
        <meta property=og:title content="Nature in Minecraft Live Wallpaper">
        <meta property=og:url content="https://motionbgs.com/nature-in-minecraft">
        <meta property=og:image content="https://motionbgs.com/media/1964/nature-in-minecraft.jpg">
        <meta property=og:video content="https://motionbgs.com/media/1964/nature-in-minecraft.960x540.mp4">
        <ul class=subtags><li><a href=/tag:games/><span>Games</span></a></li><li><a href=/tag:minecraft/><span>Minecraft</span></a></li></ul>
        <section class=dl>
          <a href=/dl/4k/1964><div><span>4K</span> Wallpaper (14.0Mb)</div><div>3840x2160 mp4 file</div></a>
          <a href=/dl/hd/1964><div><span>HD</span> Wallpaper (7.7Mb)</div><div>1920x1080 mp4 file</div></a>
        </section>
    "#;

    #[test]
    fn listing_parser_extracts_public_cards() {
        let document = Html::parse_document(LISTING_HTML);
        let wallpapers = parse_listing_wallpapers(&document);

        assert_eq!(wallpapers.len(), 1);
        assert_eq!(wallpapers[0].id, "1964");
        assert_eq!(wallpapers[0].title, "Nature in Minecraft");
        assert_eq!(wallpapers[0].quality, "4K");
        assert_eq!(wallpapers[0].width, 3840);
        assert_eq!(
            wallpapers[0].thumbnail_url,
            "https://motionbgs.com/i/c/960x540/media/1964/nature-in-minecraft.jpg"
        );
        assert_eq!(
            wallpapers[0].preview_video_url.as_deref(),
            Some("https://motionbgs.com/media/1964/nature-in-minecraft.960x540.mp4")
        );
        assert_eq!(
            wallpapers[0]
                .downloads
                .iter()
                .map(|download| download.quality.as_str())
                .collect::<Vec<_>>(),
            vec!["4K", "HD"]
        );
        assert!(has_next_page(&document));
    }

    #[test]
    fn detail_parser_extracts_video_downloads_and_tags() {
        let document = Html::parse_document(DETAIL_HTML);
        let fallback = MotionBgsWallpaper {
            id: "1964".to_string(),
            slug: "nature-in-minecraft".to_string(),
            title: "Nature in Minecraft".to_string(),
            url: "https://motionbgs.com/nature-in-minecraft".to_string(),
            thumbnail_url: "https://motionbgs.com/i/c/364x205/media/1964/nature-in-minecraft.jpg"
                .to_string(),
            preview_video_url: None,
            quality: "4K".to_string(),
            width: 3840,
            height: 2160,
            file_size: 0,
            tags: Vec::new(),
            downloads: Vec::new(),
        };

        let wallpaper = parse_detail_wallpaper(&document, &fallback).expect("detail parses");

        assert_eq!(wallpaper.id, "1964");
        assert_eq!(
            wallpaper.preview_video_url.as_deref(),
            Some("https://motionbgs.com/media/1964/nature-in-minecraft.960x540.mp4")
        );
        assert_eq!(wallpaper.tags, vec!["Games", "Minecraft"]);
        assert_eq!(wallpaper.downloads.len(), 2);
        assert_eq!(wallpaper.downloads[0].file_size, 14_680_064);
    }

    #[test]
    fn download_url_validation_accepts_only_public_mp4_routes() {
        assert!(is_motionbgs_download_url(
            "https://motionbgs.com/dl/4k/1964/"
        ));
        assert!(is_motionbgs_download_url(
            "https://motionbgs.com/dl/hd/1964/"
        ));
        assert!(!is_motionbgs_download_url(
            "https://motionbgs.com/ajax/file/1964"
        ));
        assert!(!is_motionbgs_download_url(
            "https://example.com/dl/4k/1964/"
        ));
    }

    #[test]
    fn feed_paths_match_motionbgs_pagination_routes() {
        assert_eq!(motionbgs_feed_path(None, 1), "/");
        assert_eq!(motionbgs_feed_path(None, 2), "/2/");
        assert_eq!(motionbgs_feed_path(Some("4k"), 1), "/4k/");
        assert_eq!(motionbgs_feed_path(Some("4k"), 2), "/4k/2/");
        assert_eq!(motionbgs_feed_path(Some("mobile"), 2), "/mobile/2/");
        assert_eq!(motionbgs_feed_path(Some("gifs"), 2), "/gifs/2/");
    }

    #[test]
    fn download_source_id_includes_selected_quality() {
        let wallpaper = MotionBgsWallpaper {
            id: "1964".to_string(),
            slug: "nature-in-minecraft".to_string(),
            title: "Nature in Minecraft".to_string(),
            url: "https://motionbgs.com/nature-in-minecraft".to_string(),
            thumbnail_url: "https://motionbgs.com/media/1964/nature-in-minecraft.jpg".to_string(),
            preview_video_url: None,
            quality: "4K".to_string(),
            width: 3840,
            height: 2160,
            file_size: 0,
            tags: Vec::new(),
            downloads: Vec::new(),
        };

        assert_eq!(motionbgs_download_source_id(&wallpaper), "1964-4k");
    }

    #[test]
    fn parses_mp4_duration_from_mvhd() {
        let mut mvhd_payload = vec![0_u8; 100];
        mvhd_payload[12..16].copy_from_slice(&1_000_u32.to_be_bytes());
        mvhd_payload[16..20].copy_from_slice(&2_500_u32.to_be_bytes());

        let mvhd = mp4_box(b"mvhd", &mvhd_payload);
        let moov = mp4_box(b"moov", &mvhd);
        let ftyp = mp4_box(b"ftyp", b"isom0000");
        let bytes = [ftyp, moov].concat();

        assert_eq!(mp4_duration_ms(&bytes), Some(2_500));
    }

    #[test]
    fn parses_content_disposition_filename() {
        let value = reqwest::header::HeaderValue::from_static(
            "attachment; filename=\"nature-in-minecraft.3840x2160.mp4\"",
        );

        assert_eq!(
            content_disposition_filename(Some(&value)).as_deref(),
            Some("nature-in-minecraft.3840x2160.mp4"),
        );
    }

    fn mp4_box(name: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + payload.len());
        bytes.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(payload);
        bytes
    }
}
