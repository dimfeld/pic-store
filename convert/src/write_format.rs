use std::{borrow::Cow, io::Write};

use image::{DynamicImage, GenericImageView, ImageEncoder, ImageFormat};
use rgb::FromSlice;
use thiserror::Error;

fn to_8bit(image: &'_ DynamicImage) -> Cow<'_, DynamicImage> {
    let input_color = image.color();
    match (input_color.has_alpha(), input_color.bytes_per_pixel() > 1) {
        (_, false) => Cow::Borrowed(image),
        (false, true) => Cow::Owned(DynamicImage::from(image.to_rgb8())),
        (true, true) => Cow::Owned(DynamicImage::from(image.to_rgba8())),
    }
}

fn write_png(image: &DynamicImage, writer: impl Write) -> Result<(), image::ImageError> {
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        writer,
        image::codecs::png::CompressionType::Best,
        image::codecs::png::FilterType::Adaptive,
    );

    let image = to_8bit(image);
    let (width, height) = image.dimensions();
    encoder.write_image(image.as_bytes(), width, height, image.color())
}

fn write_webp(
    image: &DynamicImage,
    quality: Option<f32>,
    mut writer: impl Write,
) -> Result<(), std::io::Error> {
    let format = if image.color().has_alpha() {
        webp::PixelLayout::Rgba
    } else {
        webp::PixelLayout::Rgb
    };

    let image = to_8bit(image);
    let (width, height) = image.dimensions();
    let quality = quality.unwrap_or(70.0);
    let encoder = webp::Encoder::new(image.as_bytes(), format, width, height);
    let output = if quality < 100.0 {
        encoder.encode(quality)
    } else {
        encoder.encode_lossless()
    };

    writer.write_all(&output)
}

fn write_jpeg(
    image: &DynamicImage,
    quality: Option<f32>,
    mut writer: impl Write,
) -> Result<(), image::ImageError> {
    let quality = quality.unwrap_or(70.0) as u8;
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality);
    let image = to_8bit(image);
    let (width, height) = image.dimensions();

    encoder.write_image(image.as_bytes(), width, height, image.color())
}

fn write_avif(
    image: &DynamicImage,
    quality: Option<f32>,
    mut writer: impl Write,
) -> Result<(), EncodeError> {
    let quality = quality.unwrap_or(60.0);
    // From https://github.com/kornelski/cavif-rs/blob/main/src/main.rs
    let alpha_quality = ((quality + 100.0_f32) / 2.).min(quality + quality / 4. + 2.);

    let (width, height) = image.dimensions();

    let encoder = ravif::Encoder::new()
        .with_quality(quality)
        .with_alpha_quality(alpha_quality)
        .with_speed(4);

    let image = to_8bit(image);

    let output = if image.color().has_alpha() {
        let input_buf =
            ravif::Img::new(image.as_bytes().as_rgba(), width as usize, height as usize);
        encoder.encode_rgba(input_buf)
    } else {
        let input_buf = ravif::Img::new(image.as_bytes().as_rgb(), width as usize, height as usize);
        encoder.encode_rgb(input_buf)
    };

    let output = output.map_err(|e| EncodeError::StringError(e.to_string()))?;

    writer.write_all(&output.avif_file)?;
    Ok(())
}

pub fn write_image(
    image: &DynamicImage,
    output_format: ImageFormat,
    quality: Option<f32>,
    writer: impl Write,
) -> Result<(), EncodeError> {
    match output_format {
        ImageFormat::Png => write_png(image, writer)?,
        ImageFormat::WebP => write_webp(image, quality, writer)?,
        ImageFormat::Avif => write_avif(image, quality, writer)?,
        ImageFormat::Jpeg => write_jpeg(image, quality, writer)?,
        _ => Err(EncodeError::UnsupportedFormat(output_format))?,
    };

    Ok(())
}

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error(transparent)]
    ImageError(image::ImageError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Unsupported output format {0:?}")]
    UnsupportedFormat(image::ImageFormat),
    #[error("{0}")]
    StringError(String),
}

impl From<image::ImageError> for EncodeError {
    fn from(err: image::ImageError) -> Self {
        match err {
            image::ImageError::IoError(e) => EncodeError::IoError(e),
            _ => EncodeError::ImageError(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use image::DynamicImage;

    fn read_test_image(filename: &str) -> DynamicImage {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures")
            .join(filename);
        image::open(path).expect("reading image")
    }

    #[test]
    #[cfg(feature = "test-slow")]
    fn write_avif() {
        let image = read_test_image("test-input.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::Avif, None, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::AVIF);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }

    #[test]
    #[cfg(feature = "test-slow")]
    fn write_avif_with_alpha() {
        let image = read_test_image("test-with-alpha.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::Avif, None, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::AVIF);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());

        std::fs::write("test-output.avif", &output).unwrap();
    }

    #[test]
    #[cfg(feature = "test-slow")]
    fn write_png() {
        let image = read_test_image("test-input.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::Png, None, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::PNG);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }

    #[test]
    fn write_webp() {
        let image = read_test_image("test-input.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::WebP, None, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::WEBP);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }

    #[test]
    fn write_jpeg() {
        let image = read_test_image("test-input.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::Jpeg, None, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::JPEG);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }
}
