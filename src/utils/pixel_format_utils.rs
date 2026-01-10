/// Converts RGBA8 pixel data to RGB format by removing the alpha channel.
///
/// # Arguments
/// * `rgba_data` - Source buffer containing RGBA8 pixels (4 bytes per pixel)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
/// A new Vec<u8> containing RGB data (3 bytes per pixel)
///
/// Written by AI
pub fn rgba8_to_rgb(rgba_data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut rgb_data = Vec::with_capacity(pixel_count * 3);

    for i in 0..pixel_count {
        let rgba_offset = i * 4;
        rgb_data.push(rgba_data[rgba_offset]); // R
        rgb_data.push(rgba_data[rgba_offset + 1]); // G
        rgb_data.push(rgba_data[rgba_offset + 2]); // B
        // Skip alpha channel (rgba_data[rgba_offset + 3])
    }

    rgb_data
}
