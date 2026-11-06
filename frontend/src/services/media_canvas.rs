//! Client-side 16:9 auto-cropper & compressor.
//!
//! Every site photo is centre-cropped to exactly 16:9 (max 1920×1080) and
//! compressed (WebP, or JPEG where WebP isn't supported, quality 0.82)
//! before upload, so a call-out pin at (x%, y%) points at the same physical
//! spot on an iPhone, iPad, Mac or Windows screen.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, HtmlInputElement, Url};

pub const MAX_WIDTH: f64 = 1920.0;
pub const QUALITY: f64 = 0.82;

/// The centre-crop rectangle (sx, sy, sw, sh) for a strict 16:9 frame.
pub fn crop_rect(width: f64, height: f64) -> (f64, f64, f64, f64) {
    let target = 16.0 / 9.0;
    if width / height > target {
        let sw = height * target;
        ((width - sw) / 2.0, 0.0, sw, height)
    } else {
        let sh = width / target;
        (0.0, (height - sh) / 2.0, width, sh)
    }
}

/// Output size: as large as the crop allows, capped at 1920×1080.
pub fn output_size(crop_w: f64) -> (u32, u32) {
    let w = crop_w.min(MAX_WIDTH).max(16.0).round();
    let h = (w * 9.0 / 16.0).round();
    (w as u32, h as u32)
}

fn load_image(src: &str) -> Result<(HtmlImageElement, JsFuture), String> {
    let img = HtmlImageElement::new().map_err(|_| "Could not read the photo.".to_string())?;
    let target = img.clone();
    let promise = js_sys::Promise::new(&mut |resolve: js_sys::Function, reject: js_sys::Function| {
        let on_load = Closure::once_into_js(move || {
            let _ = resolve.call0(&JsValue::NULL);
        });
        let on_error = Closure::once_into_js(move || {
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("load failed"));
        });
        target.set_onload(Some(on_load.unchecked_ref()));
        target.set_onerror(Some(on_error.unchecked_ref()));
    });
    img.set_src(src);
    Ok((img, JsFuture::from(promise)))
}

/// Reads the file chosen in `<input id=input_id type=file>`, crops it to
/// 16:9 and returns a compressed data URL ready for the API or the queue.
pub async fn crop_from_input(input_id: &str) -> Result<Option<String>, String> {
    let input: HtmlInputElement = crate::util::document()
        .get_element_by_id(input_id)
        .and_then(|e| e.dyn_into().ok())
        .ok_or("The camera input is missing.")?;
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
        return Ok(None);
    };
    let url = Url::create_object_url_with_blob(&file).map_err(|_| "Could not read the photo.".to_string())?;
    let result = crop_url(&url).await;
    let _ = Url::revoke_object_url(&url);
    // Allow choosing the same file again later.
    input.set_value("");
    result.map(Some)
}

/// Reads a base-map render without cropping (keeps its aspect ratio),
/// downscaled to `max_width` and compressed as JPEG.
pub async fn image_from_input(input_id: &str, max_width: f64) -> Result<Option<String>, String> {
    let input: HtmlInputElement = crate::util::document()
        .get_element_by_id(input_id)
        .and_then(|e| e.dyn_into().ok())
        .ok_or("The file input is missing.")?;
    let Some(file) = input.files().and_then(|f| f.get(0)) else {
        return Ok(None);
    };
    let url = Url::create_object_url_with_blob(&file).map_err(|_| "Could not read the image.".to_string())?;
    let result = async {
        let (img, loaded) = load_image(&url)?;
        loaded.await.map_err(|_| "That file doesn't look like an image.".to_string())?;
        let (w, h) = (img.natural_width() as f64, img.natural_height() as f64);
        let scale = (max_width / w).min(1.0);
        let (ow, oh) = ((w * scale).round() as u32, (h * scale).round() as u32);
        let canvas: HtmlCanvasElement = crate::util::document()
            .create_element("canvas")
            .map_err(|_| "Canvas unavailable".to_string())?
            .dyn_into()
            .map_err(|_| "Canvas unavailable".to_string())?;
        canvas.set_width(ow);
        canvas.set_height(oh);
        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|c| c.dyn_into().ok())
            .ok_or("Canvas unavailable")?;
        ctx.draw_image_with_html_image_element_and_dw_and_dh(&img, 0.0, 0.0, ow as f64, oh as f64)
            .map_err(|_| "Could not process the image.".to_string())?;
        canvas
            .to_data_url_with_type_and_encoder_options("image/jpeg", &JsValue::from_f64(0.9))
            .map_err(|_| "Could not compress the image.".to_string())
    }
    .await;
    let _ = Url::revoke_object_url(&url);
    input.set_value("");
    result.map(Some)
}

/// Crops any image URL to a 16:9 data URL.
pub async fn crop_url(url: &str) -> Result<String, String> {
    let (img, loaded) = load_image(url)?;
    loaded.await.map_err(|_| "That file doesn't look like a photo. Please choose a JPEG or PNG.".to_string())?;
    let (w, h) = (img.natural_width() as f64, img.natural_height() as f64);
    if w < 2.0 || h < 2.0 {
        return Err("That photo is too small.".into());
    }
    let (sx, sy, sw, sh) = crop_rect(w, h);
    let (ow, oh) = output_size(sw);

    let canvas: HtmlCanvasElement = crate::util::document()
        .create_element("canvas")
        .map_err(|_| "Canvas unavailable".to_string())?
        .dyn_into()
        .map_err(|_| "Canvas unavailable".to_string())?;
    canvas.set_width(ow);
    canvas.set_height(oh);
    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into().ok())
        .ok_or("Canvas unavailable")?;
    ctx.set_image_smoothing_enabled(true);
    ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
        &img, sx, sy, sw, sh, 0.0, 0.0, ow as f64, oh as f64,
    )
    .map_err(|_| "Could not process the photo.".to_string())?;

    let quality = JsValue::from_f64(QUALITY);
    let webp = canvas.to_data_url_with_type_and_encoder_options("image/webp", &quality).unwrap_or_default();
    if webp.starts_with("data:image/webp") {
        return Ok(webp);
    }
    canvas
        .to_data_url_with_type_and_encoder_options("image/jpeg", &quality)
        .map_err(|_| "Could not compress the photo.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_photos_are_trimmed_left_and_right() {
        let (sx, sy, sw, sh) = crop_rect(4000.0, 1500.0);
        assert_eq!(sy, 0.0);
        assert!((sw / sh - 16.0 / 9.0).abs() < 1e-9);
        assert!((sx - (4000.0 - sw) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn tall_photos_are_trimmed_top_and_bottom() {
        let (sx, _sy, sw, sh) = crop_rect(3024.0, 4032.0);
        assert_eq!(sx, 0.0);
        assert!((sw / sh - 16.0 / 9.0).abs() < 1e-9);
    }

    #[test]
    fn output_is_capped_at_1080p() {
        assert_eq!(output_size(4000.0), (1920, 1080));
        assert_eq!(output_size(1280.0), (1280, 720));
    }
}
