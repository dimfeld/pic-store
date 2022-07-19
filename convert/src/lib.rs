use image::{DynamicImage, ImageResult};
use resize::resize_image;
pub use resize::ImageSizeTransform;
pub use write_format::EncodeError;

pub mod resize;
pub mod write_format;

pub fn image_from_bytes(bytes: &[u8]) -> ImageResult<DynamicImage> {
    image::load_from_memory(bytes)
}

pub fn convert(
    image: &DynamicImage,
    format: image::ImageFormat,
    size: &ImageSizeTransform,
) -> Result<Vec<u8>, EncodeError> {
    let resized = resize_image(image, size);
    let mut output = Vec::new();

    write_format::write_image(&resized, format, &mut output)?;
    Ok(output)
}
