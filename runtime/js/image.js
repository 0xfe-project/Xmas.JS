// xmas:image – Bun.Image-compatible API backed by Rust's image crate.
// Host functions available as globals:
//   __image_metadata(b64) → JSON string "{width:N,height:N}"
//   __image_resize(b64, w, h) → "W:H:<rgba_b64>"
//   __image_encode(rgba_b64, w, h, format, quality) → b64 encoded bytes
//     (when w===0 && h===0, first arg is the original encoded bytes, no resize)
//
// API shape matches Bun.Image:
//   new Image(Buffer | Uint8Array)
//   img.metadata()           → Promise<{width, height}>
//   img.resize(w, h)         → Image  (new instance, lazy – stores pixel buf)
//   img.jpeg({quality?:80})  → Encoder
//   img.webp({quality?:80})  → Encoder
//   img.png()                → Encoder
//   encoder.buffer()         → Promise<Buffer>

(function () {
  // Convert Buffer/Uint8Array → base64 string without TextEncoder (not always available)
  function toB64(buf) {
    const bytes = buf instanceof Uint8Array ? buf : new Uint8Array(buf.buffer ?? buf);
    // btoa works on latin1; chunk to avoid stack overflow on large arrays
    let str = '';
    const CHUNK = 8192;
    for (let i = 0; i < bytes.length; i += CHUNK) {
      str += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
    }
    return btoa(str);
  }

  // base64 → Uint8Array
  function fromB64(b64) {
    const str = atob(b64);
    const out = new Uint8Array(str.length);
    for (let i = 0; i < str.length; i++) out[i] = str.charCodeAt(i);
    return out;
  }

  class Encoder {
    #source; // { kind: 'bytes', b64 } | { kind: 'pixels', b64, w, h }
    #format;
    #quality;

    constructor(source, format, quality) {
      this.#source = source;
      this.#format = format;
      this.#quality = quality;
    }

    buffer() {
      return Promise.resolve().then(() => {
        let encoded;
        if (this.#source.kind === 'pixels') {
          encoded = __image_encode(
            this.#source.b64,
            this.#source.w,
            this.#source.h,
            this.#format,
            this.#quality,
          );
        } else {
          // original bytes, no resize
          encoded = __image_encode(this.#source.b64, 0, 0, this.#format, this.#quality);
        }
        return fromB64(encoded);
      });
    }
  }

  // A sentinel used by Image.resize() to construct a pre-decoded instance
  // without re-encoding bytes: passed as the first arg, skips the bytes check.
  const PIXELS = Symbol('pixels');

  class Image {
    // Internal state: either raw encoded bytes OR decoded RGBA pixels
    #bytesB64;   // base64 of original encoded bytes (always present)
    #pixelsB64;  // base64 of RGBA pixel data (set after resize)
    #width;      // pixel width after resize
    #height;     // pixel height after resize

    constructor(buf) {
      if (buf === PIXELS) {
        // Called from resize(); caller fills private fields via the returned instance.
        this.#bytesB64 = '';
        this.#pixelsB64 = null;
        this.#width = 0;
        this.#height = 0;
        return;
      }
      if (!buf) throw new TypeError('Image: expected a Buffer or Uint8Array');
      this.#bytesB64 = toB64(buf);
      this.#pixelsB64 = null;
      this.#width = 0;
      this.#height = 0;
    }

    metadata() {
      return Promise.resolve().then(() => {
        // If this image was resized, return the resized dimensions
        if (this.#pixelsB64) {
          return { width: this.#width, height: this.#height };
        }
        const json = __image_metadata(this.#bytesB64);
        return JSON.parse(json);
      });
    }

    resize(w, h) {
      const result = __image_resize(this.#bytesB64, w, h);
      // result: "W:H:<rgba_b64>"
      const first = result.indexOf(':');
      const second = result.indexOf(':', first + 1);
      const next = new Image(PIXELS);
      next.#width = parseInt(result.slice(0, first), 10);
      next.#height = parseInt(result.slice(first + 1, second), 10);
      next.#pixelsB64 = result.slice(second + 1);
      next.#bytesB64 = this.#bytesB64; // keep original for metadata fallback
      return next;
    }

    jpeg(opts) {
      const q = opts?.quality ?? 80;
      return this._encoder('jpeg', q);
    }

    webp(opts) {
      const q = opts?.quality ?? 80;
      return this._encoder('webp', q);
    }

    png() {
      return this._encoder('png', 100);
    }

    _encoder(format, quality) {
      if (this.#pixelsB64) {
        return new Encoder(
          { kind: 'pixels', b64: this.#pixelsB64, w: this.#width, h: this.#height },
          format,
          quality,
        );
      }
      return new Encoder({ kind: 'bytes', b64: this.#bytesB64 }, format, quality);
    }
  }

  // Expose module
  globalThis.__image_module = { Image, default: Image };
})();
