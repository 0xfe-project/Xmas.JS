//! xmas:image – tests for the Bun.Image-compatible API.
#![cfg(feature = "image-module")]

use xmas_runtime::testkit::{Harness, text_of};

// A 1x1 PNG as a base64 literal, inlined to avoid disk I/O.
fn png_expr() -> &'static str {
    "(()=>{const s=atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==');const a=new Uint8Array(s.length);for(let i=0;i<s.length;i++)a[i]=s.charCodeAt(i);return a;})()"
}

// Image work answers with a promise, so each test exports an async function and settles it,
// the way harness.rs::async_test_with_timer does.

#[test]
fn image_module_is_importable() {
    let mut h = Harness::with_modules();
    let ns = h
        .module(
            "app",
            "import { Image } from 'xmas:image';
             export const out = typeof Image;",
        )
        .unwrap();
    let value = h.property(&ns, "out").unwrap();
    assert_eq!(text_of(&value).unwrap(), "function");
}

#[test]
fn metadata_returns_width_and_height() {
    let mut h = Harness::with_modules();
    let src = format!(
        "import {{ Image }} from 'xmas:image';
         export async function test() {{
             const meta = await new Image({png}).metadata();
             return meta.width + ':' + meta.height;
         }}",
        png = png_expr()
    );
    let ns = h.module("app", &src).unwrap();
    let promise = h.call(&ns, "test", &[]).unwrap();
    let value = smol::block_on(h.settle(promise)).unwrap();
    assert_eq!(text_of(&value).unwrap(), "1:1");
}

#[test]
fn png_encode_returns_bytes() {
    let mut h = Harness::with_modules();
    let src = format!(
        "import {{ Image }} from 'xmas:image';
         export async function test() {{
             const encoded = await new Image({png}).png().buffer();
             return (encoded instanceof Uint8Array) + ':' + (encoded.length > 0);
         }}",
        png = png_expr()
    );
    let ns = h.module("app", &src).unwrap();
    let promise = h.call(&ns, "test", &[]).unwrap();
    let value = smol::block_on(h.settle(promise)).unwrap();
    assert_eq!(text_of(&value).unwrap(), "true:true");
}

#[test]
fn jpeg_encode_returns_bytes() {
    let mut h = Harness::with_modules();
    let src = format!(
        "import {{ Image }} from 'xmas:image';
         export async function test() {{
             const encoded = await new Image({png}).jpeg({{ quality: 80 }}).buffer();
             return (encoded instanceof Uint8Array) + ':' + (encoded.length > 0);
         }}",
        png = png_expr()
    );
    let ns = h.module("app", &src).unwrap();
    let promise = h.call(&ns, "test", &[]).unwrap();
    let value = smol::block_on(h.settle(promise)).unwrap();
    assert_eq!(text_of(&value).unwrap(), "true:true");
}

#[test]
fn resize_then_encode_works() {
    let mut h = Harness::with_modules();
    let src = format!(
        "import {{ Image }} from 'xmas:image';
         export async function test() {{
             const encoded = await new Image({png}).resize(4, 4).png().buffer();
             return (encoded instanceof Uint8Array) + ':' + (encoded.length > 0);
         }}",
        png = png_expr()
    );
    let ns = h.module("app", &src).unwrap();
    let promise = h.call(&ns, "test", &[]).unwrap();
    let value = smol::block_on(h.settle(promise)).unwrap();
    assert_eq!(text_of(&value).unwrap(), "true:true");
}

#[test]
fn webp_encode_returns_bytes() {
    let mut h = Harness::with_modules();
    let src = format!(
        "import {{ Image }} from 'xmas:image';
         export async function test() {{
             const encoded = await new Image({png}).webp({{ quality: 75 }}).buffer();
             return (encoded instanceof Uint8Array) + ':' + (encoded.length > 0);
         }}",
        png = png_expr()
    );
    let ns = h.module("app", &src).unwrap();
    let promise = h.call(&ns, "test", &[]).unwrap();
    let value = smol::block_on(h.settle(promise)).unwrap();
    assert_eq!(text_of(&value).unwrap(), "true:true");
}


