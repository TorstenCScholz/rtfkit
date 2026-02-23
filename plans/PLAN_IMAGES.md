# rtfkit Plan: Image/Picture Support

**Status: PLANNED**

Add first-class image support to the RTF-to-IR-to-output pipeline. RTF `\pict` blocks containing embedded PNG, JPEG, and WMF/EMF image data are currently dropped with a `DroppedContent` warning. This is the biggest gap for real-world RTF conversion fidelity.

## 1. Primary objective

Extract embedded image data from RTF `\pict` groups, represent images in the IR, and emit them correctly in DOCX, HTML, and PDF output.

```
RTF {\pict\pngblip ...hexdata...}
  → IR Image { format, data, width, height }
  → DOCX <w:drawing> / HTML <img src="data:..."> / Typst #image()
```

## 2. Scope

### In scope

- Parse `\pict` groups and extract image binary data (hex-encoded in RTF).
- Recognize image format control words: `\pngblip` (PNG), `\jpegblip` (JPEG).
- Extract dimension control words: `\picw`, `\pich` (natural size in twips), `\picwgoal`, `\pichgoal` (desired display size in twips), `\picscalex`, `\picscaley` (scale percentages).
- New IR block element `ImageBlock` containing decoded image bytes, format, and dimensions.
- DOCX writer: embed image in `word/media/` and reference via `<w:drawing>`.
- HTML writer: emit `<img>` tag with inline `data:` URI or extracted file reference.
- Typst/PDF writer: emit `#image()` with embedded image bytes.
- Handle `\shppict` / `\nonshppict` preference (prefer `\shppict` content).
- Fixtures: at least 5 new RTF fixtures covering PNG, JPEG, dimensions, multiple images, and images in tables.
- Golden IR snapshots (with image data represented as base64 or hash).
- Strict-mode: images no longer emit `DroppedContent`.
- Parser limits: add `max_image_bytes` limit to prevent memory exhaustion from large embedded images.

### Out of scope

- WMF/EMF vector format conversion (emit warning, drop image or include raw bytes for DOCX passthrough).
- Image cropping (`\piccropl`, `\piccropr`, `\piccropt`, `\piccropb`).
- Image borders and shading.
- Floating/anchored image positioning (images are treated as inline block elements).
- OLE objects (`\object`, `\objdata`) — separate feature track.
- Image compression or re-encoding.
- BMP format (`\dibitmap`, `\wbitmap`) — rare in modern RTF.

## 3. Design constraints and principles

- Images are **block-level** elements in RTF (they appear where a paragraph would). Model as a `Block::Image(ImageBlock)`.
- Image data in RTF is hex-encoded. Decoding must happen in the interpreter, not the writers.
- Memory safety: large images could exhaust memory. Enforce `max_image_bytes` limit (default: 50 MB total across all images in a document).
- Images must round-trip deterministically: same RTF input → same IR → same output bytes.
- If image format is unrecognized, fall back to emitting `DroppedContent` with a descriptive reason — never silently lose an image.
- Preserve image dimensions. If display dimensions (`\picwgoal`, `\pichgoal`) are present, use those. Otherwise fall back to natural dimensions (`\picw`, `\pich`). If neither, omit dimensions and let the output format use intrinsic size.

## 4. IR changes

### New block element

```rust
/// Supported image formats in the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Png,
    Jpeg,
}

/// An embedded image block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageBlock {
    /// Image format
    pub format: ImageFormat,
    /// Raw image bytes (decoded from RTF hex)
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
    /// Display width in twips (from \picwgoal or \picw)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_twips: Option<i32>,
    /// Display height in twips (from \pichgoal or \pich)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_twips: Option<i32>,
}
```

Add to `Block` enum:

```rust
pub enum Block {
    Paragraph(Paragraph),
    ListBlock(ListBlock),
    TableBlock(TableBlock),
    Image(ImageBlock),          // NEW
}
```

### Base64 serialization

For IR JSON, image bytes must serialize as base64 to avoid huge hex dumps. Use a custom serde module (`base64_serde`) or the `base64` crate with serde support.

### Determinism note

Golden IR snapshots for image fixtures will contain base64-encoded image data. This is deterministic since the same hex input always decodes to the same bytes.

## 5. Interpreter changes

### `\pict` parsing state machine

Currently `\pict` and `\shppict` are in the `Dropped` destination list. Replace with targeted image-parsing mode:

1. When `\pict` is encountered as a destination, enter **image-parsing mode**.
2. Track image metadata control words within the `\pict` group:
   - `\pngblip` → format = PNG
   - `\jpegblip` → format = JPEG
   - `\emfblip`, `\wmetafile` → format = unsupported (emit warning)
   - `\picw N` → natural width in twips
   - `\pich N` → natural height in twips
   - `\picwgoal N` → desired display width in twips
   - `\pichgoal N` → desired display height in twips
   - `\picscalex N` → horizontal scale percentage (default 100)
   - `\picscaley N` → vertical scale percentage (default 100)
3. Collect hex text data within the group (concatenate all `Text` events).
4. On `\pict` group close:
   - Decode hex string to bytes.
   - Validate decoded size against `max_image_bytes` limit.
   - Compute display dimensions: `picwgoal` (preferred) or `picw` (fallback), scaled by `picscalex/y`.
   - Emit `Block::Image(ImageBlock { ... })`.
   - If format is unsupported or hex decoding fails, emit `DroppedContent` and skip.

### `\shppict` / `\nonshppict` handling

RTF often wraps images in `{\shppict {\pict ...}}` with a fallback `{\nonshppict {\pict ...}}`.

- `\shppict`: enter group, allow `\pict` parsing inside (preferred path).
- `\nonshppict`: skip entire group (it's the fallback for older readers).

### Hex decoding

RTF image data is a stream of hex digit pairs (`89504e47...`), potentially with whitespace/newlines. The decoder must:
- Strip all whitespace.
- Decode pairs of hex chars to bytes.
- Handle odd-length input gracefully (emit warning, truncate).

### Memory limit

Add to `ParserLimits`:

```rust
pub struct ParserLimits {
    // ...existing fields...
    /// Maximum total image bytes across all images (default: 50 MB)
    pub max_image_bytes: usize,
}
```

Track cumulative image bytes in the interpreter. Fail with `ConversionError` (exit code 2) if exceeded.

## 6. Writer changes

### DOCX writer (`rtfkit-docx`)

- Maintain an image registry mapping image index to media path (`word/media/imageN.png`).
- For each `Block::Image`, write image bytes to `word/media/imageN.{png,jpeg}`.
- Emit Drawing ML in `word/document.xml`:
  ```xml
  <w:drawing>
    <wp:inline distT="0" distB="0" distL="0" distR="0">
      <wp:extent cx="{width_emu}" cy="{height_emu}"/>
      <wp:docPr id="{N}" name="Image{N}"/>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
          <pic:pic>
            <pic:blipFill>
              <a:blip r:embed="rId{M}"/>
              <a:stretch><a:fillRect/></a:stretch>
            </pic:blipFill>
            <pic:spPr>
              <a:xfrm>
                <a:ext cx="{width_emu}" cy="{height_emu}"/>
              </a:xfrm>
            </pic:spPr>
          </pic:pic>
        </a:graphicData>
      </a:graphic>
    </wp:inline>
  </w:drawing>
  ```
- Add relationship entry in `word/_rels/document.xml.rels`.
- Dimension conversion: twips to EMU (1 twip = 914.4 EMU, or equivalently 1 inch = 914400 EMU = 1440 twips).
- Check `docx-rs` API for image/drawing support. If insufficient, may need raw XML construction.

### HTML writer (`rtfkit-html`)

- For `Block::Image`, emit:
  ```html
  <figure class="rtf-image">
    <img src="data:{mime};base64,{data}" width="{px}" height="{px}" alt="">
  </figure>
  ```
- Dimension conversion: twips to pixels (1 twip = 1/1440 inch, at 96 DPI → 1 twip ≈ 0.0667 px, or px = twips / 15).
- Alt text: empty string (RTF has no alt text concept). Could use `alt="Embedded image"` for accessibility.
- Add `.rtf-image` CSS class for styling (max-width: 100%, etc.).
- Consider optional external file output mode (future) vs. always-inline base64.

### Typst/PDF writer (`rtfkit-render-typst`)

- For `Block::Image`, write image bytes to a temporary file or use Typst's `#image.decode()` (bytes API) if available.
- Emit:
  ```typst
  #image.decode(bytes((0x89, 0x50, ...)), width: Xcm, height: Ycm)
  ```
  Or write to a virtual file in the Typst world and reference by path.
- Dimension conversion: twips to cm or pt for Typst.
- Typst's embedded compiler should handle PNG and JPEG natively.

## 7. Fixtures and tests

### New fixtures

| Fixture | Content |
|---------|---------|
| `image_png_simple.rtf` | Single embedded PNG image |
| `image_jpeg_simple.rtf` | Single embedded JPEG image |
| `image_with_dimensions.rtf` | Image with explicit `\picwgoal` and `\pichgoal` |
| `image_multiple.rtf` | Document with text, image, text, image |
| `image_in_table.rtf` | Image embedded inside a table cell |
| `image_unsupported_format.rtf` | WMF/EMF image (should warn and drop) |
| `image_large_truncated.rtf` | Image exceeding limit (should fail) |

### Test categories

| Category | Tests |
|----------|-------|
| Golden IR snapshots | 6 (base64 image data in JSON) |
| Hex decoding unit tests | 5 (valid, whitespace, odd-length, empty, invalid chars) |
| DOCX integration | 4 (verify media files in ZIP, Drawing ML in XML) |
| HTML integration | 3 (verify `<img>` tags, data URIs, dimensions) |
| PDF integration | 2 (verify image in Typst source) |
| Contract/strict-mode | 3 (image docs pass strict; unsupported formats still fail) |
| Limits | 2 (within limit passes, over limit fails with exit 2) |
| Determinism | 2 (IR and DOCX stability for image fixtures) |
| **Total** | **~27** |

## 8. Implementation order

1. **IR types** — Add `ImageFormat`, `ImageBlock`, extend `Block` enum. Add base64 serde support.
2. **Update all existing writers** — Add `Block::Image` match arm (initially emit warning or placeholder).
3. **Hex decoder** — Utility function for hex string to bytes with whitespace handling.
4. **Interpreter** — Add `\pict` parsing state machine, `\shppict`/`\nonshppict` handling.
5. **Parser limits** — Add `max_image_bytes` to `ParserLimits`.
6. **Fixtures** — Create RTF test files with embedded image data.
7. **Golden snapshots** — Generate and verify IR JSON with base64 image data.
8. **DOCX writer** — Image embedding with Drawing ML.
9. **HTML writer** — `<img>` with data URI.
10. **Typst writer** — `#image.decode()` or temp file approach.
11. **Contract tests** — Strict mode, limits, determinism.
12. **Documentation** — Update feature-support.md, warning-reference.md, limits-policy.md, CHANGELOG.

## 9. Acceptance criteria

1. `rtfkit convert image_png_simple.rtf -o out.docx` produces a DOCX with the image visible in Word.
2. `rtfkit convert image_png_simple.rtf --to html -o out.html` produces HTML with a visible `<img>`.
3. `rtfkit convert image_png_simple.rtf --to pdf -o out.pdf` produces a PDF with the image rendered.
4. `rtfkit convert image_png_simple.rtf --strict` exits 0 (no `DroppedContent`).
5. `rtfkit convert image_unsupported_format.rtf --strict` exits 4 (unsupported format dropped).
6. `rtfkit convert image_large_truncated.rtf` exits 2 (limit exceeded).
7. Image dimensions in output match `\picwgoal`/`\pichgoal` from the RTF source.
8. DOCX ZIP contains `word/media/image1.png` with correct bytes.
9. Golden IR snapshots pass for all new fixtures.
10. All existing tests continue to pass (no regressions).

## 10. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Large image data bloats IR JSON | Use base64 encoding (33% overhead vs hex's 100%); consider hash-only mode for `--emit-ir` |
| `docx-rs` may lack Drawing ML support | Check API; fall back to raw XML construction for `<w:drawing>` |
| Typst bytes API may not exist in embedded version | Write to virtual filesystem instead; test with actual Typst library |
| Hex decode performance for large images | Use efficient hex decoder; process in chunks if needed |
| WMF/EMF images are common in legacy RTF | Clearly document as unsupported; emit descriptive warning. Possible future track for WMF→SVG conversion |
| Memory pressure from multiple large images | `max_image_bytes` limit covers cumulative size; individual images bounded by overall input size limit |
| Golden snapshot size for image fixtures | Use small test images (1x1 pixel or small icon); keep fixture files manageable |

## 11. Dependencies

- `base64` crate (for serde serialization of image bytes in IR JSON).
- Possibly `hex` crate (for efficient hex decoding, though hand-rolled is fine).
- No new system dependencies — Typst compiler already handles PNG/JPEG natively.
