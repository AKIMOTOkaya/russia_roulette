//! Header HUD SVG renderer.

#![forbid(unsafe_code)]

use roulette_domain::{GameStatus, RoomPhase, Weather};

use crate::theme::{LayoutMetrics, Theme};

/// Renders the top HUD header containing title, room tag, phase, weather, and stalemate warning.
pub struct HeaderRenderer;

impl HeaderRenderer {
    /// Generates SVG markup for the header HUD bar.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render(
        room_id: &str,
        room_name: &str,
        phase: RoomPhase,
        round: u32,
        status: Option<GameStatus>,
        weather: Weather,
        stalemate_rounds: u32,
        metrics: &LayoutMetrics,
    ) -> String {
        let mut svg = String::with_capacity(4096);
        let w = metrics.canvas_width;

        svg.push_str("  <!-- TOP HEADER HUD -->\n");
        svg.push_str("  <g id=\"header-hud\">\n");

        // Top HUD background bar
        svg.push_str(&format!(
            "    <rect x=\"24\" y=\"20\" width=\"{}\" height=\"86\" rx=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" />\n",
            w - 48,
            Theme::CARD_BG.to_svg_color(),
            Theme::CARD_BORDER.to_svg_color()
        ));

        // Left: Game title & Room badge
        svg.push_str(&format!(
            "    <text x=\"44\" y=\"50\" font-size=\"18\" font-weight=\"900\" fill=\"{}\" letter-spacing=\"1.5\">RUSSIAN ROULETTE <tspan font-size=\"14\" font-weight=\"normal\" fill=\"{}\">// 俄罗斯轮盘</tspan></text>\n",
            Theme::TEXT_PRIMARY.to_svg_color(),
            Theme::TEXT_MUTED.to_svg_color()
        ));

        svg.push_str(&format!(
            "    <rect x=\"44\" y=\"64\" width=\"96\" height=\"24\" rx=\"5\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" />\n",
            Theme::SECTION_BG.to_svg_color(),
            Theme::ACCENT_CYAN.to_svg_color()
        ));
        svg.push_str(&format!(
            "    <text x=\"92\" y=\"80\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">房间 [{}]</text>\n",
            Theme::ACCENT_CYAN.to_svg_color(),
            room_id
        ));

        let display_name = if room_name.is_empty() {
            "对局室"
        } else {
            room_name
        };
        svg.push_str(&format!(
            "    <text x=\"150\" y=\"81\" font-size=\"13\" font-weight=\"500\" fill=\"{}\">「{}」</text>\n",
            Theme::TEXT_SECONDARY.to_svg_color(),
            display_name
        ));

        // Center: Match Phase & Round
        let (phase_label, phase_color) = match phase {
            RoomPhase::Waiting => ("⏳ 等待开局中", Theme::TEXT_SECONDARY),
            RoomPhase::Playing => ("⚔️ 激战中", Theme::ACCENT_AMBER),
            RoomPhase::Finished => match status {
                Some(GameStatus::Finished { winner_id: Some(w) }) => {
                    return Self::render_winner_header(svg, w.0, room_id, room_name, metrics);
                }
                Some(GameStatus::Finished { winner_id: None }) => {
                    ("💀 全员阵亡 (平局)", Theme::ACCENT_RED)
                }
                _ => ("🏁 终局", Theme::ACCENT_GREEN),
            },
        };

        let center_x = w / 2;
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"32\" width=\"180\" height=\"30\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\" />\n",
            center_x - 90,
            Theme::SECTION_BG.to_svg_color(),
            phase_color.to_svg_color()
        ));
        let round_text = if phase == RoomPhase::Playing {
            format!("{} · ROUND {}", phase_label, round)
        } else {
            phase_label.to_string()
        };
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"52\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">{}</text>\n",
            center_x,
            phase_color.to_svg_color(),
            round_text
        ));

        // Stalemate warning banner under phase badge
        if stalemate_rounds > 0 {
            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"80\" font-size=\"12\" font-weight=\"bold\" fill=\"#ef4444\" text-anchor=\"middle\">🔥 连续 {} 轮未减员 (致死率飙升)</text>\n",
                center_x, stalemate_rounds
            ));
        } else {
            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"80\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">确定性物理裁定 &amp; BFS事件树管线</text>\n",
                center_x,
                Theme::TEXT_MUTED.to_svg_color()
            ));
        }

        // Right: Weather Card
        let (weather_icon, weather_label, weather_color) = Theme::weather_style(weather);
        let weather_card_x = w - 270;
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"32\" width=\"230\" height=\"60\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" />\n",
            weather_card_x,
            Theme::SECTION_BG.to_svg_color(),
            weather_color.to_svg_color()
        ));
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"54\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\">{} 全局天气</text>\n",
            weather_card_x + 12,
            weather_color.to_svg_color(),
            weather_icon
        ));
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"76\" font-size=\"11\" fill=\"{}\">{}</text>\n",
            weather_card_x + 12,
            Theme::TEXT_SECONDARY.to_svg_color(),
            weather_label
        ));

        svg.push_str("  </g>\n");
        svg
    }

    fn render_winner_header(
        mut svg: String,
        winner_id: u32,
        _room_id: &str,
        _room_name: &str,
        metrics: &LayoutMetrics,
    ) -> String {
        let center_x = metrics.canvas_width / 2;
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"30\" width=\"260\" height=\"34\" rx=\"6\" fill=\"#064e3b\" stroke=\"#10b981\" stroke-width=\"1.5\" />\n",
            center_x - 130
        ));
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"52\" font-size=\"14\" font-weight=\"900\" fill=\"#34d399\" text-anchor=\"middle\">🏆 胜者加冕: PLAYER {} 最终幸存！</text>\n",
            center_x, winner_id
        ));
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"80\" font-size=\"12\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">对局已圆满结束，全场决斗闭环</text>\n",
            center_x, Theme::TEXT_SECONDARY.to_svg_color()
        ));
        svg.push_str("  </g>\n");
        svg
    }
}
