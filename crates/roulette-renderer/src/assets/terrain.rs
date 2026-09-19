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
        let mut svg = String::with_capacity(1024);
        let cx = px + cell_size / 2;
        let cy = py + cell_size / 2;

        match terrain {
            Terrain::Empty => {
                // Continuous seamless ground: do NOT render a bounding box or tile background.
                // Just an extremely subtle micro-dot at the cell center for spatial reference.
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"1.5\" fill=\"#cbd5e1\" opacity=\"0.5\" />\n"
                ));
            }
            Terrain::Wall => {
                // Solid slate masonry barrier (H:2) - tactile obstacle block, no text
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-wall)\" stroke=\"#334155\" stroke-width=\"1\" filter=\"url(#fx-drop-shadow)\" />\n"
                ));
                // Masonry horizontal mortar seams
                let h1 = py + cell_size / 3;
                let h2 = py + (cell_size * 2) / 3;
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{h1}\" x2=\"{}\" y2=\"{h1}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.4\" />\n",
                    px + 6, px + cell_size - 6
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{h2}\" x2=\"{}\" y2=\"{h2}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.4\" />\n",
                    px + 6, px + cell_size - 6
                ));
                // Masonry vertical staggered seams
                svg.push_str(&format!(
                    "    <line x1=\"{cx}\" y1=\"{}\" x2=\"{cx}\" y2=\"{h1}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.4\" />\n",
                    py + 6
                ));
                let v2 = px + cell_size / 4;
                let v3 = px + (cell_size * 3) / 4;
                svg.push_str(&format!(
                    "    <line x1=\"{v2}\" y1=\"{h1}\" x2=\"{v2}\" y2=\"{h2}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.4\" />\n"
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{v3}\" y1=\"{h2}\" x2=\"{v3}\" y2=\"{}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.4\" />\n",
                    py + cell_size - 6
                ));
                // Central reinforced diamond armor crest
                svg.push_str(&format!(
                    "    <polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#f8fafc\" opacity=\"0.85\" />\n",
                    cx, cy - 7, cx + 6, cy, cx, cy + 7, cx - 6, cy
                ));
            }
            Terrain::Crate => {
                // Warm caramel wood obstacle (H:1) - diagonal cross bracing, no text
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-crate)\" stroke=\"#f97316\" stroke-width=\"1\" filter=\"url(#fx-drop-shadow)\" />\n"
                ));
                // Timber cross bracing
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#c2410c\" stroke-width=\"2.5\" opacity=\"0.7\" stroke-linecap=\"round\" />\n",
                    px + 8, py + 8, px + cell_size - 8, py + cell_size - 8
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#c2410c\" stroke-width=\"2.5\" opacity=\"0.7\" stroke-linecap=\"round\" />\n",
                    px + cell_size - 8, py + 8, px + 8, py + cell_size - 8
                ));
                // Center fastener stud
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"14\" height=\"14\" rx=\"3\" fill=\"#fff7ed\" stroke=\"#ea580c\" stroke-width=\"1.5\" />\n",
                    cx - 7, cy - 7
                ));
            }
            Terrain::Water => {
                // Serene soft sky blue dynamic water pool - wave curves, no text
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-water)\" stroke=\"#7dd3fc\" stroke-width=\"1\" />\n"
                ));
                let w1_y = py + cell_size / 3;
                svg.push_str(&format!(
                    "    <path d=\"M {} {} Q {} {} {} {} T {} {}\" fill=\"none\" stroke=\"#0284c7\" stroke-width=\"2\" opacity=\"0.7\" stroke-linecap=\"round\" />\n",
                    px + 10, w1_y, px + cell_size / 4, w1_y - 4, px + cell_size / 2, w1_y, px + cell_size - 10, w1_y
                ));
                let w2_y = py + (cell_size * 2) / 3;
                svg.push_str(&format!(
                    "    <path d=\"M {} {} Q {} {} {} {} T {} {}\" fill=\"none\" stroke=\"#0284c7\" stroke-width=\"2\" opacity=\"0.7\" stroke-linecap=\"round\" />\n",
                    px + 10, w2_y, px + (cell_size * 3) / 4, w2_y + 4, px + cell_size / 2, w2_y, px + cell_size - 10, w2_y
                ));
                svg.push_str(&format!(
                    "    <ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"8\" ry=\"3\" fill=\"none\" stroke=\"#0369a1\" stroke-width=\"1.2\" opacity=\"0.5\" />\n"
                ));
            }
            Terrain::Ice => {
                // Crystalline glacial ice surface - fractured rays and gleams, no text
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-ice)\" stroke=\"#bae6fd\" stroke-width=\"1\" />\n"
                ));
                svg.push_str(&format!(
                    "    <path d=\"M {} {} L {cx} {cy} L {} {} M {cx} {cy} L {} {}\" fill=\"none\" stroke=\"#0284c7\" stroke-width=\"1.5\" opacity=\"0.6\" stroke-linecap=\"round\" />\n",
                    px + 10, py + 10, px + cell_size - 10, py + 14, px + 12, py + cell_size - 10
                ));
                svg.push_str(&format!(
                    "    <polygon points=\"{},{} {},{} {},{}\" fill=\"#ffffff\" opacity=\"0.9\" />\n",
                    cx - 6, cy - 12, cx + 2, cy - 16, cx - 1, cy - 7
                ));
                svg.push_str(&format!(
                    "    <polygon points=\"{},{} {},{} {},{}\" fill=\"#ffffff\" opacity=\"0.8\" />\n",
                    cx + 6, cy + 6, cx + 14, cy + 10, cx + 8, cy + 14
                ));
            }
            Terrain::Mine => {
                // Concealed buried minefield - concentric hazard perimeter, no text
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-mine)\" stroke=\"#f87171\" stroke-width=\"1\" />\n"
                ));
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"16\" fill=\"none\" stroke=\"#ef4444\" stroke-width=\"1.2\" stroke-dasharray=\"4 3\" opacity=\"0.7\" />\n"
                ));
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"8\" fill=\"none\" stroke=\"#dc2626\" stroke-width=\"1.5\" />\n"
                ));
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"3.5\" fill=\"#dc2626\" />\n"
                ));
            }
            Terrain::Medkit => {
                // Fresh mint shield / medical pod - clean cross & shield ring, no text
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-medkit)\" stroke=\"#34d399\" stroke-width=\"1\" />\n"
                ));
                let arm_len = 8;
                let arm_w = 4;
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"1\" fill=\"#059669\" />\n",
                    cx - arm_w / 2, cy - arm_len, arm_w, arm_len * 2
                ));
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"1\" fill=\"#059669\" />\n",
                    cx - arm_len, cy - arm_w / 2, arm_len * 2, arm_w
                ));
                svg.push_str(&format!(
                    "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"14\" fill=\"none\" stroke=\"#10b981\" stroke-width=\"1.2\" opacity=\"0.6\" />\n"
                ));
            }
            Terrain::HighGround => {
                // Pastel warm gold elevation advantage - contour ring & upward chevrons, no text
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" rx=\"8\" fill=\"url(#grad-highground)\" stroke=\"#facc15\" stroke-width=\"1\" />\n"
                ));
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"none\" stroke=\"#ca8a04\" stroke-width=\"1\" opacity=\"0.5\" />\n",
                    px + 10, py + 10, cell_size - 20, cell_size - 20
                ));
                svg.push_str(&format!(
                    "    <path d=\"M {} {} L {cx} {} L {} {}\" fill=\"none\" stroke=\"#a16207\" stroke-width=\"2.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />\n",
                    cx - 8, cy + 2, cy - 6, cx + 8, cy + 2
                ));
                svg.push_str(&format!(
                    "    <path d=\"M {} {} L {cx} {} L {} {}\" fill=\"none\" stroke=\"#a16207\" stroke-width=\"2.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />\n",
                    cx - 8, cy + 8, cy, cx + 8, cy + 8
                ));
            }
        }

        svg
    }
}
