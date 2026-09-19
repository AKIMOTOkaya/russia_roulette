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
        let mut svg = String::with_capacity(1024);
        let pcolor = Theme::player_color(player.id);
        let pcolor_str = pcolor.to_svg_color();

        // 1. If eliminated: render destroyed / fallen token
        if player.status == PlayerStatus::Eliminated {
            Self::render_eliminated(&mut svg, cx, cy, radius, player.id);
            return svg;
        }

        // 2. Shield Bubble / Forcefield Aura
        if player.has_shield {
            let s_r = radius + 6;
            svg.push_str(&format!(
                "    <!-- Shield Barrier -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{s_r}\" fill=\"none\" stroke=\"#0284c7\" stroke-width=\"2\" stroke-dasharray=\"6 3\" opacity=\"0.85\" filter=\"url(#fx-glow-cyan)\" />\n"
            ));
        }

        // 3. Active Turn Halo Ring & Directional Pointer
        if is_turn {
            let t_r = radius + 4;
            svg.push_str(&format!(
                "    <!-- Turn Indicator -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{t_r}\" fill=\"none\" stroke=\"#f59e0b\" stroke-width=\"2.5\" filter=\"url(#fx-glow-amber)\" />\n"
            ));

            // Compass arrow indicating facing or turn pointer
            let arrow_dir = facing_dir.unwrap_or(Direction::Up);
            Self::render_pointer(&mut svg, cx, cy, radius, arrow_dir);
        }

        // 4. Token Base Disc with Drop Shadow
        svg.push_str(&format!(
            "    <!-- Player Base -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"{pcolor_str}\" stroke=\"#ffffff\" stroke-width=\"2.5\" filter=\"url(#fx-token-shadow)\" />\n"
        ));

        // 5. Kind-specific Vector Emblem (Human dot vs Bot notch)
        match player.kind {
            PlayerKind::Human => {
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{}\" r=\"2.5\" fill=\"#ffffff\" opacity=\"0.9\" />\n",
                    cy - radius * 4 / 10
                ));
            }
            PlayerKind::Bot => {
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"8\" height=\"2.5\" rx=\"1\" fill=\"#ffffff\" opacity=\"0.85\" />\n",
                    cx - 4,
                    cy + radius * 4 / 10 - 2
                ));
            }
        }

        // 6. Crisp Seat Label (P1, P2...) in center
        let font_size = (radius * 6 / 10).max(11);
        let text_y = cy + font_size / 3 + 1;
        svg.push_str(&format!(
            "    <text x=\"{cx}\" y=\"{text_y}\" font-size=\"{font_size}\" font-weight=\"900\" fill=\"#ffffff\" text-anchor=\"middle\" letter-spacing=\"-0.5\">P{}</text>\n",
            player.id.0
        ));

        // 7. Water hazard warning indicator
        if player.water_turns > 0 {
            let water_y = cy + radius + 4;
            svg.push_str(&format!(
                "    <path d=\"M {},{} Q {},{} {},{} T {},{}\" fill=\"none\" stroke=\"#0284c7\" stroke-width=\"2\" opacity=\"0.8\" stroke-linecap=\"round\" />\n",
                cx - 8, water_y, cx - 4, water_y - 2, cx, water_y, cx + 8, water_y
            ));
        }

        svg
    }

    /// Renders a destroyed / eliminated player silhouette without bulky text.
    fn render_eliminated(svg: &mut String, cx: u32, cy: u32, radius: u32, player_id: PlayerId) {
        svg.push_str(&format!(
            "    <!-- Eliminated Player P{} -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\" fill=\"#f1f5f9\" stroke=\"#cbd5e1\" stroke-width=\"1.5\" />\n",
            player_id.0
        ));
        // Crossed diagonal red lines
        let arm = radius / 2;
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ef4444\" stroke-width=\"2\" stroke-linecap=\"round\" opacity=\"0.7\" />\n",
            cx - arm, cy - arm, cx + arm, cy + arm
        ));
        svg.push_str(&format!(
            "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ef4444\" stroke-width=\"2\" stroke-linecap=\"round\" opacity=\"0.7\" />\n",
            cx + arm, cy - arm, cx - arm, cy + arm
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
