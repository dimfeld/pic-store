pub use error::*;
use eyre::eyre;
use image::{
    error::DecodingError, flat::SampleLayout, DynamicImage, FlatSamples, ImageBuffer, ImageError,
    Rgb, Rgba,
};
use resize::resize_image;
pub use resize::ImageSizeTransform;
pub use write_format::EncodeError;

mod error;
pub mod resize;
pub mod write_format;

fn load_avif(bytes: &[u8]) -> eyre::Result<DynamicImage> {
    let pixels = libavif::decode_rgb(bytes).map_err(|e| {
        ImageError::Decoding(DecodingError::new(image::ImageFormat::Avif.into(), e))
    })?;

    let img = ImageBuffer::from_vec(pixels.width(), pixels.height(), pixels.to_vec())
        // If the above succeeded, this should always succeed.
        .expect("Failed to match dimensions");
    Ok(DynamicImage::ImageRgba8(img))
}

fn load_heic(bytes: &[u8]) -> eyre::Result<DynamicImage> {
    use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};

    let lib_heif = LibHeif::new();
    let context = HeifContext::read_from_bytes(bytes)?;
    let handle = context.primary_image_handle()?;

    let bits_per_pixel = handle.luma_bits_per_pixel();
    let bytes_per_pixel = ((bits_per_pixel + 7) / 8) as usize;

    let has_alpha = handle.has_alpha_channel();
    let chroma = if has_alpha {
        RgbChroma::Rgba
    } else {
        RgbChroma::Rgb
    };

    let image = lib_heif.decode(&handle, ColorSpace::Rgb(chroma), None)?;

    let plane = image
        .planes()
        .interleaved
        .ok_or_else(|| eyre!("No interleaved data found for image"))?;

    let channels = if has_alpha { 4 } else { 3 };
    let width_stride = bytes_per_pixel * channels as usize;
    let samples = image::flat::FlatSamples {
        samples: plane.data,
        layout: SampleLayout {
            channels,
            channel_stride: bytes_per_pixel,
            width: plane.width,
            width_stride,
            height: plane.height,
            height_stride: plane.stride,
        },
        color_hint: None,
    };

    let width = plane.width;
    let height = plane.height;

    let output = match (has_alpha, bytes_per_pixel) {
        (true, 2) => {
            let u16samples = unsafe { std::mem::transmute::<_, FlatSamples<&[u16]>>(samples) };
            let view = u16samples.as_view::<Rgba<u16>>()?;
            DynamicImage::ImageRgba16(
                ImageBuffer::from_raw(width, height, view.samples().to_vec())
                    .ok_or_else(|| eyre!("Not enough image data to match dimensions"))?,
            )
        }
        (false, 2) => {
            let u16samples = unsafe { std::mem::transmute::<_, FlatSamples<&[u16]>>(samples) };
            let view = u16samples.as_view::<Rgb<u16>>()?;
            DynamicImage::ImageRgb16(
                ImageBuffer::from_raw(width, height, view.samples().to_vec())
                    .ok_or_else(|| eyre!("Not enough image data to match dimensions"))?,
            )
        }
        (true, 1) => {
            let view = samples.as_view::<Rgba<u8>>()?;
            DynamicImage::ImageRgba8(
                ImageBuffer::from_raw(width, height, view.samples().to_vec())
                    .ok_or_else(|| eyre!("Not enough image data to match dimensions"))?,
            )
        }
        (false, 1) => {
            let view = samples.as_view::<Rgb<u8>>()?;
            DynamicImage::ImageRgb8(
                ImageBuffer::from_raw(width, height, view.samples().to_vec())
                    .ok_or_else(|| eyre!("Not enough image data to match dimensions"))?,
            )
        }
        _ => return Err(eyre!("Unsupported bit depth {bits_per_pixel}")),
    };

    Ok(output)
}

pub fn image_from_bytes(bytes: &[u8]) -> Result<DynamicImage, Error> {
    let format = imageinfo::ImageInfo::from_raw_data(bytes).map(|i| i.format);
    let result = match format {
        // Some AVIF format files don't parse well using the image crate, so we
        // use libavif instead.
        Ok(imageinfo::ImageFormat::AVIF) => load_avif(bytes),
        Ok(imageinfo::ImageFormat::HEIC) => load_heic(bytes),
        _ => image::load_from_memory(bytes).map_err(eyre::Report::from),
    };

    result.map_err(|error| Error::Read {
        format: format.ok(),
        error,
    })
}

pub struct ConvertResult {
    pub width: u32,
    pub height: u32,
    pub image: Vec<u8>,
}

pub fn convert(
    image: &DynamicImage,
    format: image::ImageFormat,
    size: &ImageSizeTransform,
) -> Result<ConvertResult, EncodeError> {
    let resized = resize_image(image, size);
    let mut output = Vec::new();

    let convert_input = resized.as_ref().unwrap_or(image);

    let width = convert_input.width();
    let height = convert_input.height();

    write_format::write_image(convert_input, format, &mut output)?;
    Ok(ConvertResult {
        width,
        height,
        image: output,
    })
}

#[cfg(test)]
mod tests {
    use std::{io::Read, path::PathBuf};

    use image::DynamicImage;

    use crate::write_format::write_image;

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
    fn read_heic() {
        let image = read_test_image("test-input.heic");
        assert_eq!(image.width(), 768);
        assert_eq!(image.height(), 1024);
    }

    #[test]
    #[ignore]
    fn read_heic_and_test_output() {
        let image = read_test_image("test-input.heic");
        assert_eq!(image.width(), 768);
        assert_eq!(image.height(), 1024);

        let writer = std::fs::File::create("test-output.jpeg").unwrap();
        write_image(&image, image::ImageFormat::Jpeg, writer).unwrap();
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
