//! Known-size calibration fixture shared by the native-image probe and its tests.
use image::{Rgba, RgbaImage};

/// An odd-sized 131x97 crop inside a 151x117 PNG exposes cell rounding/resizing errors.
/// A one-pixel white border surrounds coloured quadrants and central one-pixel stripes.
pub fn calibration_image() -> RgbaImage {
    let mut image = RgbaImage::new(151, 117);
    for y in 0_u32..97 {
        for x in 0_u32..131 {
            let color = if x == 0 || y == 0 || x == 130 || y == 96 {
                [255, 255, 255, 255]
            } else if (60..=70).contains(&x) {
                if x % 2 == 0 {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                }
            } else {
                match (x < 65, y < 48) {
                    (true, true) => [255, 0, 0, 255],
                    (false, true) => [0, 255, 0, 255],
                    (true, false) => [0, 0, 255, 255],
                    (false, false) => [255, 255, 0, 255],
                }
            };
            image.put_pixel(x.saturating_add(10), y.saturating_add(10), Rgba(color));
        }
    }
    image
}
