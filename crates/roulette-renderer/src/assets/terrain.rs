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
            Terrain::Plain => {
                // Continuous seamless ground: naturally transparent against the board plate.
                // No bounding box, no stroke, no distracting dots.
            }
            Terrain::Water => {
                // Soft sky blue water ground tint: seamless pool without outer stroke or box
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" fill=\"url(#grad-water)\" />\n"
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
                // Glacial ice ground tint: seamless surface without outer stroke or box
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" fill=\"url(#grad-ice)\" />\n"
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
            Terrain::HighGround => {
                // High ground elevation tint: seamless area without outer stroke or box
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" fill=\"url(#grad-highground)\" />\n"
                ));
                // Elegant curved elevation contour lines
                svg.push_str(&format!(
                    "    <path d=\"M {} {} Q {cx} {} {} {}\" fill=\"none\" stroke=\"#ca8a04\" stroke-width=\"1.5\" opacity=\"0.5\" stroke-linecap=\"round\" />\n",
                    cx - 20, cy + 18, cy + 4, cx + 20, cy + 18
                ));
                svg.push_str(&format!(
                    "    <path d=\"M {} {} Q {cx} {} {} {}\" fill=\"none\" stroke=\"#ca8a04\" stroke-width=\"1.5\" opacity=\"0.6\" stroke-linecap=\"round\" />\n",
                    cx - 14, cy + 10, cy - 2, cx + 14, cy + 10
                ));
                // Peak vantage chevrons
                svg.push_str(&format!(
                    "    <path d=\"M {} {} L {cx} {} L {} {}\" fill=\"none\" stroke=\"#a16207\" stroke-width=\"2.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />\n",
                    cx - 8, cy + 2, cy - 6, cx + 8, cy + 2
                ));
            }
        }

        svg
    }
}
