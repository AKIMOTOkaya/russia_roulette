//! Unified vector asset atlas / tilesheet generator.
//!
//! Provides a visual showcase and catalog of all procedural game assets:
//! terrain tiles, player tokens, combat effects, and status glyphs.

#![forbid(unsafe_code)]

use roulette_domain::{
    Direction, PlayerId, PlayerKind, PlayerState, PlayerStatus, Terrain, Weather,
};

use crate::RenderError;
use crate::assets::effects::EffectsRenderer;
use crate::assets::player::PlayerAssetRenderer;
use crate::assets::terrain::TerrainAssetRenderer;
use crate::theme::Theme;

/// Renderer for the complete procedural asset catalog / tilesheet.
pub struct TileSheetRenderer;

impl TileSheetRenderer {
    /// Renders the complete asset catalog as an SVG document.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render_svg() -> String {
        let width = 1040;
        let height = 820;

        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
        );

        // Defs
        svg.push_str(crate::assets::render_shared_defs());

        // Background (Canvas white)
        svg.push_str(&format!(
            "  <rect width=\"{width}\" height=\"{height}\" fill=\"{}\" />\n",
            Theme::CANVAS_BG.to_svg_color()
        ));

        // Header
        svg.push_str("  <g id=\"sheet-header\">\n");
        svg.push_str("    <text x=\"40\" y=\"48\" font-size=\"22\" font-weight=\"900\" fill=\"#0f172a\" letter-spacing=\"1.5\">RUSSIAN ROULETTE // PROCEDURAL VECTOR ASSET CATALOG</text>\n");
        svg.push_str("    <text x=\"40\" y=\"72\" font-size=\"13\" fill=\"#64748b\">Pure geometric vector shaders &amp; tokens • Zero emoji dependencies • Fully scalable</text>\n");
        svg.push_str(&format!(
            "    <line x1=\"40\" y1=\"84\" x2=\"{}\" y2=\"84\" stroke=\"#e2e8f0\" stroke-width=\"1.5\" />\n",
            width - 40
        ));
        svg.push_str("  </g>\n\n");

        // SECTION 1: TERRAIN TILES (8 tiles)
        let s1_y = 106;
        svg.push_str(&format!(
            "  <g id=\"section-terrain\">\n    <text x=\"40\" y=\"{s1_y}\" font-size=\"14\" font-weight=\"bold\" fill=\"#0284c7\" letter-spacing=\"1\">[01] TERRAIN TILES (8 TYPES // 96×96 UNIT)</text>\n"
        ));

        let terrains = [
            (Terrain::Empty, "EMPTY // 平原空地", "walkable ground"),
            (Terrain::Wall, "WALL // 掩体墙", "H:2 / absorbs 2"),
            (Terrain::Crate, "CRATE // 木箱", "H:1 / breakable"),
            (Terrain::Water, "WATER // 深水", "drowning hazard"),
            (Terrain::Ice, "ICE // 冰面", "sliding surface"),
            (Terrain::Mine, "MINE // 暗雷", "lethal blast"),
            (Terrain::Medkit, "SHIELD // 护盾舱", "blocks 1 lethal"),
            (
                Terrain::HighGround,
                "HIGHGROUND // 高地",
                "range +1 / vantage",
            ),
        ];

        let cell_size = 96;
        let tile_gap = 24;
        let start_x = 40;
        let tile_y = s1_y + 16;

        for (i, (terr, title, desc)) in terrains.iter().enumerate() {
            let col = (i % 4) as u32;
            let row = (i / 4) as u32;
            let px = start_x + col * (cell_size + tile_gap + 120);
            let py = tile_y + row * (cell_size + 48);

            // For Empty demo, draw a subtle plate container so the ground dot is contextualized
            if *terr == Terrain::Empty {
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"#f8fafc\" stroke=\"#e2e8f0\" stroke-width=\"1.5\" />\n"
                ));
            }

            // Render procedural terrain
            svg.push_str(&TerrainAssetRenderer::render(px, py, cell_size, *terr));

            // Labels to the right of tile
            let text_x = px + cell_size + 12;
            svg.push_str(&format!(
                "    <text x=\"{text_x}\" y=\"{}\" font-size=\"12\" font-weight=\"bold\" fill=\"#0f172a\">{title}</text>\n",
                py + 34
            ));
            svg.push_str(&format!(
                "    <text x=\"{text_x}\" y=\"{}\" font-size=\"11\" fill=\"#64748b\">{desc}</text>\n",
                py + 54
            ));
        }
        svg.push_str("  </g>\n\n");

        // SECTION 2: PLAYER TOKENS & STATES
        let s2_y = tile_y + 2 * (cell_size + 48) + 16;
        svg.push_str(&format!(
            "  <g id=\"section-players\">\n    <text x=\"40\" y=\"{s2_y}\" font-size=\"14\" font-weight=\"bold\" fill=\"#ea580c\" letter-spacing=\"1\">[02] COMBATANT TOKENS (P1-P6 // HUMAN &amp; BOT // STATES)</text>\n"
        ));

        let players_demo = [
            (
                PlayerId(1),
                PlayerKind::Human,
                PlayerStatus::Alive,
                false,
                false,
                "P1 HUMAN // 真人战术标",
            ),
            (
                PlayerId(2),
                PlayerKind::Bot,
                PlayerStatus::Alive,
                false,
                false,
                "P2 BOT // 机器人战术标",
            ),
            (
                PlayerId(3),
                PlayerKind::Human,
                PlayerStatus::Alive,
                true,
                false,
                "P3 HUMAN // 行动回合金色指示",
            ),
            (
                PlayerId(4),
                PlayerKind::Bot,
                PlayerStatus::Alive,
                false,
                true,
                "P4 BOT // 激活护盾力场罩",
            ),
            (
                PlayerId(5),
                PlayerKind::Human,
                PlayerStatus::Eliminated,
                false,
                false,
                "P5 HUMAN // 阵亡战损标",
            ),
            (
                PlayerId(6),
                PlayerKind::Bot,
                PlayerStatus::Alive,
                true,
                true,
                "P6 BOT // 护盾+行动复合态",
            ),
        ];

        let p_start_y = s2_y + 20;
        let token_r = 28;
        for (i, (pid, kind, status, is_turn, has_shield, label)) in players_demo.iter().enumerate()
        {
            let col = (i % 3) as u32;
            let row = (i / 3) as u32;
            let cx = start_x + 40 + col * 320;
            let cy = p_start_y + 36 + row * 84;

            let player = PlayerState {
                id: *pid,
                name: format!("Player {}", pid.0),
                kind: *kind,
                status: *status,
                position: None,
                has_shield: *has_shield,
                water_turns: 0,
                consecutive_shots: 0,
            };

            svg.push_str(&PlayerAssetRenderer::render(
                cx,
                cy,
                token_r,
                &player,
                *is_turn,
                Some(Direction::Up),
            ));

            let label_x = cx + token_r + 20;
            svg.push_str(&format!(
                "    <text x=\"{label_x}\" y=\"{}\" font-size=\"12\" font-weight=\"bold\" fill=\"#0f172a\">{label}</text>\n",
                cy + 4
            ));
        }
        svg.push_str("  </g>\n\n");

        // SECTION 3: COMBAT EFFECTS & WEATHER BADGES
        let s3_y = p_start_y + 2 * 84 + 36;
        svg.push_str(&format!(
            "  <g id=\"section-effects\">\n    <text x=\"40\" y=\"{s3_y}\" font-size=\"14\" font-weight=\"bold\" fill=\"#e11d48\" letter-spacing=\"1\">[03] BALLISTIC LASER TRAJECTORY &amp; MOVEMENT TRAILS</text>\n"
        ));

        let eff_y = s3_y + 30;
        // Demo 1: High-energy laser hit
        svg.push_str(&EffectsRenderer::render_bullet_trajectory(
            60, eff_y, 280, eff_y, true, true,
        ));
        svg.push_str(&format!(
            "    <text x=\"300\" y=\"{}\" font-size=\"11\" fill=\"#64748b\">弹道轨迹 // 枪口闪光 + 极速激光束 + 爆点击破</text>\n",
            eff_y + 4
        ));

        // Demo 2: Movement displacement trail
        let trail_y = eff_y + 36;
        let cyan_hex = Theme::ACCENT_CYAN.to_svg_color();
        svg.push_str(&EffectsRenderer::render_movement_trail(
            60, trail_y, 280, trail_y, &cyan_hex,
        ));
        svg.push_str(&format!(
            "    <text x=\"300\" y=\"{}\" font-size=\"11\" fill=\"#64748b\">战术位移 // 虚线滑行轨迹 + 航向箭簇</text>\n",
            trail_y + 4
        ));

        // Weather Badges showcase
        let w_x = 640;
        let weathers = [
            Weather::Clear,
            Weather::Blizzard,
            Weather::Heatwave,
            Weather::DenseFog,
        ];
        svg.push_str(&format!(
            "    <text x=\"{w_x}\" y=\"{s3_y}\" font-size=\"14\" font-weight=\"bold\" fill=\"#0284c7\" letter-spacing=\"1\">[04] TACTICAL WEATHER BADGES</text>\n"
        ));
        for (i, w) in weathers.iter().enumerate() {
            let row_y = eff_y + (i as u32) * 26;
            let (icon, label, color) = Theme::weather_style(*w);
            let col_hex = color.to_svg_color();
            svg.push_str(&format!(
                "    <circle cx=\"{w_x}\" cy=\"{}\" r=\"4\" fill=\"{col_hex}\" />\n",
                row_y - 4
            ));
            svg.push_str(&format!(
                "    <text x=\"{}\" y=\"{row_y}\" font-size=\"12\" fill=\"{col_hex}\">{icon} {label}</text>\n",
                w_x + 14
            ));
        }

        svg.push_str("  </g>\n");
        svg.push_str("</svg>\n");

        svg
    }

    /// Renders the asset catalog as high-resolution PNG image bytes.
    ///
    /// # Errors
    /// Returns `RenderError` if rasterization fails.
    pub fn render_png() -> Result<Vec<u8>, RenderError> {
        let svg_str = Self::render_svg();
        crate::render_png_from_svg(&svg_str)
    }
}
