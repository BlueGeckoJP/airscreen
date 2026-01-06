use pipewire::spa::param::video::VideoFormat;

/// Convert YUV components (with U/V already offset by -128) to RGB 0..255 tuple
/// Written by AI
fn yuv_to_rgb_f(y: f32, u: f32, v: f32) -> (u8, u8, u8) {
    let r = (y + 1.402 * v).round().clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * u - 0.714136 * v).round().clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * u).round().clamp(0.0, 255.0) as u8;
    (r, g, b)
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
            // RGBA -> drop A
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
