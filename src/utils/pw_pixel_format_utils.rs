use pipewire::spa::param::video::VideoFormat;

use crate::utils::pixel_format_utils;

/// Convert YUV components (with U/V already offset by -128) to RGB 0..255 tuple
/// Written by AI
fn yuv_to_rgb_f(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    let r = (y + 1.402 * v).round().clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * u - 0.714136 * v).round().clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * u).round().clamp(0.0, 255.0) as u8;
    (r, g, b)
}

/// Convert RGB components to YUV (with U/V offset by +128)
/// Written by AI
fn rgb_to_yuv_f(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;

    let y = (0.299 * r + 0.587 * g + 0.114 * b)
        .round()
        .clamp(0.0, 255.0) as u8;
    let u = (-0.169 * r - 0.331 * g + 0.500 * b + 128.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    let v = (0.500 * r - 0.419 * g - 0.081 * b + 128.0)
        .round()
        .clamp(0.0, 255.0) as u8;

    (y, u, v)
}

/// Convert a buffer in `format` to tightly-packed RGB (3 bytes per pixel).
///
/// Note: This assumes the input buffer is tightly packed (no per-line stride/padding).
/// Written by AI
pub fn convert_to_rgb(format: VideoFormat, width: u32, height: u32, src: &[u8]) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let pixel_count = w.saturating_mul(h);

    match format {
        VideoFormat::RGB => {
            // Already RGB (3 bytes per pixel)
            let need = pixel_count * 3;
            if src.len() >= need {
                src[0..need].to_vec()
            } else {
                let mut v = src.to_vec();
                v.resize(need, 0u8);
                v
            }
        }
        VideoFormat::RGBA => {
            // RGBA -> drop A using shared utility function
            pixel_format_utils::rgba8_to_rgb(src, width, height)
        }
        VideoFormat::RGBx => {
            // 4 bytes per pixel, last ignored
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 3 < src.len() && out.len() / 3 < pixel_count {
                out.push(src[i]);
                out.push(src[i + 1]);
                out.push(src[i + 2]);
                i += 4;
            }
            out
        }
        VideoFormat::BGRx => {
            // B G R x -> R G B
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 3 < src.len() && out.len() / 3 < pixel_count {
                let b = src[i];
                let g = src[i + 1];
                let r = src[i + 2];
                out.push(r);
                out.push(g);
                out.push(b);
                i += 4;
            }
            out
        }
        VideoFormat::BGR => {
            // 3 bytes per pixel B,G,R -> R,G,B
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 2 < src.len() && out.len() / 3 < pixel_count {
                let b = src[i];
                let g = src[i + 1];
                let r = src[i + 2];
                out.push(r);
                out.push(g);
                out.push(b);
                i += 3;
            }
            out
        }
        VideoFormat::YUY2 => {
            // Packed YUYV: Y0 U Y1 V  -> two pixels
            let mut out = Vec::with_capacity(pixel_count * 3);
            let mut i = 0usize;
            while i + 3 < src.len() && out.len() / 3 < pixel_count {
                let y0 = src[i] as f32;
                let u = src[i + 1] as f32 - 128.0;
                let y1 = src[i + 2] as f32;
                let v = src[i + 3] as f32 - 128.0;
                let (r0, g0, b0) = yuv_to_rgb_f(y0, u, v);
                let (r1, g1, b1) = yuv_to_rgb_f(y1, u, v);
                out.push(r0);
                out.push(g0);
                out.push(b0);
                if out.len() / 3 < pixel_count {
                    out.push(r1);
                    out.push(g1);
                    out.push(b1);
                }
                i += 4;
            }
            // truncate if over
            out.truncate(pixel_count * 3);
            out
        }
        VideoFormat::I420 => {
            // Planar YUV420 (I420): Y plane (w*h), then U (w/2*h/2), then V (w/2*h/2)
            let y_plane_len = w.saturating_mul(h);
            let uv_w = w.div_ceil(2);
            let uv_h = h.div_ceil(2);
            let uv_plane_len = uv_w.saturating_mul(uv_h);
            if src.len() < y_plane_len + 2 * uv_plane_len {
                return vec![0u8; pixel_count * 3];
            }
            let y_plane = &src[0..y_plane_len];
            let u_plane = &src[y_plane_len..y_plane_len + uv_plane_len];
            let v_plane = &src[y_plane_len + uv_plane_len..y_plane_len + 2 * uv_plane_len];

            let mut out = Vec::with_capacity(pixel_count * 3);
            for yy in 0..h {
                for xx in 0..w {
                    let y = y_plane[yy * w + xx] as f32;
                    let ux = xx / 2;
                    let uy = yy / 2;
                    let u = u_plane[uy * uv_w + ux] as f32 - 128.0;
                    let v = v_plane[uy * uv_w + ux] as f32 - 128.0;
                    let (r, g, b) = yuv_to_rgb_f(y, u, v);
                    out.push(r);
                    out.push(g);
                    out.push(b);
                }
            }
            out
        }
        _ => {
            // Fallback: try to interpret as tightly-packed RGB triplets
            let need = pixel_count * 3;
            if src.len() >= need {
                src[0..need].to_vec()
            } else {
                let mut v = src.to_vec();
                v.resize(need, 0u8);
                v
            }
        }
    }
}

/// Convert a buffer in `format` to YUV420 (I420) planar format.
///
/// The output layout is: Y plane (width*height), U plane (width/2*height/2), V plane (width/2*height/2).
/// Note: This assumes the input buffer is tightly packed (no per-line stride/padding).
/// Written by AI
pub fn convert_to_yuv420(format: VideoFormat, width: u32, height: u32, src: &[u8]) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let y_plane_len = w.saturating_mul(h);
    let uv_w = w.div_ceil(2);
    let uv_h = h.div_ceil(2);
    let uv_plane_len = uv_w.saturating_mul(uv_h);
    let total_len = y_plane_len + 2 * uv_plane_len;

    match format {
        VideoFormat::I420 => {
            // Already YUV420 - just copy
            if src.len() >= total_len {
                src[0..total_len].to_vec()
            } else {
                let mut v = src.to_vec();
                v.resize(total_len, 128);
                v
            }
        }
        VideoFormat::RGB => {
            // RGB (3 bytes per pixel) -> YUV420
            let mut out = vec![0u8; total_len];

            // Convert Y plane
            for yy in 0..h {
                for xx in 0..w {
                    let idx = (yy * w + xx) * 3;
                    if idx + 2 < src.len() {
                        let r = src[idx];
                        let g = src[idx + 1];
                        let b = src[idx + 2];
                        let (y, _, _) = rgb_to_yuv_f(r, g, b);
                        out[yy * w + xx] = y;
                    }
                }
            }

            // Subsample UV planes (2x2 blocks)
            for yy in 0..uv_h {
                for xx in 0..uv_w {
                    let src_y = yy * 2;
                    let src_x = xx * 2;

                    // Sample 4 pixels (or fewer at edges)
                    let mut u_sum = 0u32;
                    let mut v_sum = 0u32;
                    let mut count = 0u32;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let py = src_y + dy;
                            let px = src_x + dx;
                            if py < h && px < w {
                                let idx = (py * w + px) * 3;
                                if idx + 2 < src.len() {
                                    let r = src[idx];
                                    let g = src[idx + 1];
                                    let b = src[idx + 2];
                                    let (_, u, v) = rgb_to_yuv_f(r, g, b);
                                    u_sum += u as u32;
                                    v_sum += v as u32;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        out[y_plane_len + yy * uv_w + xx] = (u_sum / count) as u8;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = (v_sum / count) as u8;
                    } else {
                        out[y_plane_len + yy * uv_w + xx] = 128;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = 128;
                    }
                }
            }
            out
        }
        VideoFormat::RGBA => {
            // RGBA (4 bytes per pixel) -> YUV420
            let mut out = vec![0u8; total_len];

            for yy in 0..h {
                for xx in 0..w {
                    let idx = (yy * w + xx) * 4;
                    if idx + 3 < src.len() {
                        let r = src[idx];
                        let g = src[idx + 1];
                        let b = src[idx + 2];
                        let (y, _, _) = rgb_to_yuv_f(r, g, b);
                        out[yy * w + xx] = y;
                    }
                }
            }

            for yy in 0..uv_h {
                for xx in 0..uv_w {
                    let src_y = yy * 2;
                    let src_x = xx * 2;
                    let mut u_sum = 0u32;
                    let mut v_sum = 0u32;
                    let mut count = 0u32;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let py = src_y + dy;
                            let px = src_x + dx;
                            if py < h && px < w {
                                let idx = (py * w + px) * 4;
                                if idx + 3 < src.len() {
                                    let r = src[idx];
                                    let g = src[idx + 1];
                                    let b = src[idx + 2];
                                    let (_, u, v) = rgb_to_yuv_f(r, g, b);
                                    u_sum += u as u32;
                                    v_sum += v as u32;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        out[y_plane_len + yy * uv_w + xx] = (u_sum / count) as u8;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = (v_sum / count) as u8;
                    } else {
                        out[y_plane_len + yy * uv_w + xx] = 128;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = 128;
                    }
                }
            }
            out
        }
        VideoFormat::RGBx => {
            // RGBx (4 bytes per pixel, last byte ignored) -> YUV420
            let mut out = vec![0u8; total_len];

            for yy in 0..h {
                for xx in 0..w {
                    let idx = (yy * w + xx) * 4;
                    if idx + 3 < src.len() {
                        let r = src[idx];
                        let g = src[idx + 1];
                        let b = src[idx + 2];
                        let (y, _, _) = rgb_to_yuv_f(r, g, b);
                        out[yy * w + xx] = y;
                    }
                }
            }

            for yy in 0..uv_h {
                for xx in 0..uv_w {
                    let src_y = yy * 2;
                    let src_x = xx * 2;
                    let mut u_sum = 0u32;
                    let mut v_sum = 0u32;
                    let mut count = 0u32;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let py = src_y + dy;
                            let px = src_x + dx;
                            if py < h && px < w {
                                let idx = (py * w + px) * 4;
                                if idx + 3 < src.len() {
                                    let r = src[idx];
                                    let g = src[idx + 1];
                                    let b = src[idx + 2];
                                    let (_, u, v) = rgb_to_yuv_f(r, g, b);
                                    u_sum += u as u32;
                                    v_sum += v as u32;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        out[y_plane_len + yy * uv_w + xx] = (u_sum / count) as u8;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = (v_sum / count) as u8;
                    } else {
                        out[y_plane_len + yy * uv_w + xx] = 128;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = 128;
                    }
                }
            }
            out
        }
        VideoFormat::BGRx => {
            // BGRx (B G R x) -> YUV420
            let mut out = vec![0u8; total_len];

            for yy in 0..h {
                for xx in 0..w {
                    let idx = (yy * w + xx) * 4;
                    if idx + 3 < src.len() {
                        let b = src[idx];
                        let g = src[idx + 1];
                        let r = src[idx + 2];
                        let (y, _, _) = rgb_to_yuv_f(r, g, b);
                        out[yy * w + xx] = y;
                    }
                }
            }

            for yy in 0..uv_h {
                for xx in 0..uv_w {
                    let src_y = yy * 2;
                    let src_x = xx * 2;
                    let mut u_sum = 0u32;
                    let mut v_sum = 0u32;
                    let mut count = 0u32;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let py = src_y + dy;
                            let px = src_x + dx;
                            if py < h && px < w {
                                let idx = (py * w + px) * 4;
                                if idx + 3 < src.len() {
                                    let b = src[idx];
                                    let g = src[idx + 1];
                                    let r = src[idx + 2];
                                    let (_, u, v) = rgb_to_yuv_f(r, g, b);
                                    u_sum += u as u32;
                                    v_sum += v as u32;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        out[y_plane_len + yy * uv_w + xx] = (u_sum / count) as u8;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = (v_sum / count) as u8;
                    } else {
                        out[y_plane_len + yy * uv_w + xx] = 128;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = 128;
                    }
                }
            }
            out
        }
        VideoFormat::BGR => {
            // BGR (3 bytes per pixel) -> YUV420
            let mut out = vec![0u8; total_len];

            for yy in 0..h {
                for xx in 0..w {
                    let idx = (yy * w + xx) * 3;
                    if idx + 2 < src.len() {
                        let b = src[idx];
                        let g = src[idx + 1];
                        let r = src[idx + 2];
                        let (y, _, _) = rgb_to_yuv_f(r, g, b);
                        out[yy * w + xx] = y;
                    }
                }
            }

            for yy in 0..uv_h {
                for xx in 0..uv_w {
                    let src_y = yy * 2;
                    let src_x = xx * 2;
                    let mut u_sum = 0u32;
                    let mut v_sum = 0u32;
                    let mut count = 0u32;

                    for dy in 0..2 {
                        for dx in 0..2 {
                            let py = src_y + dy;
                            let px = src_x + dx;
                            if py < h && px < w {
                                let idx = (py * w + px) * 3;
                                if idx + 2 < src.len() {
                                    let b = src[idx];
                                    let g = src[idx + 1];
                                    let r = src[idx + 2];
                                    let (_, u, v) = rgb_to_yuv_f(r, g, b);
                                    u_sum += u as u32;
                                    v_sum += v as u32;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        out[y_plane_len + yy * uv_w + xx] = (u_sum / count) as u8;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = (v_sum / count) as u8;
                    } else {
                        out[y_plane_len + yy * uv_w + xx] = 128;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = 128;
                    }
                }
            }
            out
        }
        VideoFormat::YUY2 => {
            // Packed YUYV: Y0 U Y1 V -> YUV420
            let mut out = vec![0u8; total_len];

            // Extract Y plane
            let mut pixel_idx = 0;
            let mut i = 0;
            while i + 3 < src.len() && pixel_idx + 1 < y_plane_len {
                out[pixel_idx] = src[i];
                out[pixel_idx + 1] = src[i + 2];
                pixel_idx += 2;
                i += 4;
            }

            // Subsample UV planes
            for yy in 0..uv_h {
                for xx in 0..uv_w {
                    let src_y = yy * 2;
                    let src_x = xx * 2;
                    let mut u_sum = 0u32;
                    let mut v_sum = 0u32;
                    let mut count = 0u32;

                    // Each YUY2 macropixel (4 bytes) covers 2 pixels horizontally
                    // Sample the corresponding macropixels
                    for dy in 0..2 {
                        let py = src_y + dy;
                        if py < h {
                            let px = src_x;
                            if px < w {
                                let macro_idx = (py * w + px) / 2;
                                let byte_idx = macro_idx * 4;
                                if byte_idx + 3 < src.len() {
                                    let u = src[byte_idx + 1];
                                    let v = src[byte_idx + 3];
                                    u_sum += u as u32;
                                    v_sum += v as u32;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        out[y_plane_len + yy * uv_w + xx] = (u_sum / count) as u8;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = (v_sum / count) as u8;
                    } else {
                        out[y_plane_len + yy * uv_w + xx] = 128;
                        out[y_plane_len + uv_plane_len + yy * uv_w + xx] = 128;
                    }
                }
            }
            out
        }
        _ => {
            // Fallback: return empty YUV420 buffer with neutral chroma
            let mut out = vec![0u8; total_len];
            out[y_plane_len..].fill(128);
            out
        }
    }
}
