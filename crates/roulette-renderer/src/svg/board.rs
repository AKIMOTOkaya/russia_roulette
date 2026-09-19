//! Tactical 5x5 board SVG renderer.

#![forbid(unsafe_code)]

use roulette_domain::{CellView, PlayerId, PlayerKind, PlayerState, Position, Terrain};

use crate::theme::{Color, LayoutMetrics, Theme};

/// Renders the 5x5 tactical board SVG group.
pub struct BoardRenderer;

impl BoardRenderer {
    /// Generates SVG markup for the board grid, coordinates, terrain tiles, and players.
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

        svg.push_str("  <!-- TACTICAL BOARD SECTION -->\n");
        svg.push_str("  <g id=\"tactical-board\">\n");

        // Board background panel
        let board_total_w = u32::from(map_size) * cell_size + (u32::from(map_size) - 1) * gap + 40;
        let board_total_h = u32::from(map_size) * cell_size + (u32::from(map_size) - 1) * gap + 40;
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
        for x in 0..map_size {
            let cx = origin_x + u32::from(x) * (cell_size + gap) + cell_size / 2;
            let cy = origin_y - 6;
            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">X{}</text>\n",
                cx, cy, Theme::TEXT_MUTED.to_svg_color(), x
            ));
        }

        // Coordinate headers (Y axis at left)
        for y in 0..map_size {
            let cx = origin_x - 8;
            let cy = origin_y + u32::from(y) * (cell_size + gap) + cell_size / 2 + 4;
            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"end\">Y{}</text>\n",
                cx, cy, Theme::TEXT_MUTED.to_svg_color(), y
            ));
        }

        // Grid cells
        for y in 0..map_size {
            for x in 0..map_size {
                let cell_pos = Position { x, y };
                let cell = cells.iter().find(|c| c.position == cell_pos);
                let terrain = cell.map_or(Terrain::Empty, |c| c.terrain);
                let px = origin_x + u32::from(x) * (cell_size + gap);
                let py = origin_y + u32::from(y) * (cell_size + gap);

                let pal = Theme::terrain_palette(terrain);

                // Cell base rectangle
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" />\n",
                    px, py, cell_size, cell_size, pal.fill.to_svg_color(), pal.stroke.to_svg_color()
                ));

                // Terrain emblems and rule markings
                Self::render_terrain_emblem(&mut svg, px, py, cell_size, terrain, pal.emblem);

                // Living player occupancy
                if let Some(player) = players.iter().find(|p| p.position == Some(cell_pos)) {
                    Self::render_player_token(
                        &mut svg,
                        px,
                        py,
                        cell_size,
                        player,
                        current_player_id == Some(player.id),
                    );
                }
            }
        }

        svg.push_str("  </g>\n");
        svg
    }

    fn render_terrain_emblem(
        svg: &mut String,
        px: u32,
        py: u32,
        cell_size: u32,
        terrain: Terrain,
        emblem_color: Color,
    ) {
        let center_x = px + cell_size / 2;
        let center_y = py + cell_size / 2;
        let col = emblem_color.to_svg_color();

        match terrain {
            Terrain::Empty => {
                svg.push_str(&format!(
                    "    <circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"{}\" opacity=\"0.5\" />\n",
                    center_x, center_y, col
                ));
            }
            Terrain::Wall => {
                // Heavy masonry icon with hardness badge [H:2]
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"10\" rx=\"2\" fill=\"{}\" opacity=\"0.3\" />\n",
                    px + 10, center_y - 8, cell_size - 20, col
                ));
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"14\" font-weight=\"900\" fill=\"{}\" text-anchor=\"middle\">🧱 2H</text>\n",
                    center_x, center_y + 16, col
                ));
            }
            Terrain::Crate => {
                // Wooden crate with hardness badge [H:1]
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"14\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">📦 1H</text>\n",
                    center_x, center_y + 5, col
                ));
            }
            Terrain::Water => {
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">🌊 水域</text>\n",
                    center_x, center_y + 5, col
                ));
            }
            Terrain::Ice => {
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">🧊 冰面</text>\n",
                    center_x, center_y + 5, col
                ));
            }
            Terrain::Mine => {
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">💣 暗雷</text>\n",
                    center_x, center_y + 5, col
                ));
            }
            Terrain::Medkit => {
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">🛡️ 护盾</text>\n",
                    center_x, center_y + 5, col
                ));
            }
            Terrain::HighGround => {
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">⛰️ 高地</text>\n",
                    center_x, center_y + 5, col
                ));
            }
        }
    }

    fn render_player_token(
        svg: &mut String,
        px: u32,
        py: u32,
        cell_size: u32,
        player: &PlayerState,
        is_turn: bool,
    ) {
        let center_x = px + cell_size / 2;
        let center_y = py + cell_size / 2;
        let pcolor = Theme::player_color(player.id);
        let pcolor_str = pcolor.to_svg_color();

        // 1. Active Turn Halo Ring (pulsing / highlighted)
        if is_turn {
            svg.push_str(&format!(
                "    <circle cx=\"{}\" cy=\"{}\" r=\"29\" fill=\"none\" stroke=\"#f59e0b\" stroke-width=\"3\" opacity=\"0.9\" stroke-dasharray=\"6 3\" />\n",
                center_x, center_y
            ));
            // Indicator pointer
            svg.push_str(&format!(
                "    <polygon points=\"{},{} {},{} {},{}\" fill=\"#f59e0b\" />\n",
                center_x,
                py + 2,
                center_x - 5,
                py - 4,
                center_x + 5,
                py - 4
            ));
        }

        // 2. Shield Bubble Ring
        if player.has_shield {
            svg.push_str(&format!(
                "    <circle cx=\"{}\" cy=\"{}\" r=\"26\" fill=\"none\" stroke=\"#38bdf8\" stroke-width=\"2.5\" opacity=\"0.9\" />\n",
                center_x, center_y
            ));
        }

        // 3. Player Circular Avatar Base
        svg.push_str(&format!(
            "    <circle cx=\"{}\" cy=\"{}\" r=\"21\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2.5\" />\n",
            center_x, center_y, Theme::CARD_BG.to_svg_color(), pcolor_str
        ));

        // 4. Token Text & Badge
        let kind_icon = match player.kind {
            PlayerKind::Bot => "🤖",
            PlayerKind::Human => "👤",
        };

        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"11\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">P{}</text>\n",
            center_x, center_y - 2, pcolor_str, player.id.0
        ));
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"10\" text-anchor=\"middle\">{}</text>\n",
            center_x,
            center_y + 11,
            kind_icon
        ));
    }
}
