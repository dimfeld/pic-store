use image::{DynamicImage, GenericImageView, ImageEncoder, ImageResult};
use rgb::FromSlice;
use std::{borrow::Cow, io::Write};
use thiserror::Error;

pub enum ImageFormat {
    Png,
    WebP,
    Avif,
}

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

    let (width, height) = image.dimensions();
    let source_image = to_8bit(image);
    encoder.write_image(source_image.as_bytes(), width, height, source_image.color())
}

fn write_webp(image: &DynamicImage, mut writer: impl Write) -> Result<(), std::io::Error> {
    let image = to_8bit(image);
    let format = if image.color().has_alpha() {
        webp::PixelLayout::Rgba
    } else {
        webp::PixelLayout::Rgb
    };
    let (width, height) = image.dimensions();
    let encoder = webp::Encoder::new(image.as_bytes(), format, width, height);
    let output = encoder.encode(60.0);

    writer.write_all(&output)
}

fn write_avif(image: &DynamicImage, mut writer: impl Write) -> Result<(), EncodeError> {
    let quality = 60.0;
    // From https://github.com/kornelski/cavif-rs/blob/main/src/main.rs
    let alpha_quality = ((quality + 100.0_f32) / 2.).min(quality + quality / 4. + 2.);

    let image = to_8bit(image);
    let (width, height) = image.dimensions();

    let config = ravif::Config {
        quality,
        alpha_quality,
        speed: 4,
        premultiplied_alpha: false,
        color_space: ravif::ColorSpace::RGB,
        threads: 0,
    };

    let output = if image.color().has_alpha() {
        let pixel_buf = image.as_bytes().as_rgba();
        let input_buf = ravif::Img::new(pixel_buf, width as usize, height as usize);
        ravif::encode_rgba(input_buf, &config).map(|(o, _, _)| o)
    } else {
        let pixel_buf = image.as_bytes().as_rgb();
        let input_buf = ravif::Img::new(pixel_buf, width as usize, height as usize);
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
    };

    Ok(())
}

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error(transparent)]
    ImageError(image::ImageError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
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
