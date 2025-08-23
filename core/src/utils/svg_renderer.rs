//! # User Badge SVG Renderer
//!
//! This module provides functionality to render user badges to PNG format.
//! It uses the `resvg` crate for high-quality SVG rendering with a predefined template.
//!

use fontdb::Database;
use resvg::{tiny_skia, usvg};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SvgRenderError {
    #[error("Failed to parse SVG: {0}")]
    SvgParseError(String),
    #[error("Failed to create pixmap")]
    PixmapCreationError,
    #[error("Failed to save PNG: {0}")]
    PngSaveError(String),
}

/// Calculate dynamic box width based on text length
/// Increases box width for longer text to ensure it fits comfortably
fn calculate_box_width(text: &str) -> f32 {
    let base_width = 29.0;
    let base_chars = 2;
    let char_width = 6.5;

    if text.len() <= base_chars {
        base_width
    } else {
        base_width + ((text.len() - base_chars) as f32 * char_width)
    }
}

fn get_box_width(text: &str, fontdb: std::sync::Arc<Database>) -> Result<f32, SvgRenderError> {
    // Create a minimal SVG just for text measurement
    let measurement_svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg">
            <text font-family="Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
                  font-size="11.606"
                  font-weight="600"
                  letter-spacing="0.05em">{text}</text>
        </svg>"#
    );

    let usvg_options = usvg::Options {
        fontdb,
        ..Default::default()
    };

    let padding = 13.0;
    match usvg::Tree::from_str(&measurement_svg, &usvg_options) {
        Ok(tree) => {
            // Use the tree's bounding box instead of searching for text nodes
            let bbox = tree.root().abs_bounding_box();
            Ok(bbox.width() + padding)
        }
        Err(_) => {
            // Fallback to improved estimation
            Err(SvgRenderError::SvgParseError(
                "Failed to parse SVG".to_string(),
            ))
        }
    }
}

/// Renders a user avatar badge to PNG data using a predefined SVG template
///
/// This function uses a specific SVG template that creates a speech bubble design
/// with customizable color and name text.
///
/// # Arguments
///
/// * `color` - Hex color code (e.g., "#FF5733" or "red") for the badge background
/// * `name` - Name text to display in the badge
///
/// # Returns
///
/// Returns `Ok(Vec<u8>)` containing PNG data on success or `Err(SvgRenderError)` on failure
pub fn render_user_badge_to_png(
    color: &str,
    name: &str,
    pointer: bool,
) -> Result<Vec<u8>, SvgRenderError> {
    // Calculate dynamic box width based on text length
    let scale_factor = 3;
    let filter_width = 120;
    let view_box_width = 200;
    let view_box_height = 60;

    // Create font database
    let mut fontdb = Database::new();
    fontdb.load_system_fonts();
    let fontdb = std::sync::Arc::new(fontdb);

    let mut box_width = if let Ok(width) = get_box_width(name, fontdb.clone()) {
        width
    } else {
        log::error!("Failed to get box width for name: {name} using fallback");
        calculate_box_width(name)
    };

    let mut name = name.to_string();
    /* This might not work perfectly for every name. */
    if box_width > 152.0 {
        box_width = 152.0;
        name = name.chars().take(17).collect::<String>() + "...";
    };

    // Choose SVG template based on pointer flag
    let svg_template = if pointer {
        // Pointer template
        format!(
            r#"<svg width="100%" height="100%" viewBox="{view_port_x} {view_port_y} {view_box_width} {view_box_height}" fill="none" xmlns="http://www.w3.org/2000/svg">
<g filter="url(#filter0_d_3690_153)" transform="scale({scale_factor_pointer}) rotate(-30)">
<path fill="{color}" d="M-368.99-226.1v-9h-4v-8h-4v-8h-4v-9h-5v-4h-4v-8h9v4h4v12h4v-54h8v38h4v-17h9v17h4v-13h8v17h4v-13h5v5h4v29h-4v12h-5v9z" transform="translate(94.337 75.2)scale(.23944)"/>
  <path fill="white" d="M-372.99-222.1v-13h-4v-8h-4v-8h-5v-9h-4v-4h-4v-12h13v4h4v-38h4v-4h8v4h4v17h9v4h12v4h9v4h4v5h4v29h-4v12h-4v13zm37-4v-9h5v-12h4v-29h-4v-5h-5v13h-4v-17h-8v13h-4v-17h-9v17h-4v-38h-8v54h-4v-12h-4v-4h-9v8h4v4h5v9h4v8h4v8h4v9z" transform="translate(94.337 75.2)scale(.23944)"/>
</g>
<g filter="url(#filter1_d_3690_153)" transform="scale({scale_factor}) translate(14, 0)">
<rect x="16.8486" y="22" width="{box_width}" height="21.9191" rx="10.9596" fill="{color}" shape-rendering="crispEdges"/>
<rect x="17.2022" y="22.5645" width="{box_width}" height="21.2121" rx="10.606" stroke="black" stroke-opacity="0.05" stroke-width="0.707069" shape-rendering="crispEdges"/>
<text fill="white" xml:space="preserve" style="white-space: pre" font-family="Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif" font-size="11.606" font-weight="600" letter-spacing="0.05em"><tspan x="22.9243" y="37.0946">{name}</tspan></text>
</g>
<defs>
<filter id="filter0_d_3690_153" x="0" y="0" width="24.8572" height="28.0661" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
<feFlood flood-opacity="0" result="BackgroundImageFix" />
<feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha"/>
<feOffset dy="0.707069"/>
<feGaussianBlur stdDeviation="1.0606"/>
<feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.35 0"/>
<feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_3690_153"/>
<feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_3690_153" result="shape"/>
</filter>
<filter id="filter1_d_3690_153" x="6.24259" y="11.6049" width="{filter_width}" height="43.131" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
<feFlood flood-opacity="0" result="BackgroundImageFix"/>
<feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha"/>
<feOffset/>
<feGaussianBlur stdDeviation="5.30302"/>
<feComposite in2="hardAlpha" operator="out"/>
<feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.1 0"/>
<feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_3690_153"/>
<feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_3690_153" result="shape"/>
</filter>
</defs>
</svg>"#,
            scale_factor_pointer = 1.5 * (scale_factor as f32),
            view_port_x = 5 * scale_factor,
            view_port_y = -6 * scale_factor,
            color = color,
            scale_factor = scale_factor,
            name = name,
            box_width = box_width,
            view_box_width = view_box_width * scale_factor,
            view_box_height = view_box_height * scale_factor,
            filter_width = filter_width * scale_factor,
        )
    } else {
        // Regular template with dynamic dimensions
        format!(
            r#"<svg width="100%" height="100%" viewBox="0 0 {view_box_width} {view_box_height}" fill="none" xmlns="http://www.w3.org/2000/svg">
<g filter="url(#filter0_d_3690_153)" transform="scale({scale_factor})">
<path d="M9.21246 25.5608C8.52706 26.643 6.87099 26.3292 6.62908 25.0712L2.59236 4.08025C2.3623 2.88395 3.65167 1.97598 4.70046 2.59573L22.485 13.1048C23.5565 13.7379 23.3466 15.3466 22.1485 15.6836L14.542 17.8229C14.206 17.9174 13.9169 18.1328 13.7302 18.4276L9.21246 25.5608Z" fill="{color}"/>
<path d="M2.93945 4.01367C2.76691 3.11645 3.73391 2.43558 4.52051 2.90039L22.3047 13.4092C23.1083 13.884 22.9512 15.09 22.0527 15.3428L14.4463 17.4824C14.0264 17.6005 13.665 17.8698 13.4316 18.2383L8.91406 25.3721C8.40002 26.1834 7.15831 25.9479 6.97656 25.0049L2.93945 4.01367Z" stroke="white" stroke-opacity="0.4" stroke-width="0.707069"/>
</g>
<g filter="url(#filter1_d_3690_153)" transform="scale({scale_factor})">
<rect x="16.8486" y="22.2109" width="{box_width}" height="21.9191" rx="10.9596" fill="{color}" shape-rendering="crispEdges"/>
<rect x="17.2022" y="22.5645" width="{box_width}" height="21.2121" rx="10.606" stroke="black" stroke-opacity="0.05" stroke-width="0.707069" shape-rendering="crispEdges"/>
<text fill="white" xml:space="preserve" style="white-space: pre" font-family="Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif" font-size="11.606" font-weight="600" letter-spacing="0.05em"><tspan x="22.9243" y="37.0946">{name}</tspan></text>
</g>
<defs>
<filter id="filter0_d_3690_153" x="0.444222" y="0.981369" width="24.8572" height="28.0661" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
<feFlood flood-opacity="0" result="BackgroundImageFix"/>
<feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha"/>
<feOffset dy="0.707069"/>
<feGaussianBlur stdDeviation="1.0606"/>
<feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.35 0"/>
<feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_3690_153"/>
<feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_3690_153" result="shape"/>
</filter>
<filter id="filter1_d_3690_153" x="6.24259" y="11.6049" width="{filter_width}" height="43.131" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB">
<feFlood flood-opacity="0" result="BackgroundImageFix"/>
<feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha"/>
<feOffset/>
<feGaussianBlur stdDeviation="5.30302"/>
<feComposite in2="hardAlpha" operator="out"/>
<feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.1 0"/>
<feBlend mode="normal" in2="BackgroundImageFix" result="effect1_dropShadow_3690_153"/>
<feBlend mode="normal" in="SourceGraphic" in2="effect1_dropShadow_3690_153" result="shape"/>
</filter>
</defs>
</svg>"#,
            scale_factor = scale_factor,
            box_width = box_width,
            color = color,
            name = name,
            view_box_width = view_box_width * scale_factor,
            view_box_height = view_box_height * scale_factor,
            filter_width = filter_width * scale_factor,
        )
    };

    // Parse the SVG with font database
    let usvg_options = usvg::Options {
        fontdb,
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg_template, &usvg_options)
        .map_err(|e| SvgRenderError::SvgParseError(e.to_string()))?;

    // Get the SVG size
    let svg_size = tree.size();
    let width = svg_size.width() as u32;
    let height = svg_size.height() as u32;

    // Create a pixmap to render into
    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or(SvgRenderError::PixmapCreationError)?;

    // Render the SVG
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    // Encode as PNG and return the data
    pixmap
        .encode_png()
        .map_err(|e| SvgRenderError::PngSaveError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_user_badge_to_png() {
        let png_data = render_user_badge_to_png("#FF5733", "Alice", false).unwrap();

        // Verify it's valid PNG data by checking PNG signature
        assert_eq!(&png_data[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

        // Should have some reasonable size (not empty)
        assert!(png_data.len() > 100);

        // Test with different parameters
        let png_data2 = render_user_badge_to_png("#00FF00", "Bob Doe", false).unwrap();
        assert_eq!(&png_data2[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(png_data2.len() > 100);

        // The two images should be different (different color/name)
        assert_ne!(png_data, png_data2);
    }

    #[test]
    fn test_calculate_box_width() {
        // Short names should use base width
        assert_eq!(calculate_box_width("John"), 40.0);
        assert_eq!(calculate_box_width("Alice"), 40.0);

        // Longer names should have increased width
        let long_width = calculate_box_width("Alice & Bob");
        assert!(long_width > 40.0);

        // Very long names should have proportionally wider boxes
        let very_long_width = calculate_box_width("Very Long Username");
        assert!(very_long_width > long_width);

        // Test specific calculations
        assert_eq!(calculate_box_width("1234567"), 40.0 + 6.5); // 7 chars = +1 char * 6.5px
    }

    #[test]
    fn test_different_name_lengths() {
        // Test badges with different name lengths (now with dynamic box width)
        let very_short_badge = render_user_badge_to_png("#9FB8E8", "Me", false).unwrap();
        let short_badge = render_user_badge_to_png("#9FB8E8", "Joe", false).unwrap();
        let medium_badge = render_user_badge_to_png("#9FB8E8", "Alice Doe", false).unwrap();
        let long_badge = render_user_badge_to_png("#9FB8E8", "Iason Parask", false).unwrap();
        let extra_long_badge =
            render_user_badge_to_png("#9FB8E8", "AlexanderGGGGGGGGGGG", false).unwrap();
        let extra_long_badge_two =
            render_user_badge_to_png("#9FB8E8", "Lykourgos Mpezentakos", false).unwrap();

        // All should generate valid PNG data
        assert_eq!(&short_badge[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert_eq!(&medium_badge[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert_eq!(&long_badge[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert_eq!(&extra_long_badge[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

        // Save examples for visual inspection
        std::fs::write("very_short_name_wide_badge.png", very_short_badge).unwrap();
        std::fs::write("short_name_wide_badge.png", short_badge).unwrap();
        std::fs::write("medium_name_wide_badge.png", medium_badge).unwrap();
        std::fs::write("long_name_wide_badge.png", long_badge).unwrap();
        std::fs::write("extra_long_name_wide_badge.png", extra_long_badge).unwrap();
        std::fs::write("extra_long_name_wide_badge_two.png", extra_long_badge_two).unwrap();
    }

    #[test]
    fn test_pointer_badge() {
        // Test the pointer template
        let pointer_badge = render_user_badge_to_png("#FF5733", "Costa", true).unwrap();

        // Verify it's valid PNG data by checking PNG signature
        assert_eq!(&pointer_badge[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);

        // Should have some reasonable size (not empty)
        assert!(pointer_badge.len() > 100);

        // Test regular badge for comparison
        let regular_badge = render_user_badge_to_png("#FF5733", "Costa", false).unwrap();

        // The two images should be different (different templates)
        assert_ne!(pointer_badge, regular_badge);

        // Save example for visual inspection
        std::fs::write("test_pointer_badge.png", pointer_badge).unwrap();
    }
}
