use std::num::NonZeroU32;

use image::{imageops, DynamicImage};

pub struct ImageSizeTransform {
    /// Desired width of the scaled object
    pub width: Option<NonZeroU32>,
    /// Desired height of the scaled object
    pub height: Option<NonZeroU32>,

    /// Preserve aspect ratio, only checked if both width and height are provided.
    pub preserve_aspect_ratio: bool,
}

pub struct ImageSpec {
    pub size: ImageSizeTransform,
}

pub fn resize_image(input: &DynamicImage, transform: &ImageSizeTransform) -> DynamicImage {
    let tw = transform.width.map(|w| w.get());
    let th = transform.height.map(|h| h.get());

    let output = match (tw, th, transform.preserve_aspect_ratio) {
        (Some(tw), Some(th), false) => input.resize_exact(tw, th, imageops::FilterType::CatmullRom),
        (Some(tw), Some(th), true) => input.resize(tw, th, imageops::FilterType::CatmullRom),
        (Some(tw), None, _) => input.resize(tw, u32::MAX, imageops::FilterType::CatmullRom),
        (None, Some(th), _) => input.resize(u32::MAX, th, imageops::FilterType::CatmullRom),
        (None, None, _) => panic!("resize_image must take width or height"),
    };

    output
}
