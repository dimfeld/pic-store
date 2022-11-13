use image::{error::DecodingError, DynamicImage, ImageBuffer, ImageError, ImageResult};
use resize::resize_image;
pub use resize::ImageSizeTransform;
pub use write_format::EncodeError;

pub mod resize;
pub mod write_format;

fn load_avif(bytes: &[u8]) -> ImageResult<DynamicImage> {
    let pixels = libavif::decode_rgb(bytes).map_err(|e| {
        ImageError::Decoding(DecodingError::new(image::ImageFormat::Avif.into(), e))
    })?;

    let img = ImageBuffer::from_vec(pixels.width(), pixels.height(), pixels.to_vec())
        // If the above succeeded, this should always succeed.
        .expect("Failed to match dimensions");
    Ok(DynamicImage::ImageRgba8(img))
}

pub fn image_from_bytes(bytes: &[u8]) -> ImageResult<DynamicImage> {
    let format = imageinfo::ImageInfo::from_raw_data(bytes).map(|i| i.format);
    match format {
        // Some AVIF format files don't parse well using the image crate, so we
        // use libavif instead.
        Ok(imageinfo::ImageFormat::AVIF) => load_avif(bytes),
        _ => image::load_from_memory(bytes),
    }
}

pub fn convert(
    image: &DynamicImage,
    format: image::ImageFormat,
    size: &ImageSizeTransform,
) -> Result<Vec<u8>, EncodeError> {
    let resized = resize_image(image, size);
    let mut output = Vec::new();

    write_format::write_image(resized.as_ref().unwrap_or(image), format, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::{io::Read, path::PathBuf};

    use image::DynamicImage;

    fn read_test_image(filename: &str) -> DynamicImage {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures")
            .join(filename);
        let mut file = std::fs::File::open(&path).expect("opening file");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).expect("reading file");
        super::image_from_bytes(buffer.as_slice()).expect("parsing file")
    }

    #[test]
    fn read_avif() {
        let image = read_test_image("test-input.avif");
        assert_eq!(image.width(), 1334);
        assert_eq!(image.height(), 890);
    }

    #[test]
    fn read_jpeg() {
        let image = read_test_image("test-input.jpeg");
        assert_eq!(image.width(), 1334);
        assert_eq!(image.height(), 890);
    }

    #[test]
    fn read_png() {
        let image = read_test_image("test-input.png");
        assert_eq!(image.width(), 667);
        assert_eq!(image.height(), 445);
    }

    #[test]
    fn read_webp() {
        let image = read_test_image("test-input.webp");
        assert_eq!(image.width(), 1334);
        assert_eq!(image.height(), 890);
    }
}
