//! PNG rasterization pipeline using `resvg` and `tiny-skia`.

#![forbid(unsafe_code)]

use std::sync::{Arc, OnceLock};
use thiserror::Error;

/// Error encountered during SVG parsing or PNG rasterization.
#[derive(Debug, Error)]
pub enum RasterError {
    /// Failed to parse SVG markup into an abstract syntax tree.
    #[error("failed to parse SVG: {0}")]
    SvgParse(String),
    /// Failed to allocate raster pixmap memory.
    #[error("failed to allocate pixmap buffer")]
    PixmapAllocation,
    /// Failed to encode pixmap into PNG byte stream.
    #[error("failed to encode PNG: {0}")]
    PngEncode(String),
}

static FONT_DB: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();

fn get_font_database() -> Arc<resvg::usvg::fontdb::Database> {
    FONT_DB
        .get_or_init(|| {
            let mut db = resvg::usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

/// Rasterizes an SVG string into PNG image bytes.
///
/// # Errors
///
/// Returns `RasterError` if SVG is malformed or PNG encoding fails.
pub fn svg_to_png(svg_str: &str) -> Result<Vec<u8>, RasterError> {
    let fontdb = get_font_database();
    let opt = resvg::usvg::Options {
        fontdb,
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg_str, &opt)
        .map_err(|e| RasterError::SvgParse(e.to_string()))?;

    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or(RasterError::PixmapAllocation)?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );

    pixmap
        .encode_png()
        .map_err(|e| RasterError::PngEncode(e.to_string()))
}
