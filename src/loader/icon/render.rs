use crate::utils::paths::get_cache_dir;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn render_svg_to_cache(key: &str, path: PathBuf) -> Option<Arc<Path>> {
    if !path.exists() {
        return None;
    }
    if path.extension().and_then(|e| e.to_str()) != Some("svg") {
        return Some(Arc::from(path.into_boxed_path()));
    }
    let svg_data = std::fs::read(&path).ok()?;
    render_to_png_cache(key, &svg_data)
}

pub fn render_to_png_cache(key: &str, svg_data: &[u8]) -> Option<Arc<Path>> {
    let mut out = get_cache_dir().ok()?.join("icons");
    std::fs::create_dir_all(&out).ok()?;
    out.push(format!("{}.png", key.replace('/', "_")));

    if out.exists() {
        return Some(Arc::from(out.into_boxed_path()));
    }

    let mut opt = usvg::Options::default();
    opt.shape_rendering = usvg::ShapeRendering::GeometricPrecision;
    opt.text_rendering = usvg::TextRendering::OptimizeLegibility;
    opt.image_rendering = usvg::ImageRendering::OptimizeQuality;

    let tree = usvg::Tree::from_data(svg_data, &opt)
        .map_err(|e| eprintln!("Failed to parse SVG {key}: {e}"))
        .ok()?;

    let svg_w = tree.size().width();
    let svg_h = tree.size().height();
    let render_size = (48.0_f32 * 2.0).max(64.0);
    let zoom = render_size / svg_w.max(svg_h);
    let width = (svg_w * zoom).round() as u32;
    let height = (svg_h * zoom).round() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height).or_else(|| {
        eprintln!("Failed to create pixmap for {key}");
        None
    })?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(zoom, zoom),
        &mut pixmap.as_mut(),
    );

    pixmap
        .save_png(&out)
        .map_err(|e| eprintln!("Failed to cache {key}: {e}"))
        .ok()?;

    Some(Arc::from(out.into_boxed_path()))
}
