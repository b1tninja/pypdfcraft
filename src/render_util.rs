use pdfcraft_automation::ToolError;
use pdfcraft_render::{RenderConfig, RenderRequest, RequestKind};
use png::{BitDepth, ColorType, Encoder};

pub fn encode_png_rgba(width: u32, height: u32, premultiplied: &[u8]) -> Result<Vec<u8>, ToolError> {
    let mut rgba = premultiplied.to_vec();
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    let mut out = Vec::new();
    let mut enc = Encoder::new(&mut out, width, height);
    enc.set_color(ColorType::Rgba);
    enc.set_depth(BitDepth::Eight);
    let mut w = enc
        .write_header()
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    w.write_image_data(&rgba)
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    w.finish().map_err(|e| ToolError::Failed(e.to_string()))?;
    Ok(out)
}

pub fn encode_png_gray(width: u32, height: u32, premultiplied: &[u8]) -> Result<Vec<u8>, ToolError> {
    let mut gray = Vec::with_capacity((width * height) as usize);
    for px in premultiplied.as_chunks::<4>().0 {
        let r = u32::from(px[0]);
        let g = u32::from(px[1]);
        let b = u32::from(px[2]);
        let a = u32::from(px[3]);
        let lum = if a == 0 {
            255
        } else {
            ((r * 299 + g * 587 + b * 114 + 500) / 1000).min(255) as u8
        };
        gray.push(lum);
    }
    let mut out = Vec::new();
    let mut enc = Encoder::new(&mut out, width, height);
    enc.set_color(ColorType::Grayscale);
    enc.set_depth(BitDepth::Eight);
    let mut w = enc
        .write_header()
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    w.write_image_data(&gray)
        .map_err(|e| ToolError::Failed(e.to_string()))?;
    w.finish().map_err(|e| ToolError::Failed(e.to_string()))?;
    Ok(out)
}

pub fn render_page_png(
    display: &std::sync::Arc<Vec<u8>>,
    password: Option<&str>,
    page: usize,
    dpi: f64,
    colorspace: &str,
) -> Result<(Vec<u8>, u32, u32), ToolError> {
    use pdfcraft_render::PageRenderer;
    let config = RenderConfig {
        password: password.map(std::sync::Arc::from),
        ..Default::default()
    };
    let mut renderer = PageRenderer::new(display.clone(), config);
    let scale = (dpi / 72.0) as f32;
    let out = renderer.render(RenderRequest {
        page,
        kind: RequestKind::Pixels,
        tile: None,
        scale,
        tag: 0,
    });
    if let Some(e) = out.error {
        return Err(ToolError::Failed(format!(
            "page {} could not be rendered: {e}",
            page + 1
        )));
    }
    let data = match colorspace {
        "gray" | "grey" => encode_png_gray(out.width, out.height, &out.rgba)?,
        _ => encode_png_rgba(out.width, out.height, &out.rgba)?,
    };
    Ok((data, out.width, out.height))
}

pub fn dpi_for_max_side(width_pts: f32, height_pts: f32, max_side: u32) -> f64 {
    let long = width_pts.max(height_pts).max(1.0);
    (max_side as f64) * 72.0 / long as f64
}
