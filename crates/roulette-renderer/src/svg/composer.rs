//! SVG Composer assembling tactical HUD visual components into a complete SVG document.

#![forbid(unsafe_code)]

use roulette_domain::{GameView, RefereeMemberView, RefereeRoomView};

use crate::svg::board::BoardRenderer;
use crate::theme::{LayoutMetrics, Theme};

/// Composes tactical maps and room projections into an SVG document.
pub struct SvgComposer;

impl SvgComposer {
    /// Renders an omniscient referee room view into an SVG document.
    ///
    /// Outputs a pure tactical map when a game is active, or a clean minimalist
    /// waiting canvas during room assembly.
    #[must_use]
    pub fn compose_referee_room(room: &RefereeRoomView, _metrics: &LayoutMetrics) -> String {
        if let Some(game) = &room.game {
            BoardRenderer::render_standalone(
                &game.cells,
                &game.players,
                game.current_player_id,
                &game.records,
                Some(80),
            )
        } else {
            Self::render_waiting_pure_svg(&room.id.0, &room.name, &room.members)
        }
    }

    /// Renders a public player game view into an SVG document.
    ///
    /// Always outputs the pure tactical map without web cards or roster columns.
    #[must_use]
    pub fn compose_game_view(
        game: &GameView,
        _room_name: &str,
        _room_id: &str,
        _metrics: &LayoutMetrics,
    ) -> String {
        BoardRenderer::render_standalone(
            &game.cells,
            &game.players,
            game.current_player_id,
            &game.records,
            Some(80),
        )
    }

    /// Renders a clean minimalist waiting canvas before match initiation.
    fn render_waiting_pure_svg(
        room_id: &str,
        room_name: &str,
        members: &[RefereeMemberView],
    ) -> String {
        let size = 480;
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 {size} {size}\">\n"
        );
        svg.push_str("  <defs>\n");
        svg.push_str("    <style>\n");
        svg.push_str("      text { font-family: 'MiSans', 'Noto Sans CJK SC', 'Source Han Sans SC', 'PingFang SC', 'Microsoft YaHei', 'DejaVu Sans', sans-serif; }\n");
        svg.push_str("    </style>\n");
        svg.push_str("  </defs>\n");
        svg.push_str(crate::assets::render_shared_defs());

        // Background
        svg.push_str(&format!(
            "  <!-- WAITING CANVAS BACKGROUND -->\n  <rect width=\"{size}\" height=\"{size}\" fill=\"{}\" />\n",
            Theme::CANVAS_BG.to_svg_color()
        ));

        // Ground Board Plate with subtle dashed border
        svg.push_str(&format!(
            "  <rect x=\"24\" y=\"24\" width=\"432\" height=\"432\" rx=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"6 4\" />\n",
            Theme::BOARD_BG.to_svg_color(),
            Theme::BOARD_BORDER.to_svg_color()
        ));

        // Room header
        svg.push_str(&format!(
            "  <text x=\"240\" y=\"68\" font-size=\"16\" font-weight=\"900\" fill=\"{}\" text-anchor=\"middle\">{} · {}</text>\n",
            Theme::TEXT_PRIMARY.to_svg_color(),
            room_id,
            room_name
        ));
        svg.push_str(&format!(
            "  <text x=\"240\" y=\"92\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">等待开局中</text>\n",
            Theme::TEXT_MUTED.to_svg_color()
        ));

        // Center callout
        svg.push_str(&format!(
            "  <text x=\"240\" y=\"200\" font-size=\"18\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">⏳ 战术沙盘整备中</text>\n",
            Theme::TEXT_SECONDARY.to_svg_color()
        ));
        svg.push_str(&format!(
            "  <text x=\"240\" y=\"228\" font-size=\"13\" fill=\"{}\" text-anchor=\"middle\">等待房主指派席位并开启对局，棋盘将在开局时投影</text>\n",
            Theme::TEXT_MUTED.to_svg_color()
        ));

        // Members list at bottom
        let start_y = 290;
        for (i, m) in members.iter().take(6).enumerate() {
            let col = (i % 2) as u32;
            let row = (i / 2) as u32;
            let mx = 48 + col * 196;
            let my = start_y + row * 38;

            let kind_label = match m.kind {
                roulette_domain::PlayerKind::Human => "👤 真人",
                roulette_domain::PlayerKind::Bot => "🤖 Bot",
            };
            let owner_badge = if m.is_owner { " [房主]" } else { "" };
            svg.push_str(&format!(
                "  <rect x=\"{mx}\" y=\"{}\" width=\"188\" height=\"28\" rx=\"6\" fill=\"#ffffff\" stroke=\"#e2e8f0\" stroke-width=\"1\" />\n",
                my - 18
            ));
            svg.push_str(&format!(
                "  <text x=\"{}\" y=\"{my}\" font-size=\"11\" font-weight=\"bold\" fill=\"{}\">{kind_label}: {}{owner_badge}</text>\n",
                mx + 10,
                Theme::TEXT_SECONDARY.to_svg_color(),
                m.name
            ));
        }

        svg.push_str("</svg>\n");
        svg
    }
}
