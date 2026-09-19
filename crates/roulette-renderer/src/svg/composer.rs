//! SVG Composer assembling tactical HUD visual components into a complete SVG document.

#![forbid(unsafe_code)]

use roulette_domain::{GameView, RefereeRoomView, RoomPhase, Weather};

use crate::svg::banner::BannerRenderer;
use crate::svg::board::BoardRenderer;
use crate::svg::header::HeaderRenderer;
use crate::svg::roster::RosterRenderer;
use crate::theme::{LayoutMetrics, Theme};

/// Composes full-scene tactical HUD cards into an SVG document.
pub struct SvgComposer;

impl SvgComposer {
    /// Renders an omniscient referee room view into an SVG document.
    #[must_use]
    pub fn compose_referee_room(room: &RefereeRoomView, metrics: &LayoutMetrics) -> String {
        let mut svg = String::with_capacity(16384);
        Self::render_svg_opening(&mut svg, metrics);

        let room_id = room.id.0.as_str();
        let room_name = room.name.as_str();

        if let Some(game) = &room.game {
            // Header
            svg.push_str(&HeaderRenderer::render(
                room_id,
                room_name,
                room.phase,
                game.round,
                Some(game.status),
                game.weather,
                game.rounds_without_elimination,
                metrics,
            ));

            // Board
            svg.push_str(&BoardRenderer::render(
                game.map_size,
                &game.cells,
                &game.players,
                game.current_player_id,
                metrics,
            ));

            // Roster
            svg.push_str(&RosterRenderer::render_game_roster(
                &game.players,
                game.current_player_id,
                &game.records,
                metrics,
            ));

            // Bottom Banner
            svg.push_str(&BannerRenderer::render(
                game.revision,
                game.seed,
                &game.records,
                metrics,
            ));
        } else {
            // Waiting Room Phase
            svg.push_str(&HeaderRenderer::render(
                room_id,
                room_name,
                room.phase,
                0,
                None,
                Weather::Clear,
                0,
                metrics,
            ));

            // Waiting placeholder board
            svg.push_str(&Self::render_waiting_board_placeholder(metrics));

            // Waiting Roster
            svg.push_str(&RosterRenderer::render_waiting_roster(
                &room.members,
                metrics,
            ));

            // Empty banner
            svg.push_str(&BannerRenderer::render(0, 0, &[], metrics));
        }

        Self::render_svg_closing(&mut svg);
        svg
    }

    /// Renders a public player game view into an SVG document.
    #[must_use]
    pub fn compose_game_view(
        game: &GameView,
        room_name: &str,
        room_id: &str,
        metrics: &LayoutMetrics,
    ) -> String {
        let mut svg = String::with_capacity(16384);
        Self::render_svg_opening(&mut svg, metrics);

        let phase = match game.status {
            roulette_domain::GameStatus::Running => RoomPhase::Playing,
            roulette_domain::GameStatus::Finished { .. } => RoomPhase::Finished,
        };

        // Header
        svg.push_str(&HeaderRenderer::render(
            room_id,
            room_name,
            phase,
            game.round,
            Some(game.status),
            game.weather,
            game.rounds_without_elimination,
            metrics,
        ));

        // Board
        svg.push_str(&BoardRenderer::render(
            game.map_size,
            &game.cells,
            &game.players,
            game.current_player_id,
            metrics,
        ));

        // Roster
        svg.push_str(&RosterRenderer::render_game_roster(
            &game.players,
            game.current_player_id,
            &game.records,
            metrics,
        ));

        // Bottom Banner
        svg.push_str(&BannerRenderer::render(
            game.revision,
            game.seed,
            &game.records,
            metrics,
        ));

        Self::render_svg_closing(&mut svg);
        svg
    }

    fn render_svg_opening(svg: &mut String, metrics: &LayoutMetrics) {
        let w = metrics.canvas_width;
        let h = metrics.canvas_height;
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">\n"
        ));
        svg.push_str("  <defs>\n");
        svg.push_str("    <style>\n");
        svg.push_str("      text { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'PingFang SC', 'Microsoft YaHei', sans-serif; }\n");
        svg.push_str("    </style>\n");
        svg.push_str("  </defs>\n");
        svg.push_str(crate::assets::render_shared_defs());

        // Canvas full background
        svg.push_str(&format!(
            "  <rect width=\"{w}\" height=\"{h}\" fill=\"{}\" />\n",
            Theme::CANVAS_BG.to_svg_color()
        ));
    }

    fn render_svg_closing(svg: &mut String) {
        svg.push_str("</svg>\n");
    }

    fn render_waiting_board_placeholder(metrics: &LayoutMetrics) -> String {
        let mut svg = String::with_capacity(1024);
        let origin_x = metrics.board_origin_x;
        let origin_y = metrics.board_origin_y;
        let board_total_w = 5 * metrics.cell_size + 4 * metrics.cell_gap + 40;
        let board_total_h = 5 * metrics.cell_size + 4 * metrics.cell_gap + 40;

        svg.push_str("  <!-- WAITING BOARD PLACEHOLDER -->\n");
        svg.push_str("  <g id=\"waiting-board-placeholder\">\n");
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6 4\" />\n",
            origin_x - 20, origin_y - 20, board_total_w, board_total_h, Theme::CARD_BG.to_svg_color(), Theme::CARD_BORDER.to_svg_color()
        ));
        let center_x = origin_x - 20 + board_total_w / 2;
        let center_y = origin_y - 20 + board_total_h / 2;
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"18\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">⏳ 战术沙盘整备中</text>\n",
            center_x, center_y - 12, Theme::TEXT_SECONDARY.to_svg_color()
        ));
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"13\" fill=\"{}\" text-anchor=\"middle\">等待房主指派席位并开启对局，棋盘与地形将在开局时投影</text>\n",
            center_x, center_y + 16, Theme::TEXT_MUTED.to_svg_color()
        ));
        svg.push_str("  </g>\n");
        svg
    }
}
