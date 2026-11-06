//! Stores 16:9 photos and base-map renders sent as data URLs.

use std::path::Path;

use base64::Engine;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// 20 MB is plenty for a 1920×1080 JPEG/WebP or a large base-map render.
const MAX_BYTES: usize = 20 * 1024 * 1024;

/// Decodes `data:image/...;base64,...`, writes it under `upload_dir/<folder>`
/// and returns the public URL (`/uploads/<folder>/<uuid>.<ext>`).
pub async fn save_data_url(upload_dir: &Path, folder: &str, data_url: &str) -> AppResult<String> {
    let (header, payload) = data_url
        .split_once(',')
        .ok_or_else(|| AppError::bad("The photo could not be read. Please take it again."))?;
    let ext = match header {
        h if h.starts_with("data:image/jpeg") => "jpg",
        h if h.starts_with("data:image/webp") => "webp",
        h if h.starts_with("data:image/png") => "png",
        _ => return Err(AppError::bad("Only JPEG, WebP or PNG images can be uploaded.")),
    };
    if !header.ends_with(";base64") {
        return Err(AppError::bad("The photo could not be read. Please take it again."));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|_| AppError::bad("The photo could not be read. Please take it again."))?;
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(AppError::bad("That image is too large. Please use one under 20 MB."));
    }
    let dir = upload_dir.join(folder);
    tokio::fs::create_dir_all(&dir).await.map_err(anyhow::Error::from)?;
    let name = format!("{}.{ext}", Uuid::new_v4());
    tokio::fs::write(dir.join(&name), bytes).await.map_err(anyhow::Error::from)?;
    Ok(format!("/uploads/{folder}/{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_non_images() {
        let dir = std::env::temp_dir();
        let err = save_data_url(&dir, "t", "data:text/html;base64,PGgxPg==").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn saves_png() {
        let dir = std::env::temp_dir().join(format!("sm-test-{}", Uuid::new_v4()));
        // 1x1 transparent PNG
        let png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";
        let url = save_data_url(&dir, "photos", png).await.unwrap();
        assert!(url.starts_with("/uploads/photos/") && url.ends_with(".png"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
