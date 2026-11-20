//! Stores 16:9 photos and base-map renders sent as data URLs, and cuts big
//! map pictures into zoom tiles.

use std::path::Path;

use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::RgbImage;
use shared::MapTiles;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Room for a full-resolution drone or render export (photos arrive already compressed).
const MAX_BYTES: usize = 60 * 1024 * 1024;
/// Bigger pictures would need gigabytes of memory to cut into tiles.
const MAX_MAP_PIXELS: u64 = 120_000_000;
const TILE: u32 = 512;

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
        return Err(AppError::bad("That picture is too large. Please use one under 60 MB."));
    }
    let dir = upload_dir.join(folder);
    tokio::fs::create_dir_all(&dir).await.map_err(anyhow::Error::from)?;
    let name = format!("{}.{ext}", Uuid::new_v4());
    tokio::fs::write(dir.join(&name), bytes).await.map_err(anyhow::Error::from)?;
    Ok(format!("/uploads/{folder}/{name}"))
}

fn save_jpeg(img: &RgbImage, path: &Path) -> anyhow::Result<()> {
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    JpegEncoder::new_with_quality(file, 82).encode_image(img)?;
    Ok(())
}

/// Cuts the map picture at `src` into 512 px JPEG tiles for every zoom level
/// (full size, then halved until one tile holds it all), plus a light overview.
/// Blocking and CPU-heavy: call it from `spawn_blocking`.
pub fn make_tiles(src: &Path, out_dir: &Path, base_url: &str) -> anyhow::Result<MapTiles> {
    let (w, h) = image::image_dimensions(src)?;
    if w as u64 * h as u64 > MAX_MAP_PIXELS {
        anyhow::bail!("That picture is too big to prepare ({w} × {h}). Please export it at up to about 12,000 pixels wide.");
    }
    let tiles = MapTiles { width: w, height: h, tile_size: TILE, max_level: MapTiles::levels_for(w, h, TILE), base_url: base_url.to_string() };
    let mut current = image::open(src)?.to_rgb8();
    let mut preview_saved = false;
    for level in (0..=tiles.max_level).rev() {
        let (lw, lh) = tiles.level_size(level);
        if current.dimensions() != (lw, lh) {
            current = imageops::resize(&current, lw, lh, FilterType::Triangle);
        }
        if !preview_saved && lw <= MapTiles::PREVIEW_WIDTH {
            std::fs::create_dir_all(out_dir)?;
            save_jpeg(&current, &out_dir.join("preview.jpg"))?;
            preview_saved = true;
        }
        let dir = out_dir.join(level.to_string());
        std::fs::create_dir_all(&dir)?;
        let (cols, rows) = tiles.grid(level);
        for y in 0..rows {
            for x in 0..cols {
                let (tx, ty) = (x * TILE, y * TILE);
                let tile = imageops::crop_imm(&current, tx, ty, TILE.min(lw - tx), TILE.min(lh - ty)).to_image();
                save_jpeg(&tile, &dir.join(format!("{x}_{y}.jpg")))?;
            }
        }
    }
    Ok(tiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_every_level_into_tiles() {
        let dir = std::env::temp_dir().join(format!("sm-tiles-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("map.png");
        RgbImage::from_fn(1300, 700, |x, y| image::Rgb([(x % 256) as u8, (y % 256) as u8, 90])).save(&src).unwrap();
        let t = make_tiles(&src, &dir.join("tiles"), "/uploads/maps/tiles/x").unwrap();
        assert_eq!((t.width, t.height, t.max_level), (1300, 700, 2));
        for level in 0..=2 {
            let (cols, rows) = t.grid(level);
            for (x, y) in [(0, 0), (cols - 1, rows - 1)] {
                assert!(dir.join(format!("tiles/{level}/{x}_{y}.jpg")).exists(), "level {level} tile {x},{y}");
            }
        }
        // Edge tiles are trimmed to the picture, not padded.
        assert_eq!(image::image_dimensions(dir.join("tiles/2/2_1.jpg")).unwrap(), (1300 - 1024, 700 - 512));
        assert!(dir.join("tiles/preview.jpg").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

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
