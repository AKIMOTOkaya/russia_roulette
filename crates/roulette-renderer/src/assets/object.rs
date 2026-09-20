//! Pure vector procedural graphics for game map objects (cover, traps, pickups).

#![forbid(unsafe_code)]

use roulette_domain::MapObject;

/// Renders high-fidelity procedural vector graphics for a map object.
pub struct ObjectAssetRenderer;

impl ObjectAssetRenderer {
    /// Renders the complete vector markup for a map object at `(px, py)` with `cell_size`.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn render(px: u32, py: u32, cell_size: u32, object: MapObject) -> String {
        let mut svg = String::with_capacity(1024);
        let cx = px + cell_size / 2;
        let cy = py + cell_size / 2;

        match object {
            MapObject::Wall => {
                // Slate masonry barrier: seamless solid ground tint without outer stroke or box
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" fill=\"url(#grad-wall)\" />\n"
                ));
                // Masonry horizontal mortar seams
                let h1 = py + cell_size / 3;
                let h2 = py + (cell_size * 2) / 3;
                svg.push_str(&format!(
                    "    <line x1=\"{px}\" y1=\"{h1}\" x2=\"{}\" y2=\"{h1}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.35\" />\n",
                    px + cell_size
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{px}\" y1=\"{h2}\" x2=\"{}\" y2=\"{h2}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.35\" />\n",
                    px + cell_size
                ));
                // Masonry vertical staggered seams
                svg.push_str(&format!(
                    "    <line x1=\"{cx}\" y1=\"{py}\" x2=\"{cx}\" y2=\"{h1}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.35\" />\n"
                ));
                let v2 = px + cell_size / 4;
                let v3 = px + (cell_size * 3) / 4;
                svg.push_str(&format!(
                    "    <line x1=\"{v2}\" y1=\"{h1}\" x2=\"{v2}\" y2=\"{h2}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.35\" />\n"
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{v3}\" y1=\"{h2}\" x2=\"{v3}\" y2=\"{}\" stroke=\"#1e293b\" stroke-width=\"1.5\" opacity=\"0.35\" />\n",
                    py + cell_size
                ));
                // Central reinforced diamond armor crest
                svg.push_str(&format!(
                    "    <polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#f8fafc\" opacity=\"0.85\" />\n",
                    cx, cy - 7, cx + 6, cy, cx, cy + 7, cx - 6, cy
                ));
            }
            MapObject::Crate => {
                // Warm caramel wood tint: seamless ground block without outer stroke or box
                svg.push_str(&format!(
                    "    <rect x=\"{px}\" y=\"{py}\" width=\"{cell_size}\" height=\"{cell_size}\" fill=\"url(#grad-crate)\" />\n"
                ));
                // Timber cross bracing
                let pad = cell_size / 6;
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#c2410c\" stroke-width=\"2.5\" opacity=\"0.6\" stroke-linecap=\"round\" />\n",
                    px + pad, py + pad, px + cell_size - pad, py + cell_size - pad
                ));
                svg.push_str(&format!(
                    "    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#c2410c\" stroke-width=\"2.5\" opacity=\"0.6\" stroke-linecap=\"round\" />\n",
                    px + cell_size - pad, py + pad, px + pad, py + cell_size - pad
                ));
                // Center fastener stud
                svg.push_str(&format!(
                    "    <rect x=\"{}\" y=\"{}\" width=\"14\" height=\"14\" rx=\"3\" fill=\"#fff7ed\" stroke=\"#ea580c\" stroke-width=\"1.5\" />\n",
                    cx - 7, cy - 7
                ));
            }
            MapObject::Mine => {
                // Concealed buried minefield hazard indicator
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
            MapObject::Shield => {
                // Fresh mint shield / medical supply
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
        }

        svg
    }
}
