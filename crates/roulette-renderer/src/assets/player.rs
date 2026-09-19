//! Pure procedural vector graphics for player tokens and combatants.

#![forbid(unsafe_code)]

use roulette_domain::{Direction, PlayerId, PlayerKind, PlayerState, PlayerStatus};

use crate::theme::Theme;

/// Procedural vector renderer for tactical player tokens.
pub struct PlayerAssetRenderer;

impl PlayerAssetRenderer {
    /// Renders a tactical player token at `(cx, cy)` with given `radius`.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render(
        cx: u32,
        cy: u32,
        radius: u32,
        player: &PlayerState,
        is_turn: bool,
        facing_dir: Option<Direction>,
    ) -> String {
        let mut svg = String::with_capacity(2048);
        let pcolor = Theme::player_color(player.id);
        let pcolor_str = pcolor.to_svg_color();

        // 1. If eliminated: render destroyed/KIA token
        if player.status == PlayerStatus::Eliminated {
            Self::render_eliminated(&mut svg, cx, cy, radius, player.id);
            return svg;
        }

        // 2. Shield Bubble / Forcefield Aura
        if player.has_shield {
            let s_r = radius + 6;
            svg.push_str(&format!(
                "    <!-- Shield Barrier -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{s_r}\" fill=\"none\" stroke=\"#38bdf8\" stroke-width=\"2\" stroke-dasharray=\"10 4\" opacity=\"0.85\" filter=\"url(#fx-glow-cyan)\" />\n"
            ));
        }

        // 3. Active Turn Halo Ring & Directional Pointer
        if is_turn {
            let t_r = radius + 3;
            svg.push_str(&format!(
                "    <!-- Turn Indicator -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{t_r}\" fill=\"none\" stroke=\"#f59e0b\" stroke-width=\"2.5\" stroke-dasharray=\"8 4\" filter=\"url(#fx-glow-amber)\" />\n"
            ));

            // Compass arrow indicating facing or turn pointer
            let arrow_dir = facing_dir.unwrap_or(Direction::Up);
            Self::render_pointer(&mut svg, cx, cy, radius, arrow_dir);
        }

        // 4. Token Base Disc
        svg.push_str(&format!(
            "    <!-- Player Base -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"#0b0f19\" stroke=\"{pcolor_str}\" stroke-width=\"2.5\" filter=\"url(#fx-drop-shadow)\" />\n"
        ));
        let inner_r = radius.saturating_sub(3);
        svg.push_str(&format!(
            "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{inner_r}\" fill=\"none\" stroke=\"{pcolor_str}\" stroke-width=\"1\" opacity=\"0.4\" />\n"
        ));

        // 5. Kind-specific Vector Emblem (Human Visor vs Bot Core)
        match player.kind {
            PlayerKind::Human => {
                Self::render_human_emblem(&mut svg, cx, cy);
            }
            PlayerKind::Bot => {
                Self::render_bot_emblem(&mut svg, cx, cy);
            }
        }

        // 6. Seat Badge (e.g. P1, P2) at the bottom
        let badge_w = 26;
        let badge_h = 12;
        let badge_x = cx - badge_w / 2;
        let badge_y = cy + radius - badge_h - 2;
        svg.push_str(&format!(
            "    <rect x=\"{badge_x}\" y=\"{badge_y}\" width=\"{badge_w}\" height=\"{badge_h}\" rx=\"3\" fill=\"#020617\" stroke=\"{pcolor_str}\" stroke-width=\"1\" />\n"
        ));
        let text_y = badge_y + 9;
        svg.push_str(&format!(
            "    <text x=\"{cx}\" y=\"{text_y}\" font-size=\"9\" font-weight=\"900\" fill=\"#f8fafc\" text-anchor=\"middle\" letter-spacing=\"0.5\">P{}</text>\n",
            player.id.0
        ));

        // 7. Water hazard warning indicator
        if player.water_turns > 0 {
            let water_y = cy + radius + 4;
            svg.push_str(&format!(
                "    <path d=\"M {},{} Q {},{} {},{} T {},{}\" fill=\"none\" stroke=\"#38bdf8\" stroke-width=\"1.5\" />\n",
                cx - 8, water_y, cx - 4, water_y - 2, cx, water_y, cx + 8, water_y
            ));
        }

        svg
    }

    /// Renders a destroyed / eliminated player silhouette.
    fn render_eliminated(svg: &mut String, cx: u32, cy: u32, radius: u32, player_id: PlayerId) {
        svg.push_str(&format!(
            "    <!-- Eliminated Player P{} -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"#111827\" stroke=\"#475569\" stroke-width=\"1.5\" opacity=\"0.6\" />\n",
            player_id.0
        ));
        // Crossed diagonal bones
        let arm = 10;
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#dc2626\" stroke-width=\"2\" stroke-linecap=\"round\" opacity=\"0.8\" />\n",
            cx - arm, cy - arm, cx + arm, cy + arm
        ));
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#dc2626\" stroke-width=\"2\" stroke-linecap=\"round\" opacity=\"0.8\" />\n",
            cx + arm, cy - arm, cx - arm, cy + arm
        ));
        // Center KIA badge
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"26\" height=\"12\" rx=\"2\" fill=\"#450a0a\" stroke=\"#ef4444\" stroke-width=\"1\" />\n",
            cx - 13, cy - 6
        ));
        svg.push_str(&format!(
            "    <text x=\"{cx}\" y=\"{}\" font-size=\"8\" font-weight=\"900\" fill=\"#fca5a5\" text-anchor=\"middle\">KIA</text>\n",
            cy + 3
        ));
    }

    /// Renders an orientation pointer arrow on the perimeter of the token.
    fn render_pointer(svg: &mut String, cx: u32, cy: u32, radius: u32, dir: Direction) {
        let (p1, p2, p3) = match dir {
            Direction::Up => {
                let tip_y = cy.saturating_sub(radius + 8);
                let base_y = cy.saturating_sub(radius + 3);
                ((cx, tip_y), (cx - 5, base_y), (cx + 5, base_y))
            }
            Direction::Down => {
                let tip_y = cy + radius + 8;
                let base_y = cy + radius + 3;
                ((cx, tip_y), (cx - 5, base_y), (cx + 5, base_y))
            }
            Direction::Left => {
                let tip_x = cx.saturating_sub(radius + 8);
                let base_x = cx.saturating_sub(radius + 3);
                ((tip_x, cy), (base_x, cy - 5), (base_x, cy + 5))
            }
            Direction::Right => {
                let tip_x = cx + radius + 8;
                let base_x = cx + radius + 3;
                ((tip_x, cy), (base_x, cy - 5), (base_x, cy + 5))
            }
        };

        svg.push_str(&format!(
            "    <polygon points=\"{},{} {},{} {},{}\" fill=\"#f59e0b\" />\n",
            p1.0, p1.1, p2.0, p2.1, p3.0, p3.1
        ));
    }

    /// Renders a high-tech tactical visor for human players.
    fn render_human_emblem(svg: &mut String, cx: u32, cy: u32) {
        let visor_y = cy - 3;
        // Headset helmet arch
        svg.push_str(&format!(
            "    <path d=\"M {},{} Q {},{} {},{}\" fill=\"none\" stroke=\"#94a3b8\" stroke-width=\"1.5\" />\n",
            cx - 11, visor_y - 3, cx, visor_y - 12, cx + 11, visor_y - 3
        ));
        // Headset ear caps
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"3\" height=\"8\" rx=\"1\" fill=\"#64748b\" />\n",
            cx - 13,
            visor_y - 3
        ));
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"3\" height=\"8\" rx=\"1\" fill=\"#64748b\" />\n",
            cx + 10,
            visor_y - 3
        ));
        // Neon Visor slit
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{visor_y}\" width=\"20\" height=\"6\" rx=\"3\" fill=\"#00f0ff\" filter=\"url(#fx-glow-cyan)\" />\n",
            cx - 10
        ));
        // Glint specular reflection
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ffffff\" stroke-width=\"1.5\" stroke-linecap=\"round\" />\n",
            cx - 7, visor_y + 2, cx - 2, visor_y + 2
        ));
    }

    /// Renders a cybernetic CPU microchip and optical sensor eye for bot players.
    fn render_bot_emblem(svg: &mut String, cx: u32, cy: u32) {
        let chip_y = cy - 4;
        // Chip pins on left and right
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#64748b\" stroke-width=\"1.5\" />\n",
            cx - 12, chip_y - 2, cx - 8, chip_y - 2
        ));
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#64748b\" stroke-width=\"1.5\" />\n",
            cx - 12, chip_y + 4, cx - 8, chip_y + 4
        ));
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#64748b\" stroke-width=\"1.5\" />\n",
            cx + 8, chip_y - 2, cx + 12, chip_y - 2
        ));
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#64748b\" stroke-width=\"1.5\" />\n",
            cx + 8, chip_y + 4, cx + 12, chip_y + 4
        ));
        // Chip body
        svg.push_str(&format!(
            "    <rect x=\"{}\" y=\"{}\" width=\"16\" height=\"14\" rx=\"2\" fill=\"#1e293b\" stroke=\"#94a3b8\" stroke-width=\"1.5\" />\n",
            cx - 8, chip_y - 4
        ));
        // Central glowing optical sensor eye
        svg.push_str(&format!(
            "    <circle cx=\"{cx}\" cy=\"{}\" r=\"3.5\" fill=\"#ef4444\" filter=\"url(#fx-glow-laser)\" />\n",
            chip_y + 3
        ));
        svg.push_str(&format!(
            "    <circle cx=\"{cx}\" cy=\"{}\" r=\"1.2\" fill=\"#ffffff\" />\n",
            chip_y + 3
        ));
    }

    /// Generates a standalone SVG string for an individual player token, ready to be embedded or saved.
    #[must_use]
    pub fn render_standalone(
        player_id: PlayerId,
        kind: PlayerKind,
        is_turn: bool,
        has_shield: bool,
        size: u32,
    ) -> String {
        let radius = size / 2 - 8;
        let cx = size / 2;
        let cy = size / 2;

        let dummy_player = PlayerState {
            id: player_id,
            name: format!("Player {}", player_id.0),
            kind,
            status: PlayerStatus::Alive,
            position: None,
            has_shield,
            water_turns: 0,
            consecutive_shots: 0,
        };

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 {size} {size}\">\n"
        );
        svg.push_str(crate::assets::render_shared_defs());
        svg.push_str(&Self::render(
            cx,
            cy,
            radius,
            &dummy_player,
            is_turn,
            Some(Direction::Up),
        ));
        svg.push_str("</svg>\n");
        svg
    }
}
