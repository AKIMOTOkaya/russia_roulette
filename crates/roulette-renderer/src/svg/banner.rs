//! Bottom battle feed banner SVG renderer.

#![forbid(unsafe_code)]

use roulette_domain::{Direction, GameEvent, GameRecord, GameRecordContent, PlayerCommand};

use crate::theme::{LayoutMetrics, Theme};

/// Renders the bottom battle feed banner showing latest action and dramatic event.
pub struct BannerRenderer;

impl BannerRenderer {
    /// Generates SVG markup for the bottom banner.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render(
        revision: u64,
        seed: u64,
        records: &[GameRecord],
        metrics: &LayoutMetrics,
    ) -> String {
        let mut svg = String::with_capacity(4096);
        let banner_x = 24;
        let banner_y = metrics.canvas_height - 96;
        let banner_w = metrics.canvas_width - 48;
        let banner_h = 76;

        svg.push_str("  <!-- BOTTOM BATTLE BANNER -->\n");
        svg.push_str("  <g id=\"battle-banner\">\n");

        // Banner background
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" />\n",
            banner_x, banner_y, banner_w, banner_h, Theme::CARD_BG.to_svg_color(), Theme::CARD_BORDER.to_svg_color()
        ));

        // Latest action text
        let action_text = Self::extract_latest_action(records);
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"13\" font-weight=\"bold\" fill=\"{}\">🎯 [最近裁定] <tspan font-weight=\"normal\" fill=\"{}\">{}</tspan></text>\n",
            banner_x + 20, banner_y + 28, Theme::ACCENT_AMBER.to_svg_color(), Theme::TEXT_PRIMARY.to_svg_color(), action_text
        ));

        // Latest drama event or notification text
        let event_text = Self::extract_latest_drama_or_event(records);
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"12\" font-weight=\"500\" fill=\"{}\">⚡ [战局动向] <tspan fill=\"{}\">{}</tspan></text>\n",
            banner_x + 20, banner_y + 52, Theme::ACCENT_PURPLE.to_svg_color(), Theme::TEXT_SECONDARY.to_svg_color(), event_text
        ));

        // Right-aligned engine watermark
        let watermark_x = banner_x + banner_w - 20;
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"10\" fill=\"{}\" text-anchor=\"end\">RUSSIAN ROULETTE CORE // REV: {} // SEED: {}</text>\n",
            watermark_x, banner_y + 60, Theme::TEXT_MUTED.to_svg_color(), revision, seed
        ));

        svg.push_str("  </g>\n");
        svg
    }

    fn extract_latest_action(records: &[GameRecord]) -> String {
        for r in records.iter().rev() {
            if let GameRecordContent::Action { actor_id, command } = &r.content {
                let cmd_str = match command {
                    PlayerCommand::Move { direction } => {
                        format!("移动位移 [{}]", Self::dir_str(*direction))
                    }
                    PlayerCommand::Shoot { direction } => {
                        format!("扣动扳机射击 [{}]", Self::dir_str(*direction))
                    }
                    PlayerCommand::Wait => "放弃本轮行动 (等待)".to_string(),
                    PlayerCommand::Suicide => "饮弹自戕".to_string(),
                };
                return format!("Player {} 执行了 {}", actor_id.0, cmd_str);
            }
        }
        "等待首个行动意图提交...".to_string()
    }

    fn extract_latest_drama_or_event(records: &[GameRecord]) -> String {
        for r in records.iter().rev() {
            if let GameRecordContent::Event { event } = &r.content {
                match event {
                    GameEvent::DramaticEvent {
                        title, narrative, ..
                    } => {
                        return format!("【{title}】{narrative}");
                    }
                    GameEvent::PlayerEliminated {
                        player_id,
                        cause,
                        by_player_id,
                    } => {
                        let by =
                            by_player_id.map_or(String::new(), |b| format!(" (由 P{} 击杀)", b.0));
                        return format!("Player {} 出局！死因: {:?}{}", player_id.0, cause, by);
                    }
                    GameEvent::ShieldConsumed { player_id, .. } => {
                        return format!("Player {} 护盾碎裂，成功抵挡一次致命裁定！", player_id.0);
                    }
                    GameEvent::EmptyChamber { actor_id, .. } => {
                        return format!("Player {} 扣动扳机 ➔ 咔哒 (转轮哑火空弹)", actor_id.0);
                    }
                    GameEvent::WeatherChanged { from, to } => {
                        return format!("天气异变: 从 {:?} 转变为 {:?}", from, to);
                    }
                    GameEvent::TerrainChanged { from, to, position } => {
                        return format!(
                            "地形重塑: ({}, {}) 从 {:?} 转化为 {:?}",
                            position.x, position.y, from, to
                        );
                    }
                    _ => {}
                }
            }
        }
        "全场环境稳定，各席位保持警戒。".to_string()
    }

    const fn dir_str(dir: Direction) -> &'static str {
        match dir {
            Direction::Up => "向北 ↑",
            Direction::Down => "向南 ↓",
            Direction::Left => "向西 ←",
            Direction::Right => "向东 →",
        }
    }
}
