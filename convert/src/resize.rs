use image::{imageops, DynamicImage};

pub struct ImageSizeTransform {
    /// Desired width of the scaled object
    pub width: Option<u32>,
    /// Desired height of the scaled object
    pub height: Option<u32>,

    /// Preserve aspect ratio, only checked if both width and height are provided.
    pub preserve_aspect_ratio: bool,
}

pub struct ImageSpec {
    pub size: ImageSizeTransform,
}

/// Calculate target dimensions to resize an image, using the desired width and height as maximum
/// dimensions.
fn calculate_size(width: u32, height: u32, desired: &ImageSizeTransform) -> (u32, u32) {
    let tw = desired.width.map(|w| w as f64).unwrap_or(f64::MAX);
    let th = desired.height.map(|h| h as f64).unwrap_or(f64::MAX);

    let aspect = width as f64 / height as f64;

    // First try calculating the size based on the desired height and the aspect ratio. If that's too large
    // for the maximum width, then calculate it from the desired width instead.
    let width_from_height = th * aspect;
    let result = if width_from_height > tw {
        (tw, tw / aspect)
    } else {
        (width_from_height, th)
    };

    (result.0.round() as u32, result.1.round() as u32)
}

pub fn resize_image(input: &DynamicImage, transform: &ImageSizeTransform) -> Option<DynamicImage> {
    let tw = transform.width;
    let th = transform.height;
    let (iw, ih) = (input.width(), input.height());

    let (w, h) = match (tw, th, transform.preserve_aspect_ratio) {
        (Some(w), Some(h), false) => (w, h),
        // No resize requested. Allow this so we don't have to check explciitly for it everywhere.
        (None, None, _) => (iw, ih),
        _ => calculate_size(iw, ih, transform),
    };

    if w == iw && h == ih {
        None
    } else {
        Some(input.resize_exact(w, h, imageops::FilterType::CatmullRom))
    }
}

#[cfg(test)]
mod tests {
    use image::DynamicImage;

    use super::*;

    #[test]
    fn no_preserve_aspect() {
        let image = DynamicImage::new_rgb8(100, 100);
        let output = resize_image(
            &image,
            &ImageSizeTransform {
                width: Some(200),
                height: Some(125),
                preserve_aspect_ratio: false,
            },
        )
        .unwrap();

        assert_eq!(output.width(), 200, "width");
        assert_eq!(output.height(), 125, "height");
    }

    #[test]
    fn nothing_to_do() {
        let image = DynamicImage::new_rgb8(100, 100);
        let output = resize_image(
            &image,
            &ImageSizeTransform {
                width: Some(100),
                height: Some(100),
                preserve_aspect_ratio: false,
            },
        );

        assert!(output.is_none(), "Should return None");
    }

    #[test]
    fn no_resize_requested() {
        let image = DynamicImage::new_rgb8(100, 100);
        let output = resize_image(
            &image,
            &ImageSizeTransform {
                width: None,
                height: None,
                preserve_aspect_ratio: false,
            },
        );

        assert!(output.is_none(), "Should return None");
    }

    #[test]
    fn wh_same_size_max_width() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: Some(150),
                height: Some(200),
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (150, 100));
    }

    #[test]
    fn wh_same_size_max_height() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: Some(400),
                height: Some(100),
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (150, 100));
    }

    #[test]
    fn wh_limit_width() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: Some(400),
                height: Some(1000),
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (400, 267));
    }

    #[test]
    fn wh_limit_height() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: Some(4000),
                height: Some(200),
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (300, 200));
    }

    #[test]
    fn w_larger() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: Some(300),
                height: None,
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (300, 200));
    }

    #[test]
    fn h_larger() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: None,
                height: Some(400),
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (600, 400));
    }

    #[test]
    fn w_smaller() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: Some(100),
                height: None,
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (100, 67));
    }

    #[test]
    fn h_smaller() {
        let output = calculate_size(
            150,
            100,
            &ImageSizeTransform {
                width: None,
                height: Some(50),
                preserve_aspect_ratio: true,
            },
        );

        assert_eq!(output, (75, 50));
    }
}
