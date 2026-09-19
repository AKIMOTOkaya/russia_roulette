//! Procedural vector graphics for tactical combat effects, ballistic lasers, and motion trails.

#![forbid(unsafe_code)]
#![allow(clippy::similar_names, clippy::cast_precision_loss)]

use roulette_domain::{
    Direction, GameEvent, GameRecord, GameRecordContent, PlayerCommand, PlayerId, PlayerState,
    Position,
};

use crate::theme::{BoardMetrics, Theme};

/// Procedural vector renderer for combat visual effects.
pub struct EffectsRenderer;

impl EffectsRenderer {
    /// Renders a dynamic laser beam trajectory between two center coordinates.
    #[must_use]
    pub fn render_bullet_trajectory(
        from_cx: u32,
        from_cy: u32,
        to_cx: u32,
        to_cy: u32,
        is_hit: bool,
        is_lethal: bool,
    ) -> String {
        let mut svg = String::with_capacity(1024);

        svg.push_str("  <!-- BALLISTIC LASER TRAJECTORY -->\n");
        svg.push_str("  <g id=\"bullet-trajectory\">\n");

        // 1. Muzzle Flash Starburst at origin
        Self::render_muzzle_flash(&mut svg, from_cx, from_cy);

        // 2. High-energy Laser Tracer
        // Outer glowing halo
        svg.push_str(&format!(
            "    <line x1=\"{from_cx}\" y1=\"{from_cy}\" x2=\"{to_cx}\" y2=\"{to_cy}\" stroke=\"#e11d48\" stroke-width=\"3.5\" opacity=\"0.9\" stroke-linecap=\"round\" filter=\"url(#fx-glow-laser)\" />\n"
        ));
        // Inner intense white beam core
        svg.push_str(&format!(
            "    <line x1=\"{from_cx}\" y1=\"{from_cy}\" x2=\"{to_cx}\" y2=\"{to_cy}\" stroke=\"#ffffff\" stroke-width=\"1.2\" stroke-linecap=\"round\" />\n"
        ));

        // 3. Impact Flash & Spark Burst at destination
        Self::render_impact_burst(&mut svg, to_cx, to_cy, is_hit, is_lethal);

        svg.push_str("  </g>\n");
        svg
    }

    /// Renders a directional movement trail between two grid cells.
    #[must_use]
    pub fn render_movement_trail(
        from_cx: u32,
        from_cy: u32,
        to_cx: u32,
        to_cy: u32,
        color_hex: &str,
    ) -> String {
        let mut svg = String::with_capacity(512);
        svg.push_str("  <!-- MOVEMENT DISPLACEMENT TRAIL -->\n");
        svg.push_str("  <g id=\"movement-trail\">\n");

        // Origin ghost footprint
        svg.push_str(&format!(
            "    <circle cx=\"{from_cx}\" cy=\"{from_cy}\" r=\"4\" fill=\"none\" stroke=\"{color_hex}\" stroke-width=\"1.5\" stroke-dasharray=\"3 2\" opacity=\"0.7\" />\n"
        ));

        // Trail dash line
        svg.push_str(&format!(
            "    <line x1=\"{from_cx}\" y1=\"{from_cy}\" x2=\"{to_cx}\" y2=\"{to_cy}\" stroke=\"{color_hex}\" stroke-width=\"2\" stroke-dasharray=\"6 3\" opacity=\"0.85\" />\n"
        ));

        // Target arrival arrowhead
        let dx = i64::from(to_cx) - i64::from(from_cx);
        let dy = i64::from(to_cy) - i64::from(from_cy);
        let len = ((dx * dx + dy * dy) as f64).sqrt();
        if len > 0.1 {
            let u_x = (dx as f64) / len;
            let u_y = (dy as f64) / len;
            let p_x = -u_y;
            let p_y = u_x;

            let tip_x = f64::from(to_cx) - u_x * 8.0;
            let tip_y = f64::from(to_cy) - u_y * 8.0;
            let left_x = tip_x - u_x * 8.0 + p_x * 5.0;
            let left_y = tip_y - u_y * 8.0 + p_y * 5.0;
            let right_x = tip_x - u_x * 8.0 - p_x * 5.0;
            let right_y = tip_y - u_y * 8.0 - p_y * 5.0;

            svg.push_str(&format!(
                "    <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" fill=\"{color_hex}\" opacity=\"0.9\" />\n",
                tip_x, tip_y, left_x, left_y, right_x, right_y
            ));
        }

        svg.push_str("  </g>\n");
        svg
    }

    /// Automatically scans recent game records and renders active ballistic lasers or movement trails.
    #[must_use]
    pub fn render_recent_records(
        records: &[GameRecord],
        metrics: &BoardMetrics,
        players: &[PlayerState],
    ) -> String {
        if records.is_empty() {
            return String::new();
        }

        let mut svg = String::new();

        // Scan from newest records backwards to find the last action / event pair
        let mut last_action_idx = None;
        for (i, rec) in records.iter().enumerate().rev() {
            if matches!(rec.content, GameRecordContent::Action { .. }) {
                last_action_idx = Some(i);
                break;
            }
        }

        let Some(act_idx) = last_action_idx else {
            return String::new();
        };

        let GameRecordContent::Action { actor_id, command } = &records[act_idx].content else {
            return String::new();
        };

        // Subsequent events up to next turn or end
        let sub_events: Vec<&GameEvent> = records[act_idx + 1..]
            .iter()
            .filter_map(|r| match &r.content {
                GameRecordContent::Event { event } => Some(event),
                _ => None,
            })
            .collect();

        match command {
            PlayerCommand::Move { .. } => {
                for ev in &sub_events {
                    if let GameEvent::Moved { actor_id, from, to } = ev {
                        let (fx, fy) = metrics.cell_center(*from);
                        let (tx, ty) = metrics.cell_center(*to);
                        let pcol = Theme::player_color(*actor_id).to_svg_color();
                        svg.push_str(&Self::render_movement_trail(fx, fy, tx, ty, &pcol));
                        break;
                    }
                }
            }
            PlayerCommand::Shoot { direction } => {
                // Find shooter position
                let shooter = players.iter().find(|p| p.id == *actor_id);
                if let Some(start_pos) = shooter.and_then(|s| s.position) {
                    let (fx, fy) = metrics.cell_center(start_pos);
                    let (tx, ty, is_hit, is_lethal) =
                        Self::resolve_shot_target(start_pos, *direction, &sub_events, metrics);
                    svg.push_str(&Self::render_bullet_trajectory(
                        fx, fy, tx, ty, is_hit, is_lethal,
                    ));
                }
            }
            PlayerCommand::Wait | PlayerCommand::Suicide => {}
        }

        svg
    }

    fn resolve_shot_target(
        start_pos: Position,
        dir: Direction,
        events: &[&GameEvent],
        metrics: &BoardMetrics,
    ) -> (u32, u32, bool, bool) {
        let mut is_hit = false;
        let mut is_lethal = false;
        let mut target_pos = None;

        for ev in events {
            match ev {
                GameEvent::PlayerEliminated { player_id, .. } => {
                    is_hit = true;
                    is_lethal = true;
                    target_pos = Self::find_event_target_pos(*player_id, events);
                }
                GameEvent::ShieldConsumed { player_id, .. } => {
                    is_hit = true;
                    target_pos = Self::find_event_target_pos(*player_id, events);
                }
                GameEvent::TerrainChanged { position, .. } => {
                    is_hit = true;
                    target_pos = Some(*position);
                }
                GameEvent::ShotMissed { .. } | GameEvent::EmptyChamber { .. } => {
                    is_hit = false;
                }
                _ => {}
            }
        }

        // If target pos determined from events, use its center
        if let Some(t_pos) = target_pos {
            let (tx, ty) = metrics.cell_center(t_pos);
            return (tx, ty, is_hit, is_lethal);
        }

        // Fallback: estimate ray end along direction (stay inside board ground plate)
        let (tx, ty) = match dir {
            Direction::Up => {
                let (cx, _) = metrics.cell_center(start_pos);
                (cx, metrics.padding + 6)
            }
            Direction::Down => {
                let (cx, _) = metrics.cell_center(start_pos);
                (cx, metrics.total_height() - metrics.padding - 6)
            }
            Direction::Left => {
                let (_, cy) = metrics.cell_center(start_pos);
                (metrics.padding + 6, cy)
            }
            Direction::Right => {
                let (_, cy) = metrics.cell_center(start_pos);
                (metrics.total_width() - metrics.padding - 6, cy)
            }
        };

        (tx, ty, is_hit, is_lethal)
    }

    fn find_event_target_pos(target_id: PlayerId, events: &[&GameEvent]) -> Option<Position> {
        for ev in events {
            match ev {
                GameEvent::DramaticEvent {
                    position: Some(pos),
                    ..
                } => return Some(*pos),
                GameEvent::TerrainChanged { position, .. } => return Some(*position),
                _ => {}
            }
        }
        let _ = target_id;
        None
    }

    fn render_muzzle_flash(svg: &mut String, cx: u32, cy: u32) {
        let arm = 8;
        svg.push_str(&format!(
            "    <!-- Muzzle Flash -->\n    <polygon points=\"{cx},{} {},{cy} {cx},{} {},{cy}\" fill=\"#f59e0b\" filter=\"url(#fx-glow-amber)\" />\n",
            cy.saturating_sub(arm), cx + arm, cy + arm, cx.saturating_sub(arm)
        ));
        svg.push_str(&format!(
            "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"2.5\" fill=\"#ffffff\" />\n"
        ));
    }

    fn render_impact_burst(svg: &mut String, cx: u32, cy: u32, is_hit: bool, is_lethal: bool) {
        if is_hit {
            let shock_color = if is_lethal { "#e11d48" } else { "#ea580c" };
            svg.push_str(&format!(
                "    <!-- Impact Burst Shockwave -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"12\" fill=\"none\" stroke=\"{shock_color}\" stroke-width=\"2\" stroke-dasharray=\"4 2\" filter=\"url(#fx-glow-laser)\" />\n"
            ));
            // Radiating sparks
            let spark_len = 6;
            svg.push_str(&format!(
                "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#f59e0b\" stroke-width=\"1.5\" stroke-linecap=\"round\" />\n",
                cx - spark_len, cy - spark_len, cx + spark_len, cy + spark_len
            ));
            svg.push_str(&format!(
                "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#f59e0b\" stroke-width=\"1.5\" stroke-linecap=\"round\" />\n",
                cx + spark_len, cy - spark_len, cx - spark_len, cy + spark_len
            ));
            svg.push_str(&format!(
                "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"3\" fill=\"#ffffff\" />\n"
            ));
        } else {
            // Dissipation miss ripples
            svg.push_str(&format!(
                "    <!-- Dissipation Ripple -->\n    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"8\" fill=\"none\" stroke=\"#94a3b8\" stroke-width=\"1.5\" stroke-dasharray=\"3 3\" opacity=\"0.7\" />\n"
            ));
        }
    }
}
