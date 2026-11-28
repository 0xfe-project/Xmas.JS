//! xmas:image – Bun.Image-compatible API: decode, resize, encode JPEG/WebP/PNG.
//!
//! JS calls host functions synchronously (images are in-memory blobs):
//!   __image_metadata(bytes_b64) → JSON "{width:N,height:N}"
//!   __image_resize(bytes_b64, w, h) → "W:H:<rgba_b64>"
//!   __image_encode(rgba_b64_or_bytes_b64, w, h, format, quality) → b64 encoded bytes
//!     w==0 && h==0 means "original encoded bytes, no resize"
//!
//! API shape matches Bun.Image (see js/image.js).

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use image::{DynamicImage, ImageFormat, imageops::FilterType};
use quickjs_oxide::engine::api::core::{JsString, RuntimeError, Scope, Value};

use crate::driver::Driver;

const PRELUDE: &str = include_str!("../../js/image.js");

fn text(body: &str) -> Result<Value, RuntimeError> {
    JsString::try_from_utf8(body)
        .map(Value::String)
        .map_err(|e| RuntimeError::Internal(e.to_string()))
}

fn decode_b64_image(b64: &str) -> Result<DynamicImage, String> {
    let bytes = STANDARD.decode(b64).map_err(|e| format!("base64: {e}"))?;
    image::load_from_memory(&bytes).map_err(|e| format!("decode: {e}"))
}

pub fn install(driver: &mut Driver) -> Result<(), String> {
    let global = driver
        .context()
        .global_object()
        .map_err(|e| format!("global: {e}"))?;

    // __image_metadata(b64) → JSON string "{width:N,height:N}"
    driver
        .context()
        .define_function(
            &global,
            "__image_metadata",
            1,
            move |scope: &mut Scope<'_>, b64: String| -> Result<Value, RuntimeError> {
                let img = decode_b64_image(&b64)
                    .map_err(|e| super::throw(scope, &e))?;
                let (w, h) = (img.width(), img.height());
                text(&format!(r#"{{"width":{w},"height":{h}}}"#))
            },
        )
        .map_err(|e| format!("__image_metadata: {e}"))?;

    // __image_resize(b64, w, h) → "W:H:<rgba_b64>"
    driver
        .context()
        .define_function(
            &global,
            "__image_resize",
            3,
            move |scope: &mut Scope<'_>, b64: String, w: f64, h: f64| -> Result<Value, RuntimeError> {
                let img = decode_b64_image(&b64)
                    .map_err(|e| super::throw(scope, &e))?;
                let resized = img.resize_exact(w as u32, h as u32, FilterType::Lanczos3);
                let rgba = resized.to_rgba8();
                let pixels_b64 = STANDARD.encode(rgba.as_raw());
                text(&format!("{}:{}:{}", resized.width(), resized.height(), pixels_b64))
            },
        )
        .map_err(|e| format!("__image_resize: {e}"))?;

    // __image_encode(source_b64, w, h, format, quality) → b64 encoded bytes
    // w > 0 && h > 0: source_b64 is raw RGBA pixels from __image_resize
    // w == 0 && h == 0: source_b64 is original encoded image bytes
    driver
        .context()
        .define_function(
            &global,
            "__image_encode",
            5,
            move |scope: &mut Scope<'_>, source_b64: String, w: f64, h: f64, format: String, quality: f64| -> Result<Value, RuntimeError> {
                let img: DynamicImage = if w > 0.0 && h > 0.0 {
                    let pixels = STANDARD.decode(&source_b64)
                        .map_err(|e| super::throw(scope, &format!("base64: {e}")))?;
                    let iw = w as u32;
                    let ih = h as u32;
                    let buf = image::RgbaImage::from_raw(iw, ih, pixels)
                        .ok_or_else(|| super::throw(scope, "pixel buffer size mismatch"))?;
                    DynamicImage::ImageRgba8(buf)
                } else {
                    decode_b64_image(&source_b64)
                        .map_err(|e| super::throw(scope, &e))?
                };

                let mut out: Vec<u8> = Vec::new();
                let q = (quality.clamp(0.0, 100.0) as u8).max(1);
                match format.as_str() {
                    "jpeg" => {
                        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q);
                        img.write_with_encoder(enc)
                            .map_err(|e| super::throw(scope, &format!("jpeg encode: {e}")))?;
                    }
                    "png" => {
                        img.write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
                            .map_err(|e| super::throw(scope, &format!("png encode: {e}")))?;
                    }
                    "webp" => {
                        img.write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::WebP)
                            .map_err(|e| super::throw(scope, &format!("webp encode: {e}")))?;
                    }
                    other => {
                        return Err(super::throw(scope, &format!("unknown format: {other}")));
                    }
                }
                text(&STANDARD.encode(&out))
            },
        )
        .map_err(|e| format!("__image_encode: {e}"))?;

    driver
        .context()
        .eval(PRELUDE)
        .map_err(|e| format!("image prelude: {e}"))?;
    let module = driver
        .context()
        .eval("globalThis.__image_module")
        .map_err(|e| format!("__image_module: {e}"))?;
    let names = ["Image", "default"];
    let exports = super::collect(driver, &module, &names)?;
    super::define_name(driver, "xmas:image", exports)
}
