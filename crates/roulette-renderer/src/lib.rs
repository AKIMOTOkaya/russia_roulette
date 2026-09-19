//! Rule-based image synthesis and tactical card rendering engine for Russian Roulette.
//!
//! This crate sits strictly in the presentation / output layer. It converts
//! authoritative domain views into crisp, high-fidelity SVG vector graphics and
//! raster PNG images for Web clients, bots, and IM chat endpoints.
//!
//! It does NOT use LLMs or generative models. All visual layouts, tactical grids,
//! terrain attributes, and HUD badges are rendered deterministically from
//! game rules and domain contracts.

#![forbid(unsafe_code)]
#![allow(
    clippy::format_push_string,
    clippy::cast_possible_truncation,
    clippy::uninlined_format_args,
    clippy::too_many_arguments
)]

pub mod raster;
pub mod svg;
pub mod theme;

pub use raster::RasterError;
pub use svg::SvgComposer;
pub use theme::{LayoutMetrics, Theme};

use roulette_domain::{GameView, RefereeRoomView};
use thiserror::Error;

/// High-level errors from the rendering engine.
#[derive(Debug, Error)]
pub enum RenderError {
    /// Rasterization into PNG failed.
    #[error("raster error: {0}")]
    Raster(#[from] RasterError),
}

/// Renders an omniscient referee room snapshot into SVG markup.
#[must_use]
pub fn render_match_svg(room: &RefereeRoomView) -> String {
    let metrics = LayoutMetrics::default();
    SvgComposer::compose_referee_room(room, &metrics)
}

/// Renders an omniscient referee room snapshot into PNG image bytes.
///
/// # Errors
///
/// Returns `RenderError` if rasterization fails.
pub fn render_match_png(room: &RefereeRoomView) -> Result<Vec<u8>, RenderError> {
    let svg = render_match_svg(room);
    raster::svg_to_png(&svg).map_err(RenderError::Raster)
}

/// Renders a public player game view into SVG markup.
#[must_use]
pub fn render_game_svg(game: &GameView, room_name: &str, room_id: &str) -> String {
    let metrics = LayoutMetrics::default();
    SvgComposer::compose_game_view(game, room_name, room_id, &metrics)
}

/// Renders a public player game view into PNG image bytes.
///
/// # Errors
///
/// Returns `RenderError` if rasterization fails.
pub fn render_game_png(
    game: &GameView,
    room_name: &str,
    room_id: &str,
) -> Result<Vec<u8>, RenderError> {
    let svg = render_game_svg(game, room_name, room_id);
    raster::svg_to_png(&svg).map_err(RenderError::Raster)
}
