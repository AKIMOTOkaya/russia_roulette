//! Tactical board SVG renderer supporting arbitrary non-square grid dimensions.
//!
//! Uses pure procedural vector assets for terrain tiles, player combatant tokens,
//! and dynamic ballistic laser / displacement trails without platform emojis.

#![forbid(unsafe_code)]

use roulette_domain::{CellView, GameRecord, PlayerId, PlayerState, Position, Terrain};

use crate::assets::effects::EffectsRenderer;
use crate::assets::object::ObjectAssetRenderer;
use crate::assets::player::PlayerAssetRenderer;
use crate::assets::terrain::TerrainAssetRenderer;
use crate::theme::{BoardMetrics, LayoutMetrics, Theme};

/// Renders tactical board graphics (both standalone dynamic map and HUD embedded group).
pub struct BoardRenderer;

impl BoardRenderer {
    /// Renders an embedded board SVG group for full HUD cards.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render(
        map_size: u8,
        cells: &[CellView],
        players: &[PlayerState],
        current_player_id: Option<PlayerId>,
        metrics: &LayoutMetrics,
    ) -> String {
        let mut svg = String::with_capacity(8192);
        let origin_x = metrics.board_origin_x;
        let origin_y = metrics.board_origin_y;
        let cell_size = metrics.cell_size;

        // Determine actual columns and rows dynamically from cells, falling back to map_size
        let max_x = cells.iter().map(|c| c.position.x).max().unwrap_or(0);
        let max_y = cells.iter().map(|c| c.position.y).max().unwrap_or(0);
        let cols = (max_x + 1).max(map_size);
        let rows = (max_y + 1).max(map_size);

        svg.push_str("  <!-- TACTICAL BOARD SECTION -->\n");
        svg.push_str("  <g id=\"tactical-board\">\n");

        // Seamless board background plate (pure white with subtle border and soft shadow)
        let board_total_w = u32::from(cols) * cell_size;
        let board_total_h = u32::from(rows) * cell_size;
        svg.push_str(&format!(
            "    <rect x=\"{origin_x}\" y=\"{origin_y}\" width=\"{board_total_w}\" height=\"{board_total_h}\" rx=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" />\n",
            Theme::BOARD_BG.to_svg_color(),
            Theme::BOARD_BORDER.to_svg_color()
        ));

        // Grid cells & Terrains (Seamless Ground Tint Layer)
        svg.push_str("    <g id=\"terrain-layer\">\n");
        for y in 0..rows {
            for x in 0..cols {
                let cell_pos = Position { x, y };
                let cell = cells.iter().find(|c| c.position == cell_pos);
                let terrain = cell.map_or(Terrain::Plain, |c| c.terrain);
                let px = origin_x + u32::from(x) * cell_size;
                let py = origin_y + u32::from(y) * cell_size;

                svg.push_str(&TerrainAssetRenderer::render(px, py, cell_size, terrain));
            }
        }
        svg.push_str("    </g>\n");

        // Map Objects Layer (Walls, Crates, Mines, Shields)
        svg.push_str("    <g id=\"object-layer\">\n");
        for y in 0..rows {
            for x in 0..cols {
                let cell_pos = Position { x, y };
                let cell = cells.iter().find(|c| c.position == cell_pos);
                if let Some(Some(object)) = cell.map(|c| c.object) {
                    let px = origin_x + u32::from(x) * cell_size;
                    let py = origin_y + u32::from(y) * cell_size;
                    svg.push_str(&ObjectAssetRenderer::render(px, py, cell_size, object));
                }
            }
        }
        svg.push_str("    </g>\n");

        // Player Tokens
        svg.push_str("    <g id=\"player-layer\">\n");
        for y in 0..rows {
            for x in 0..cols {
                let cell_pos = Position { x, y };
                let px = origin_x + u32::from(x) * cell_size;
                let py = origin_y + u32::from(y) * cell_size;

                if let Some(player) = players.iter().find(|p| p.position == Some(cell_pos)) {
                    let cx = px + cell_size / 2;
                    let cy = py + cell_size / 2;
                    let radius = cell_size / 2 - 8;
                    svg.push_str(&PlayerAssetRenderer::render(
                        cx,
                        cy,
                        radius,
                        player,
                        current_player_id == Some(player.id),
                        None,
                    ));
                }
            }
        }
        svg.push_str("    </g>\n");

        svg.push_str("  </g>\n");
        svg
    }

    /// Renders a standalone, auto-sizing dynamic tactical board graphic.
    ///
    /// Dimension is computed purely from grid dimensions `(cols, rows)` without
    /// assuming a square aspect ratio.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render_standalone(
        cells: &[CellView],
        players: &[PlayerState],
        current_player_id: Option<PlayerId>,
        records: &[GameRecord],
        custom_cell_size: Option<u32>,
    ) -> String {
        let max_x = cells.iter().map(|c| c.position.x).max().unwrap_or(0);
        let max_y = cells.iter().map(|c| c.position.y).max().unwrap_or(0);
        let cols = (max_x + 1).max(1);
        let rows = (max_y + 1).max(1);

        let cell_size = custom_cell_size.unwrap_or(80);
        let metrics = BoardMetrics::standalone(cols, rows, cell_size);
        let total_w = metrics.total_width();
        let total_h = metrics.total_height();

        let grid_x = metrics.padding;
        let grid_y = metrics.padding;
        let grid_w = u32::from(cols) * cell_size;
        let grid_h = u32::from(rows) * cell_size;

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total_w}\" height=\"{total_h}\" viewBox=\"0 0 {total_w} {total_h}\">\n"
        );

        // Include styles and shared defs
        svg.push_str("  <defs>\n");
        svg.push_str("    <style>\n");
        svg.push_str("      text { font-family: 'MiSans', 'Noto Sans CJK SC', 'Source Han Sans SC', 'PingFang SC', 'Microsoft YaHei', 'DejaVu Sans', sans-serif; }\n");
        svg.push_str("    </style>\n");
        // Clip path for seamless board rounded boundaries
        svg.push_str(&format!(
            "    <clipPath id=\"board-clip\">\n      <rect x=\"{grid_x}\" y=\"{grid_y}\" width=\"{grid_w}\" height=\"{grid_h}\" rx=\"16\" />\n    </clipPath>\n"
        ));
        svg.push_str("  </defs>\n");
        svg.push_str(crate::assets::render_shared_defs());

        // Background canvas plate (Pure White)
        svg.push_str(&format!(
            "  <!-- MAP CANVAS BACKGROUND -->\n  <rect width=\"{total_w}\" height=\"{total_h}\" fill=\"{}\" />\n",
            Theme::CANVAS_BG.to_svg_color()
        ));

        // Seamless ground board plate (Whole board card with soft drop shadow)
        svg.push_str(&format!(
            "  <!-- SEAMLESS TACTICAL BOARD PLATE -->\n  <rect x=\"{grid_x}\" y=\"{grid_y}\" width=\"{grid_w}\" height=\"{grid_h}\" rx=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" filter=\"url(#fx-drop-shadow)\" />\n",
            Theme::BOARD_BG.to_svg_color(),
            Theme::BOARD_BORDER.to_svg_color()
        ));

        // Layer 1: Seamless Terrain Tint Layer (Clipped to board rounded corners, zero gap, no boxes)
        svg.push_str("  <!-- TERRAIN TINT LAYER -->\n  <g id=\"terrain-layer\" clip-path=\"url(#board-clip)\">\n");
        for y in 0..rows {
            for x in 0..cols {
                let cell_pos = Position { x, y };
                let cell = cells.iter().find(|c| c.position == cell_pos);
                let terrain = cell.map_or(Terrain::Plain, |c| c.terrain);
                let px = metrics.cell_x(x);
                let py = metrics.cell_y(y);

                svg.push_str(&TerrainAssetRenderer::render(px, py, cell_size, terrain));
            }
        }
        svg.push_str("  </g>\n\n");

        // Layer 2: Map Objects Layer (Cover, Traps, Pickups)
        svg.push_str("  <!-- MAP OBJECTS LAYER -->\n  <g id=\"object-layer\" clip-path=\"url(#board-clip)\">\n");
        for y in 0..rows {
            for x in 0..cols {
                let cell_pos = Position { x, y };
                let cell = cells.iter().find(|c| c.position == cell_pos);
                if let Some(Some(object)) = cell.map(|c| c.object) {
                    let px = metrics.cell_x(x);
                    let py = metrics.cell_y(y);
                    svg.push_str(&ObjectAssetRenderer::render(px, py, cell_size, object));
                }
            }
        }
        svg.push_str("  </g>\n\n");

        // Layer 2: Dynamic Combat Effects (Ballistics, Lasers, Movement trails)
        svg.push_str("  <!-- COMBAT EFFECTS LAYER -->\n  <g id=\"combat-effects-layer\">\n");
        svg.push_str(&EffectsRenderer::render_recent_records(
            records, &metrics, players,
        ));
        svg.push_str("  </g>\n\n");

        // Layer 3: Player Combatant Tokens
        svg.push_str("  <!-- PLAYER LAYER -->\n  <g id=\"player-layer\">\n");
        let radius = cell_size / 2 - 8;
        for player in players {
            if let Some(pos) = player.position {
                let (cx, cy) = metrics.cell_center(pos);
                let is_turn = current_player_id == Some(player.id);
                svg.push_str(&PlayerAssetRenderer::render(
                    cx, cy, radius, player, is_turn, None,
                ));
            }
        }
        svg.push_str("  </g>\n\n");

        svg.push_str("</svg>\n");
        svg
    }
}
