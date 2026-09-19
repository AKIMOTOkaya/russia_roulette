//! Pure vector procedural graphics for game terrain tiles.

#![forbid(unsafe_code)]

use roulette_domain::Terrain;

/// Renders high-fidelity procedural vector graphics for a terrain tile.
pub struct TerrainAssetRenderer;

impl TerrainAssetRenderer {
    /// Renders the complete vector markup for a square terrain tile at `(px, py)` with `cell_size`.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render(px: u32, py: u32, cell_size: u32, terrain: Terrain) -> String {
        let mut svg = String::with_capacity(2048);
        let cx = px + cell_size / 2;
        let cy = py + cell_size / 2;

        match terrain {
            Terrain::Empty => {
                // Tactical empty ground with corner reticles
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"#161d2a\" stroke=\"#263346\" stroke-width=\"1\" />\n"
                ));
                // 4 corner crosshairs
                let arm = 6;
                // Top-left
                svg.push_str(&format!(
                    "    <path d=\"M {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\" />\n",
                    px + 4, py + 4 + arm, px + 4, py + 4, px + 4 + arm, py + 4
                ));
                // Top-right
                svg.push_str(&format!(
                    "    <path d=\"M {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\" />\n",
                    px + cell_size - 4 - arm, py + 4, px + cell_size - 4, py + 4, px + cell_size - 4, py + 4 + arm
                ));
                // Bottom-left
                svg.push_str(&format!(
                    "    <path d=\"M {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\" />\n",
                    px + 4, py + cell_size - 4 - arm, px + 4, py + cell_size - 4, px + 4 + arm, py + cell_size - 4
                ));
                // Bottom-right
                svg.push_str(&format!(
                    "    <path d=\"M {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\" />\n",
                    px + cell_size - 4 - arm, py + cell_size - 4, px + cell_size - 4, py + cell_size - 4, px + cell_size - 4, py + cell_size - 4 - arm
                ));
                // Center micro dot
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"1.5\" fill=\"#334155\" />\n"
                ));
            }
            Terrain::Wall => {
                // Reinforced stone masonry wall with bevel and H:2 armor badge
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-wall)\" stroke=\"#94a3b8\" stroke-width=\"1.5\" filter=\"url(#fx-drop-shadow)\" />\n"
                ));
                // Inner 3D bevel lip
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"none\" stroke=\"#475569\" stroke-width=\"1\" opacity=\"0.6\" />\n",
                    px + 3, py + 3, cell_size - 6, cell_size - 6
                ));
                // Masonry horizontal mortar seams
                let h1 = py + cell_size / 3;
                let h2 = py + (cell_size * 2) / 3;
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{h1}\" x2=\"{}\" y2=\"{h1}\" stroke=\"#1e293b\" stroke-width=\"2\" />\n",
                    px + 4, px + cell_size - 4
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{h2}\" x2=\"{}\" y2=\"{h2}\" stroke=\"#1e293b\" stroke-width=\"2\" />\n",
                    px + 4, px + cell_size - 4
                ));
                // Vertical joints
                svg.push_str(&format!(
                    "    <line x1=\"{cx}\" y1=\"{}\" x2=\"{cx}\" y2=\"{h1}\" stroke=\"#1e293b\" stroke-width=\"2\" />\n",
                    py + 4
                ));
                let v2 = px + cell_size / 4;
                let v3 = px + (cell_size * 3) / 4;
                svg.push_str(&format!(
                    "    <line x1=\"{v2}\" y1=\"{h1}\" x2=\"{v2}\" y2=\"{h2}\" stroke=\"#1e293b\" stroke-width=\"2\" />\n"
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{v3}\" y1=\"{h2}\" x2=\"{v3}\" y2=\"{}\" stroke=\"#1e293b\" stroke-width=\"2\" />\n",
                    py + cell_size - 4
                ));
                // Armor Hardness Crest: [H:2] with double diamond
                let badge_y = py + cell_size - 10;
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"44\" height=\"16\" rx=\"4\" fill=\"#0f172a\" stroke=\"#94a3b8\" stroke-width=\"1\" />\n",
                    cx - 22, badge_y - 12
                ));
                svg.push_str(&format!(
                    "    <text x=\"{cx}\" y=\"{}\" font-size=\"10\" font-weight=\"900\" fill=\"#f8fafc\" text-anchor=\"middle\" letter-spacing=\"0.5\">◆◆ H:2</text>\n",
                    badge_y
                ));
            }
            Terrain::Crate => {
                // Military wooden crate with corner iron brackets and diagonal bracing
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-crate)\" stroke=\"#b45309\" stroke-width=\"1.5\" />\n"
                ));
                // Wooden plank vertical seam lines
                let p1 = px + cell_size / 3;
                let p2 = px + (cell_size * 2) / 3;
                svg.push_str(&format!(
                    "    <line x1=\"{p1}\" y1=\"{}\" x2=\"{p1}\" y2=\"{}\" stroke=\"#291506\" stroke-width=\"1.5\" />\n",
                    py + 4, py + cell_size - 4
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{p2}\" y1=\"{}\" x2=\"{p2}\" y2=\"{}\" stroke=\"#291506\" stroke-width=\"1.5\" />\n",
                    py + 4, py + cell_size - 4
                ));
                // Diagonal metal tension cross
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#92400e\" stroke-width=\"3\" opacity=\"0.8\" />\n",
                    px + 8, py + 8, px + cell_size - 8, py + cell_size - 8
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#92400e\" stroke-width=\"3\" opacity=\"0.8\" />\n",
                    px + cell_size - 8, py + 8, px + 8, py + cell_size - 8
                ));
                // Center steel plate with H:1 badge
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"14\" fill=\"#1e140a\" stroke=\"#f59e0b\" stroke-width=\"1.5\" />\n"
                ));
                svg.push_str(&format!(
                    "    <text x=\"{cx}\" y=\"{}\" font-size=\"10\" font-weight=\"bold\" fill=\"#fbbf24\" text-anchor=\"middle\">◆ H:1</text>\n",
                    cy + 3
                ));
            }
            Terrain::Water => {
                // Deep dynamic water with concentric layered sine wave currents
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-water)\" stroke=\"#0284c7\" stroke-width=\"1.5\" />\n"
                ));
                // Wave current 1
                let w1_y = py + cell_size / 3;
                svg.push_str(&format!(
                    "    <path d=\"M {} {} Q {} {} {} {} T {} {}\" fill=\"none\" stroke=\"#38bdf8\" stroke-width=\"2\" opacity=\"0.7\" />\n",
                    px + 8, w1_y, px + cell_size / 4, w1_y - 4, px + cell_size / 2, w1_y, px + cell_size - 8, w1_y
                ));
                // Wave current 2
                let w2_y = py + (cell_size * 2) / 3;
                svg.push_str(&format!(
                    "    <path d=\"M {} {} Q {} {} {} {} T {} {}\" fill=\"none\" stroke=\"#0284c7\" stroke-width=\"2.5\" opacity=\"0.8\" />\n",
                    px + 8, w2_y, px + (cell_size * 3) / 4, w2_y + 4, px + cell_size / 2, w2_y, px + cell_size - 8, w2_y
                ));
                // Water ripple crest in center
                svg.push_str(&format!(
                    "    <ellipse cx=\"{cx}\" cy=\"{}\" rx=\"12\" ry=\"4\" fill=\"none\" stroke=\"#7dd3fc\" stroke-width=\"1.2\" opacity=\"0.6\" />\n",
                    cy - 2
                ));
                // Tactical label
                svg.push_str(&format!(
                    "    <text x=\"{cx}\" y=\"{}\" font-size=\"10\" font-weight=\"bold\" fill=\"#e0f2fe\" text-anchor=\"middle\" opacity=\"0.9\">深水低洼</text>\n",
                    py + cell_size - 8
                ));
            }
            Terrain::Ice => {
                // Crystalline glacial ice with faceted fracture rays and frost sheen
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-ice)\" stroke=\"#38bdf8\" stroke-width=\"1.5\" />\n"
                ));
                // Geometric fracture cracks
                svg.push_str(&format!(
                    "    <path d=\"M {} {} L {cx} {cy} L {} {} M {cx} {cy} L {} {}\" fill=\"none\" stroke=\"#e0f2fe\" stroke-width=\"1.8\" opacity=\"0.85\" />\n",
                    px + 10, py + 8, px + cell_size - 12, py + 14, px + 14, py + cell_size - 10
                ));
                // Specular glint facets
                svg.push_str(&format!(
                    "    <polygon points=\"{},{} {},{} {},{}\" fill=\"#ffffff\" opacity=\"0.4\" />\n",
                    cx - 8, cy - 14, cx + 2, cy - 20, cx - 2, cy - 8
                ));
                svg.push_str(&format!(
                    "    <polygon points=\"{},{} {},{} {},{}\" fill=\"#ffffff\" opacity=\"0.35\" />\n",
                    cx + 6, cy + 8, cx + 18, cy + 12, cx + 10, cy + 18
                ));
                // Tactical label
                svg.push_str(&format!(
                    "    <text x=\"{cx}\" y=\"{}\" font-size=\"10\" font-weight=\"bold\" fill=\"#f0fdf4\" text-anchor=\"middle\" letter-spacing=\"0.5\">极滑冰面</text>\n",
                    py + cell_size - 8
                ));
            }
            Terrain::Mine => {
                // High-explosive buried landmine with hazard perimeter and pulsating LED
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-mine)\" stroke=\"#dc2626\" stroke-width=\"1.5\" />\n"
                ));
                // Diagonal hazard hatch stripes around outer border
                let d = 10;
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ef4444\" stroke-width=\"2\" opacity=\"0.4\" />\n",
                    px + 4, py + d, px + d, py + 4
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ef4444\" stroke-width=\"2\" opacity=\"0.4\" />\n",
                    px + cell_size - d, py + 4, px + cell_size - 4, py + d
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ef4444\" stroke-width=\"2\" opacity=\"0.4\" />\n",
                    px + 4, py + cell_size - d, px + d, py + cell_size - 4
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ef4444\" stroke-width=\"2\" opacity=\"0.4\" />\n",
                    px + cell_size - d, py + cell_size - 4, px + cell_size - 4, py + cell_size - d
                ));
                // Center steel pressure detonator ring
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"16\" fill=\"#1c1917\" stroke=\"#f87171\" stroke-width=\"1.8\" />\n"
                ));
                // Danger pulsating red core LED
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"7\" fill=\"#ef4444\" filter=\"url(#fx-glow-laser)\" />\n"
                ));
                // Danger warning text
                svg.push_str(&format!(
                    "    <text x=\"{cx}\" y=\"{}\" font-size=\"9\" font-weight=\"900\" fill=\"#fca5a5\" text-anchor=\"middle\" letter-spacing=\"1\">暗雷阵地</text>\n",
                    py + cell_size - 8
                ));
            }
            Terrain::Medkit => {
                // High-tech hexagonal nano-shield pickup container
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-medkit)\" stroke=\"#10b981\" stroke-width=\"1.5\" />\n"
                ));
                // Hexagonal energy pod perimeter
                let r = 18;
                let mut hex_pts = String::new();
                for i in 0..6 {
                    let angle = std::f64::consts::PI / 3.0 * (f64::from(i) + 0.5);
                    let hx = f64::from(cx) + f64::from(r) * angle.cos();
                    let hy = f64::from(cy) + f64::from(r) * angle.sin();
                    hex_pts.push_str(&format!("{hx:.1},{hy:.1} "));
                }
                svg.push_str(&format!(
                    "    <polygon points=\"{}\" fill=\"#022c22\" stroke=\"#34d399\" stroke-width=\"1.5\" />\n",
                    hex_pts.trim()
                ));
                // Glowing Emerald Medical / Shield Cross
                let arm_len = 9;
                let arm_w = 4;
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"1\" fill=\"#6ee7b7\" filter=\"url(#fx-glow-cyan)\" />\n",
                    cx - arm_w / 2, cy - arm_len, arm_w, arm_len * 2
                ));
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"1\" fill=\"#6ee7b7\" filter=\"url(#fx-glow-cyan)\" />\n",
                    cx - arm_len, cy - arm_w / 2, arm_len * 2, arm_w
                ));
                // Label
                svg.push_str(&format!(
                    "    <text x=\"{cx}\" y=\"{}\" font-size=\"9\" font-weight=\"bold\" fill=\"#a7f3d0\" text-anchor=\"middle\">护盾补给</text>\n",
                    py + cell_size - 8
                ));
            }
            Terrain::HighGround => {
                // Topographic tactical vantage point with contour elevation steps
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-highground)\" stroke=\"#ca8a04\" stroke-width=\"1.5\" />\n"
                ));
                // Elevation contour line 1 (base plateau)
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"none\" stroke=\"#eab308\" stroke-width=\"1.2\" opacity=\"0.5\" />\n",
                    px + 8, py + 8, cell_size - 16, cell_size - 16
                ));
                // Elevation contour line 2 (summit crest)
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"#422006\" stroke=\"#facc15\" stroke-width=\"1.5\" opacity=\"0.9\" />\n",
                    cx - 16, cy - 16, 32, 32
                ));
                // Dual Vantage Chevrons ▲▲
                svg.push_str(&format!(
                    "    <path d=\"M {} {} L {cx} {} L {} {}\" fill=\"none\" stroke=\"#fde047\" stroke-width=\"2\" />\n",
                    cx - 8, cy - 2, cy - 8, cx + 8, cy - 2
                ));
                svg.push_str(&format!(
                    "    <path d=\"M {} {} L {cx} {} L {} {}\" fill=\"none\" stroke=\"#fde047\" stroke-width=\"2\" />\n",
                    cx - 8, cy + 4, cy - 2, cx + 8, cy + 4
                ));
                // Label: +1 Range
                svg.push_str(&format!(
                    "    <text x=\"{cx}\" y=\"{}\" font-size=\"9\" font-weight=\"900\" fill=\"#fef08a\" text-anchor=\"middle\">▲ +1 射程</text>\n",
                    py + cell_size - 8
                ));
            }
        }

        svg
    }
}
