// Copyright (c) 2026 Jef Wagner <jefwagner@gmail.com>
// SPDX-License-Identifier: MIT

//! Pure sizing math for termviewer.
//!
//! Everything in this module is headless-testable: given the source-image
//! dimensions, the target box in pixels, and any explicit `--width` /
//! `--height` overrides, it computes the canvas size and the scaled image
//! display size. No terminal or image decoding happens here.

/// Clamp a requested crop window to the bounds of the source image.
///
/// The crop is defined on the *unscaled* source image, so this runs before
/// any scaling. Coordinates past the image edge are pulled in, and the
/// width/height are clipped so the window stays inside the image.
/// Returns `None` if the clamped window is empty.
pub(crate) fn clamp_crop(
    crop: (u32, u32, u32, u32),
    img: (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    let (cx, cy, cw, ch) = crop;
    let (iw, ih) = img;
    let x = cx.min(iw);
    let y = cy.min(ih);
    let w = cw.min(iw - x);
    let h = ch.min(ih - y);
    if w == 0 || h == 0 {
        None
    } else {
        Some((x, y, w, h))
    }
}

/// Bump a pixel height up to the next even number (one char row = two
/// pixels), never below 2.
fn even_up(h: u32) -> u32 {
    let h = (h + 1) & !1;
    h.max(2)
}

/// Dimensions of the image scaled by `scale`, rounded, height bumped even.
fn dims_at_scale(img: (u32, u32), scale: f64) -> (u32, u32) {
    let w = (((img.0 as f64) * scale).round() as u32).max(1);
    let h = (((img.1 as f64) * scale).round() as u32).max(1);
    (w, even_up(h))
}

/// Contain-fit dimensions: preserve the image aspect ratio and fit inside
/// `box_px`. Upscaling is allowed — the caller decides whether to cap at 1.
fn fit_dims(img: (u32, u32), box_px: (u32, u32)) -> (u32, u32) {
    let scale = f64::min(
        box_px.0 as f64 / img.0.max(1) as f64,
        box_px.1 as f64 / img.1.max(1) as f64,
    );
    dims_at_scale(img, scale)
}

/// The complete sizing decision for one run.
///
/// Returns `(canvas_px, image_px)`:
///
/// - `canvas_px` — the `TerminalCanvas` size in pixels (height is even).
/// - `image_px` — the size the (already cropped) image is scaled to.
///
/// `explicit` is `Some((w, h))` when **both** `--width` and `--height` were
/// given: the canvas is then exactly that size and the image is contain-fit
/// inside it, upscaling allowed (an explicit size overrides the no-upscale
/// default). `allow_upscale` (any size flag given) lifts the no-upscale cap
/// in the default branch; with no flags, a smaller-than-box image is shown
/// at its native size.
pub(crate) fn compute_display_size(
    img: (u32, u32),
    box_px: (u32, u32),
    explicit: Option<(u32, u32)>,
    allow_upscale: bool,
) -> ((u32, u32), (u32, u32)) {
    match explicit {
        Some((w, h)) => {
            let canvas = (w.max(1), even_up(h));
            (canvas, fit_dims(img, canvas))
        }
        None => {
            let scale = f64::min(
                box_px.0.max(1) as f64 / img.0.max(1) as f64,
                box_px.1.max(1) as f64 / img.1.max(1) as f64,
            );
            let scale = if allow_upscale { scale } else { scale.min(1.0) };
            let image = dims_at_scale(img, scale);
            (image, image)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_small_image_shown_without_scaling() {
        let (canvas, image) = compute_display_size((80, 40), (200, 198), None, false);
        assert_eq!(canvas, (80, 40));
        assert_eq!(image, (80, 40));
    }

    #[test]
    fn test_large_image_scaled_down_to_fit_box() {
        // 400x200 in a 100x198 box: width binds → 100x50
        let (canvas, image) = compute_display_size((400, 200), (100, 198), None, false);
        assert_eq!(image, (100, 50));
        assert_eq!(canvas, (100, 50));
    }

    #[test]
    fn test_no_upscale_without_size_flags() {
        // Shown at native size; the odd pixel height still bumps to even
        // (the canvas needs whole char rows — a half-row of canvas black
        // shows at the bottom).
        let (_, image) = compute_display_size((50, 25), (100, 198), None, false);
        assert_eq!(image, (50, 26));
    }

    #[test]
    fn test_upscale_allowed_with_a_size_flag() {
        // One flag (-w) lifts the no-upscale cap; width binds → 100x50
        let (_, image) = compute_display_size((50, 25), (100, 198), None, true);
        assert_eq!(image, (100, 50));
    }

    #[test]
    fn test_explicit_size_canvas_exact_and_image_contained() {
        // 100x50 into a 120x80 canvas: min(1.2, 1.6) = 1.2 → 120x60
        let (canvas, image) = compute_display_size((100, 50), (80, 40), Some((120, 80)), true);
        assert_eq!(canvas, (120, 80));
        assert_eq!(image, (120, 60));
    }

    #[test]
    fn test_odd_explicit_height_bumped_to_even() {
        let (canvas, _) = compute_display_size((10, 10), (80, 40), Some((30, 41)), true);
        assert_eq!(canvas, (30, 42));
    }

    #[test]
    fn test_odd_display_height_bumped_to_even() {
        // 30x45 at scale 1.0 → height 45 rounds up to 46
        let (canvas, _) = compute_display_size((30, 45), (100, 198), None, false);
        assert_eq!(canvas, (30, 46));
    }

    #[test]
    fn test_crop_clamped_to_image_bounds() {
        assert_eq!(clamp_crop((5, 5, 10, 10), (12, 12)), Some((5, 5, 7, 7)));
        assert_eq!(clamp_crop((0, 0, 100, 8), (50, 8)), Some((0, 0, 50, 8)));
        assert_eq!(clamp_crop((0, 0, 4, 4), (12, 12)), Some((0, 0, 4, 4)));
        // Origin past the image edge leaves an empty window
        assert_eq!(clamp_crop((10, 10, 5, 5), (8, 8)), None);
    }
}
