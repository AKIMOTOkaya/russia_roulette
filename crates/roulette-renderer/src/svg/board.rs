//! Tactical board SVG renderer supporting arbitrary non-square grid dimensions.
//!
//! Uses pure procedural vector assets for terrain tiles, player combatant tokens,
//! and dynamic ballistic laser / displacement trails without platform emojis.

#![forbid(unsafe_code)]

use roulette_domain::{CellView, GameRecord, PlayerId, PlayerState, Position, Terrain};

use crate::assets::effects::EffectsRenderer;
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
        let gap = metrics.cell_gap;

        // Determine actual columns and rows dynamically from cells, falling back to map_size
        let max_x = cells.iter().map(|c| c.position.x).max().unwrap_or(0);
        let max_y = cells.iter().map(|c| c.position.y).max().unwrap_or(0);
        let cols = (max_x + 1).max(map_size);
        let rows = (max_y + 1).max(map_size);

        svg.push_str("  <!-- TACTICAL BOARD SECTION -->\n");
        svg.push_str("  <g id=\"tactical-board\">\n");

        // Board background panel
        let board_total_w =
            u32::from(cols) * cell_size + u32::from(cols.saturating_sub(1)) * gap + 40;
        let board_total_h =
            u32::from(rows) * cell_size + u32::from(rows.saturating_sub(1)) * gap + 40;
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" />\n",
            origin_x - 20,
            origin_y - 20,
            board_total_w,
            board_total_h,
            Theme::CARD_BG.to_svg_color(),
            Theme::CARD_BORDER.to_svg_color()
        ));

        // Coordinate headers (X axis at top)
        for x in 0..cols {
            let cx = origin_x + u32::from(x) * (cell_size + gap) + cell_size / 2;
            let cy = origin_y - 6;
            let col_label = (b'A' + x) as char;
            svg.push_str(&format!(
                "    <text x=\"{cx}\" y=\"{cy}\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">{col_label}</text>\n",
                Theme::TEXT_MUTED.to_svg_color()
            ));
        }

        // Coordinate headers (Y axis at left)
        for y in 0..rows {
            let cx = origin_x - 8;
            let cy = origin_y + u32::from(y) * (cell_size + gap) + cell_size / 2 + 4;
            svg.push_str(&format!(
                "    <text x=\"{cx}\" y=\"{cy}\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"end\">{}</text>\n",
                Theme::TEXT_MUTED.to_svg_color(),
                y + 1
            ));
        }

        // Grid cells
        for y in 0..rows {
            for x in 0..cols {
                let cell_pos = Position { x, y };
                let cell = cells.iter().find(|c| c.position == cell_pos);
                let terrain = cell.map_or(Terrain::Empty, |c| c.terrain);
                let px = origin_x + u32::from(x) * (cell_size + gap);
                let py = origin_y + u32::from(y) * (cell_size + gap);

                // Pure vector procedural terrain
                svg.push_str(&TerrainAssetRenderer::render(px, py, cell_size, terrain));

                // Living player occupancy
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

        let cell_size = custom_cell_size.unwrap_or(96);
        let metrics = BoardMetrics::standalone(cols, rows, cell_size);
        let total_w = metrics.total_width();
        let total_h = metrics.total_height();

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total_w}\" height=\"{total_h}\" viewBox=\"0 0 {total_w} {total_h}\">\n"
        );

        // Include defs (glow filters, gradients)
        svg.push_str(crate::assets::render_shared_defs());

        // Background tactical plate
        svg.push_str(&format!(
            "  <!-- MAP BACKGROUND -->\n  <rect width=\"{total_w}\" height=\"{total_h}\" fill=\"#0b0f19\" />\n"
        ));

        // Grid border plate
        let grid_x = metrics.padding - 12;
        let grid_y = metrics.padding - 12;
        let grid_w = total_w - grid_x * 2;
        let grid_h = total_h - grid_y * 2;
        svg.push_str(&format!(
            "  <rect x=\"{grid_x}\" y=\"{grid_y}\" width=\"{grid_w}\" height=\"{grid_h}\" rx=\"10\" fill=\"#0f172a\" stroke=\"#1e293b\" stroke-width=\"1.5\" />\n"
        ));

        // Coordinate rulers: Columns (top)
        for x in 0..cols {
            let cx = metrics.cell_x(x) + cell_size / 2;
            let cy = metrics.padding - 18;
            let col_label = (b'A' + x) as char;
            svg.push_str(&format!(
                "  <text x=\"{cx}\" y=\"{cy}\" font-size=\"14\" font-weight=\"900\" fill=\"#64748b\" text-anchor=\"middle\" letter-spacing=\"1\">{col_label}</text>\n"
            ));
        }

        // Coordinate rulers: Rows (left)
        for y in 0..rows {
            let cx = metrics.padding - 20;
            let cy = metrics.cell_y(y) + cell_size / 2 + 5;
            svg.push_str(&format!(
                "  <text x=\"{cx}\" y=\"{cy}\" font-size=\"14\" font-weight=\"900\" fill=\"#64748b\" text-anchor=\"end\">{}</text>\n",
                y + 1
            ));
        }

        // Layer 1: Procedural Terrain Grid
        svg.push_str("  <!-- TERRAIN LAYER -->\n  <g id=\"terrain-layer\">\n");
        for y in 0..rows {
            for x in 0..cols {
                let cell_pos = Position { x, y };
                let cell = cells.iter().find(|c| c.position == cell_pos);
                let terrain = cell.map_or(Terrain::Empty, |c| c.terrain);
                let px = metrics.cell_x(x);
                let py = metrics.cell_y(y);

                svg.push_str(&TerrainAssetRenderer::render(px, py, cell_size, terrain));
            }
        }
        svg.push_str("  </g>\n\n");

        // Layer 2: Player Combatant Tokens
        svg.push_str("  <!-- PLAYER LAYER -->\n  <g id=\"player-layer\">\n");
        let radius = cell_size / 2 - 10;
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

        // Layer 3: Dynamic Combat Effects (Ballistics, Lasers, Movement trails)
        svg.push_str("  <!-- COMBAT EFFECTS LAYER -->\n");
        svg.push_str(&EffectsRenderer::render_recent_records(
            records, &metrics, players,
        ));

        svg.push_str("</svg>\n");
        svg
    }
}
