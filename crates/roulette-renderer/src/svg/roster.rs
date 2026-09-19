//! Player roster panel SVG renderer.

#![forbid(unsafe_code)]

use roulette_domain::{
    EliminationCause, GameRecordContent, PlayerId, PlayerKind, PlayerState, PlayerStatus,
    RefereeMemberView,
};

use crate::theme::{LayoutMetrics, Theme};

/// Renders the right-hand player roster and vitals panel.
pub struct RosterRenderer;

impl RosterRenderer {
    /// Renders active match player states.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render_game_roster(
        players: &[PlayerState],
        current_player_id: Option<PlayerId>,
        recent_records: &[roulette_domain::GameRecord],
        metrics: &LayoutMetrics,
    ) -> String {
        let mut svg = String::with_capacity(6144);
        let panel_x = metrics.canvas_width - 460;
        let panel_y = metrics.board_origin_y - 20;
        let panel_w = 420;
        let panel_h = 450;

        svg.push_str("  <!-- PLAYER ROSTER PANEL -->\n");
        svg.push_str("  <g id=\"player-roster\">\n");

        // Panel background
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" />\n",
            panel_x, panel_y, panel_w, panel_h, Theme::CARD_BG.to_svg_color(), Theme::CARD_BORDER.to_svg_color()
        ));

        // Panel Header
        let alive_count = players
            .iter()
            .filter(|p| p.status == PlayerStatus::Alive)
            .count();
        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"14\" font-weight=\"bold\" fill=\"{}\">👥 参战席位与生命体征 <tspan fill=\"{}\">({}/{})</tspan></text>\n",
            panel_x + 16, panel_y + 26, Theme::TEXT_PRIMARY.to_svg_color(), Theme::ACCENT_CYAN.to_svg_color(), alive_count, players.len()
        ));

        // Player Cards stack
        let card_start_y = panel_y + 40;
        let card_gap = 10;
        let card_h = if players.len() <= 4 { 84 } else { 62 };

        for (idx, p) in players.iter().enumerate() {
            let cy = card_start_y + (idx as u32) * (card_h + card_gap);
            let is_turn = current_player_id == Some(p.id);
            let pcolor = Theme::player_color(p.id);

            let border_color = if is_turn {
                Theme::ACCENT_AMBER.to_svg_color()
            } else {
                Theme::SECTION_BG.to_svg_color()
            };
            let border_width = if is_turn { "2" } else { "1" };

            // Card background
            svg.push_str(&format!(
                "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" />\n",
                panel_x + 16, cy, panel_w - 32, card_h, Theme::SECTION_BG.to_svg_color(), border_color, border_width
            ));

            // Left color stripe
            svg.push_str(&format!(
                "    <rect x=\"{}\" y=\"{}\" width=\"6\" height=\"{}\" rx=\"3\" fill=\"{}\" />\n",
                panel_x + 16,
                cy,
                card_h,
                pcolor.to_svg_color()
            ));

            // Name and Role
            let kind_tag = match p.kind {
                PlayerKind::Bot => "🤖 BOT",
                PlayerKind::Human => "👤 真人",
            };
            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" font-size=\"14\" font-weight=\"bold\" fill=\"{}\">P{} · {} <tspan font-size=\"11\" font-weight=\"normal\" fill=\"{}\">[{}]</tspan></text>\n",
                panel_x + 32, cy + 22, Theme::TEXT_PRIMARY.to_svg_color(), p.id.0, p.name, Theme::TEXT_MUTED.to_svg_color(), kind_tag
            ));

            // Current Turn Badge
            if is_turn {
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"76\" height=\"20\" rx=\"4\" fill=\"#78350f\" stroke=\"#f59e0b\" stroke-width=\"1\" />\n",
                    panel_x + panel_w - 100, cy + 8
                ));
                svg.push_str(&format!(
                    "    <text x=\"{}\" y=\"{}\" font-size=\"11\" font-weight=\"bold\" fill=\"#fbbf24\" text-anchor=\"middle\">👉 行动中</text>\n",
                    panel_x + panel_w - 62, cy + 22
                ));
            }

            // Status Row
            match p.status {
                PlayerStatus::Alive => {
                    let pos_str = p.position.map_or_else(
                        || "未知".to_string(),
                        |pos| format!("({}, {})", pos.x, pos.y),
                    );
                    let shield_badge = if p.has_shield {
                        format!(
                            "<tspan fill=\"{}\"> | 🛡️ 充能护盾</tspan>",
                            Theme::ACCENT_CYAN.to_svg_color()
                        )
                    } else {
                        String::new()
                    };

                    svg.push_str(&format!(
                        "    <text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"{}\">💚 存活 <tspan fill=\"{}\">📍 坐标 {}</tspan>{}</text>\n",
                        panel_x + 32, cy + 44, Theme::ACCENT_GREEN.to_svg_color(), Theme::TEXT_SECONDARY.to_svg_color(), pos_str, shield_badge
                    ));
                }
                PlayerStatus::Eliminated => {
                    let cause_str = Self::find_elimination_cause(p.id, recent_records);
                    svg.push_str(&format!(
                        "    <text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"#ef4444\">💀 淘汰 <tspan fill=\"#94a3b8\">[出局原因: {}]</tspan></text>\n",
                        panel_x + 32, cy + 44, cause_str
                    ));
                }
            }
        }

        svg.push_str("  </g>\n");
        svg
    }

    /// Renders member roster when match is still in waiting lobby phase.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render_waiting_roster(members: &[RefereeMemberView], metrics: &LayoutMetrics) -> String {
        let mut svg = String::with_capacity(4096);
        let panel_x = metrics.canvas_width - 460;
        let panel_y = metrics.board_origin_y - 20;
        let panel_w = 420;
        let panel_h = 450;

        svg.push_str("  <!-- WAITING LOBBY ROSTER -->\n");
        svg.push_str("  <g id=\"waiting-roster\">\n");

        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\" />\n",
            panel_x, panel_y, panel_w, panel_h, Theme::CARD_BG.to_svg_color(), Theme::CARD_BORDER.to_svg_color()
        ));

        svg.push_str(&format!(
            "    <text x=\"{}\" y=\"{}\" font-size=\"14\" font-weight=\"bold\" fill=\"{}\">👥 房间成员席位 (已入座: {}/6)</text>\n",
            panel_x + 16, panel_y + 26, Theme::TEXT_PRIMARY.to_svg_color(), members.len()
        ));

        let card_start_y = panel_y + 40;
        let card_gap = 10;
        let card_h = 58;

        for (idx, m) in members.iter().enumerate() {
            let cy = card_start_y + (idx as u32) * (card_h + card_gap);
            let owner_badge = if m.is_owner { " 👑房主" } else { "" };
            let (kind_icon, kind_name) = match m.kind {
                PlayerKind::Bot => ("🤖", "Bot 机器人"),
                PlayerKind::Human => ("👤", "真人玩家"),
            };

            svg.push_str(&format!(
                "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\" />\n",
                panel_x + 16, cy, panel_w - 32, card_h, Theme::SECTION_BG.to_svg_color(), Theme::CARD_BORDER.to_svg_color()
            ));

            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" font-size=\"14\" font-weight=\"bold\" fill=\"{}\">{} 席位 {} · {}{}</text>\n",
                panel_x + 32, cy + 24, Theme::TEXT_PRIMARY.to_svg_color(), kind_icon, idx + 1, m.name, owner_badge
            ));

            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"{}\" font-size=\"12\" fill=\"{}\">类型: {} | 状态: 就绪</text>\n",
                panel_x + 32, cy + 44, Theme::TEXT_MUTED.to_svg_color(), kind_name
            ));
        }

        svg.push_str("  </g>\n");
        svg
    }

    fn find_elimination_cause(
        player_id: PlayerId,
        records: &[roulette_domain::GameRecord],
    ) -> String {
        for r in records.iter().rev() {
            if let GameRecordContent::Event {
                event:
                    roulette_domain::GameEvent::PlayerEliminated {
                        player_id: pid,
                        cause,
                        by_player_id,
                    },
            } = &r.content
                && *pid == player_id
            {
                let cause_name = match cause {
                    EliminationCause::Shot => "左轮实弹击毙",
                    EliminationCause::ElbowDuel => "拼肘决斗战败",
                    EliminationCause::Mine => "踩雷烈性殉爆",
                    EliminationCause::Drowned => "深水涉水溺亡",
                    EliminationCause::Suicide => "扣动扳机自戕",
                    EliminationCause::Collision => "剧烈撞墙身亡",
                };
                if let Some(by) = by_player_id {
                    return format!("{cause_name} (由 P{} 击杀)", by.0);
                }
                return cause_name.to_string();
            }
        }
        "生命体征清零".to_string()
    }
}
