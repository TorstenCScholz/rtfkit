//! Image block conversion from IR to docx-rs drawing elements.

use crate::allocators::ImageAllocator;
use crate::utils::{DEFAULT_IMAGE_EMU, px_to_emu, scale_emu, twips_to_emu};
use crate::DocxError;
use docx_rs::{Paragraph as DocxParagraph, Pic, Run as DocxRun};
use image::{GenericImageView, ImageFormat as RasterFormat};
use rtfkit_core::ImageBlock;
use std::io::Cursor;

/// Prepared image data: PNG bytes ready for DOCX packaging, and optional intrinsic pixel dims.
type PreparedImage = (Vec<u8>, Option<(u32, u32)>);

/// Returns image bytes that are safe for docx-rs packaging and optional intrinsic dimensions.
///
/// docx-rs stores image parts under `.png` paths, so JPEG is normalized to PNG.
/// For PNG, bytes are preserved as-is; if decoding fails we still proceed and fall
/// back to explicit RTF dimensions or a default display size.
pub(crate) fn prepare_image_for_docx(image: &ImageBlock) -> Result<PreparedImage, DocxError> {
    match image.format {
        rtfkit_core::ImageFormat::Png => {
            let intrinsic = image::load_from_memory_with_format(&image.data, RasterFormat::Png)
                .ok()
                .map(|img| img.dimensions());
            Ok((image.data.clone(), intrinsic))
        }
        rtfkit_core::ImageFormat::Jpeg => {
            let dyn_image = image::load_from_memory_with_format(&image.data, RasterFormat::Jpeg)
                .map_err(|err| DocxError::ImageEmbedding {
                    reason: format!("failed to decode JPEG image: {err}"),
                })?;

            let (width_px, height_px) = dyn_image.dimensions();
            let mut png_cursor = Cursor::new(Vec::new());
            dyn_image
                .write_to(&mut png_cursor, RasterFormat::Png)
                .map_err(|err| DocxError::ImageEmbedding {
                    reason: format!("failed to encode JPEG as PNG: {err}"),
                })?;
            Ok((
                png_cursor.into_inner(),
                Some((width_px.max(1), height_px.max(1))),
            ))
        }
    }
}

/// Resolves final display dimensions in EMUs from RTF hints and intrinsic pixel dimensions.
pub(crate) fn resolve_image_size_emu(
    image: &ImageBlock,
    intrinsic_dimensions: Option<(u32, u32)>,
) -> (u32, u32) {
    let (intrinsic_width_px, intrinsic_height_px) = intrinsic_dimensions.unwrap_or((96, 96));
    let width_from_twips = image.width_twips.filter(|w| *w > 0).map(twips_to_emu);
    let height_from_twips = image.height_twips.filter(|h| *h > 0).map(twips_to_emu);

    match (width_from_twips, height_from_twips) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => (
            width,
            scale_emu(width, intrinsic_height_px, intrinsic_width_px),
        ),
        (None, Some(height)) => (
            scale_emu(height, intrinsic_width_px, intrinsic_height_px),
            height,
        ),
        (None, None) => (
            intrinsic_dimensions
                .map(|_| px_to_emu(intrinsic_width_px))
                .unwrap_or(DEFAULT_IMAGE_EMU),
            intrinsic_dimensions
                .map(|_| px_to_emu(intrinsic_height_px))
                .unwrap_or(DEFAULT_IMAGE_EMU),
        ),
    }
}

/// Converts an IR ImageBlock to a docx-rs Paragraph containing a drawing run.
pub(crate) fn convert_image_block(
    image: &ImageBlock,
    images: &mut ImageAllocator,
) -> Result<DocxParagraph, DocxError> {
    let image_id = images.allocate_image_id();
    let (png_bytes, intrinsic_dimensions) = prepare_image_for_docx(image)?;
    let (intrinsic_w, intrinsic_h) = intrinsic_dimensions.unwrap_or((1, 1));
    let (width_emu, height_emu) = resolve_image_size_emu(image, intrinsic_dimensions);

    let pic = Pic::new_with_dimensions(png_bytes, intrinsic_w, intrinsic_h)
        .id(format!("rIdImage{image_id}"))
        .size(width_emu, height_emu);

    Ok(DocxParagraph::new().add_run(DocxRun::new().add_image(pic)))
}
