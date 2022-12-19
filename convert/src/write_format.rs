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

fn write_webp(image: &DynamicImage, mut writer: impl Write) -> Result<(), std::io::Error> {
    let format = if image.color().has_alpha() {
        webp::PixelLayout::Rgba
    } else {
        webp::PixelLayout::Rgb
    };

    let image = to_8bit(image);
    let (width, height) = image.dimensions();
    let encoder = webp::Encoder::new(image.as_bytes(), format, width, height);
    let output = encoder.encode(80.0);

    writer.write_all(&output)
}

fn write_jpeg(image: &DynamicImage, mut writer: impl Write) -> Result<(), image::ImageError> {
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, 70);
    let image = to_8bit(image);
    let (width, height) = image.dimensions();

    encoder.write_image(image.as_bytes(), width, height, image.color())
}

/// `cleared_alpha` only takes a Vec<RGBA<u8>>, not a slice, so recast the data as a Vec<rgb::RGBA<u8>>.
/// This is the same thing that the rgb crate functions do, except on a Vec.
fn cast_as_rgb_crate_format(data: image::RgbaImage) -> Vec<rgb::RGBA<u8>> {
    let data = data.into_vec();

    let mut data = std::mem::ManuallyDrop::new(data);
    let raw_data = data.as_mut_ptr();
    let len = data.len();
    let cap = data.capacity();
    unsafe { Vec::from_raw_parts(raw_data as *mut rgb::RGBA<u8>, len, cap) }
}

fn write_avif(image: &DynamicImage, mut writer: impl Write) -> Result<(), EncodeError> {
    let quality = 60.0;
    // From https://github.com/kornelski/cavif-rs/blob/main/src/main.rs
    let alpha_quality = ((quality + 100.0_f32) / 2.).min(quality + quality / 4. + 2.);

    let (width, height) = image.dimensions();

    let config = ravif::Config {
        quality,
        alpha_quality,
        speed: 4,
        premultiplied_alpha: false,
        color_space: ravif::ColorSpace::YCbCr,
        threads: None,
    };

    let image = to_8bit(image);

    let output = if image.color().has_alpha() {
        let data = image.into_owned().into_rgba8();
        let rgba_vec = cast_as_rgb_crate_format(data);

        let input_buf = ravif::Img::new(rgba_vec, width as usize, height as usize);

        let input_buf = ravif::cleared_alpha(input_buf);
        ravif::encode_rgba(input_buf.as_ref(), &config).map(|(o, _, _)| o)
    } else {
        let input_buf = ravif::Img::new(image.as_bytes().as_rgb(), width as usize, height as usize);
        ravif::encode_rgb(input_buf, &config).map(|(o, _)| o)
    };

    let output = output.map_err(|e| EncodeError::StringError(e.to_string()))?;

    writer.write_all(output.as_ref())?;
    Ok(())
}

pub fn write_image(
    image: &DynamicImage,
    output_format: ImageFormat,
    writer: impl Write,
) -> Result<(), EncodeError> {
    match output_format {
        ImageFormat::Png => write_png(image, writer)?,
        ImageFormat::WebP => write_webp(image, writer)?,
        ImageFormat::Avif => write_avif(image, writer)?,
        ImageFormat::Jpeg => write_jpeg(image, writer)?,
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
        super::write_image(&image, image::ImageFormat::Avif, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::AVIF);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }

    #[test]
    #[cfg(feature = "test-slow")]
    fn write_png() {
        let image = read_test_image("test-input.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::Png, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::PNG);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }

    #[test]
    fn write_webp() {
        let image = read_test_image("test-input.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::WebP, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::WEBP);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }

    #[test]
    fn write_jpeg() {
        let image = read_test_image("test-input.png");
        let mut output = Vec::new();
        super::write_image(&image, image::ImageFormat::Jpeg, &mut output).unwrap();

        let info = imageinfo::ImageInfo::from_raw_data(&output).expect("Reading image");
        assert_eq!(info.format, imageinfo::ImageFormat::JPEG);
        assert_eq!(info.size.width as u32, image.width());
        assert_eq!(info.size.height as u32, image.height());
    }
}
