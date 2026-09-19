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

pub mod assets;
pub mod raster;
pub mod svg;
pub mod theme;

pub use assets::{EffectsRenderer, PlayerAssetRenderer, TerrainAssetRenderer, TileSheetRenderer};
pub use raster::RasterError;
pub use svg::{SvgComposer, board::BoardRenderer};
pub use theme::{BoardMetrics, LayoutMetrics, Theme};

use roulette_domain::{
    CellView, GameRecord, GameView, PlayerId, PlayerKind, PlayerState, RefereeRoomView, Terrain,
};
use thiserror::Error;

/// High-level errors from the rendering engine.
#[derive(Debug, Error)]
pub enum RenderError {
    /// Rasterization into PNG failed.
    #[error("raster error: {0}")]
    Raster(#[from] RasterError),
}

/// Helper function to convert any SVG string into high-resolution PNG bytes.
///
/// # Errors
/// Returns `RenderError` if rasterization fails.
pub fn render_png_from_svg(svg: &str) -> Result<Vec<u8>, RenderError> {
    raster::svg_to_png(svg).map_err(RenderError::Raster)
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
/// Returns `RenderError` if rasterization fails.
pub fn render_match_png(room: &RefereeRoomView) -> Result<Vec<u8>, RenderError> {
    let svg = render_match_svg(room);
    render_png_from_svg(&svg)
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
/// Returns `RenderError` if rasterization fails.
pub fn render_game_png(
    game: &GameView,
    room_name: &str,
    room_id: &str,
) -> Result<Vec<u8>, RenderError> {
    let svg = render_game_svg(game, room_name, room_id);
    render_png_from_svg(&svg)
}

/// Renders a dynamic standalone tactical board SVG without HUD cards or rosters.
///
/// Computes width and height dynamically based on grid dimensions `(cols, rows)`
/// without assuming a square aspect ratio.
#[must_use]
pub fn render_board_svg(
    cells: &[CellView],
    players: &[PlayerState],
    current_player_id: Option<PlayerId>,
    records: &[GameRecord],
    cell_size: Option<u32>,
) -> String {
    BoardRenderer::render_standalone(cells, players, current_player_id, records, cell_size)
}

/// Renders a dynamic standalone tactical board into PNG image bytes.
///
/// # Errors
/// Returns `RenderError` if rasterization fails.
pub fn render_board_png(
    cells: &[CellView],
    players: &[PlayerState],
    current_player_id: Option<PlayerId>,
    records: &[GameRecord],
    cell_size: Option<u32>,
) -> Result<Vec<u8>, RenderError> {
    let svg = render_board_svg(cells, players, current_player_id, records, cell_size);
    render_png_from_svg(&svg)
}

/// Renders the complete vector asset catalog / tilesheet as an SVG document.
#[must_use]
pub fn render_tilesheet_svg() -> String {
    TileSheetRenderer::render_svg()
}

/// Renders the complete vector asset catalog / tilesheet as PNG image bytes.
///
/// # Errors
/// Returns `RenderError` if rasterization fails.
pub fn render_tilesheet_png() -> Result<Vec<u8>, RenderError> {
    TileSheetRenderer::render_png()
}

/// Renders a single square terrain tile as a standalone SVG document.
#[must_use]
pub fn render_single_tile_svg(terrain: Terrain, cell_size: u32) -> String {
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{cell_size}\" height=\"{cell_size}\" viewBox=\"0 0 {cell_size} {cell_size}\">\n"
    );
    svg.push_str(assets::render_shared_defs());
    svg.push_str(&TerrainAssetRenderer::render(0, 0, cell_size, terrain));
    svg.push_str("</svg>\n");
    svg
}

/// Renders a single combatant token as a standalone SVG document.
#[must_use]
pub fn render_single_player_svg(
    player_id: PlayerId,
    kind: PlayerKind,
    is_turn: bool,
    has_shield: bool,
    size: u32,
) -> String {
    PlayerAssetRenderer::render_standalone(player_id, kind, is_turn, has_shield, size)
}
