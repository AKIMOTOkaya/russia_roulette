//! Visual theme, color palettes, and geometric metrics for the tactical HUD image renderer.

#![forbid(unsafe_code)]

use roulette_domain::{CellView, PlayerId, Position, Terrain, Weather};

/// Dynamic geometric metrics for board and grid layouts.
/// Supports arbitrary non-square grid dimensions while keeping individual cells square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardMetrics {
    /// Number of grid columns (horizontal cells).
    pub cols: u8,
    /// Number of grid rows (vertical cells).
    pub rows: u8,
    /// Edge length of each square cell in pixels.
    pub cell_size: u32,
    /// Spacing between adjacent cells in pixels.
    pub cell_gap: u32,
    /// Outer border padding around the grid in pixels.
    pub padding: u32,
    /// Coordinate ruler thickness outside the grid.
    pub ruler_offset: u32,
}

impl BoardMetrics {
    /// Derives board metrics from a list of cell views, dynamically determining max columns and rows.
    #[must_use]
    pub fn from_cells(cells: &[CellView], default_size: u8) -> Self {
        let max_x = cells.iter().map(|c| c.position.x).max().unwrap_or(0);
        let max_y = cells.iter().map(|c| c.position.y).max().unwrap_or(0);
        let cols = (max_x + 1).max(default_size).max(1);
        let rows = (max_y + 1).max(default_size).max(1);

        Self {
            cols,
            rows,
            cell_size: 80,
            cell_gap: 0,
            padding: 24,
            ruler_offset: 0,
        }
    }

    /// Creates standalone large map metrics with custom cell size.
    #[must_use]
    pub fn standalone(cols: u8, rows: u8, cell_size: u32) -> Self {
        Self {
            cols,
            rows,
            cell_size,
            cell_gap: 0,
            padding: 24,
            ruler_offset: 0,
        }
    }

    /// Total pixel width of the board including grid, gaps, padding, and rulers.
    #[must_use]
    pub fn total_width(&self) -> u32 {
        let cols = u32::from(self.cols);
        let grid_w = cols * self.cell_size + cols.saturating_sub(1) * self.cell_gap;
        grid_w + self.padding * 2
    }

    /// Total pixel height of the board including grid, gaps, padding, and rulers.
    #[must_use]
    pub fn total_height(&self) -> u32 {
        let rows = u32::from(self.rows);
        let grid_h = rows * self.cell_size + rows.saturating_sub(1) * self.cell_gap;
        grid_h + self.padding * 2
    }

    /// Pixel X coordinate for the top-left corner of a cell at column `x`.
    #[must_use]
    pub fn cell_x(&self, x: u8) -> u32 {
        self.padding + u32::from(x) * (self.cell_size + self.cell_gap)
    }

    /// Pixel Y coordinate for the top-left corner of a cell at row `y`.
    #[must_use]
    pub fn cell_y(&self, y: u8) -> u32 {
        self.padding + u32::from(y) * (self.cell_size + self.cell_gap)
    }

    /// Pixel center coordinates `(cx, cy)` for a cell at `pos`.
    #[must_use]
    pub fn cell_center(&self, pos: Position) -> (u32, u32) {
        let px = self.cell_x(pos.x);
        let py = self.cell_y(pos.y);
        (px + self.cell_size / 2, py + self.cell_size / 2)
    }
}

/// Geometric dimensions and layout metrics for full HUD cards.
#[derive(Debug, Clone, Copy)]
pub struct LayoutMetrics {
    /// Overall canvas width in pixels.
    pub canvas_width: u32,
    /// Overall canvas height in pixels.
    pub canvas_height: u32,
    /// Grid cell edge length in pixels.
    pub cell_size: u32,
    /// Gap between adjacent grid cells.
    pub cell_gap: u32,
    /// Top-left offset of the board inside the canvas.
    pub board_origin_x: u32,
    /// Top-left offset of the board inside the canvas.
    pub board_origin_y: u32,
}

impl Default for LayoutMetrics {
    fn default() -> Self {
        Self {
            canvas_width: 1080,
            canvas_height: 720,
            cell_size: 76,
            cell_gap: 0,
            board_origin_x: 64,
            board_origin_y: 130,
        }
    }
}

/// Color representation for SVG and canvas fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
    /// Alpha opacity (0.0 - 1.0 represented as permille: 0 - 1000).
    pub a_permille: u16,
}

impl Color {
    /// Creates an opaque color from 8-bit RGB components.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            a_permille: 1000,
        }
    }

    /// Creates a color with alpha opacity permille (e.g. 800 for 0.8).
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a_permille: u16) -> Self {
        Self {
            r,
            g,
            b,
            a_permille,
        }
    }

    /// Formats this color as an SVG hex `#rrggbb` or `rgba(r, g, b, a)` string.
    #[must_use]
    pub fn to_svg_color(self) -> String {
        if self.a_permille >= 1000 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            let alpha = f64::from(self.a_permille) / 1000.0;
            format!("rgba({}, {}, {}, {:.3})", self.r, self.g, self.b, alpha)
        }
    }
}

/// Palette configuration for terrain cells.
#[derive(Debug, Clone, Copy)]
pub struct TerrainPalette {
    /// Background fill color.
    pub fill: Color,
    /// Border stroke color.
    pub stroke: Color,
    /// Emblem or icon accent color.
    pub emblem: Color,
}

/// Visual theme defining the minimalist light aesthetic.
pub struct Theme;

impl Theme {
    /// Pure white canvas background color.
    pub const CANVAS_BG: Color = Color::rgb(255, 255, 255);
    /// Continuous tactical board ground base.
    pub const BOARD_BG: Color = Color::rgb(248, 250, 252);
    /// Subtle board border line.
    pub const BOARD_BORDER: Color = Color::rgb(226, 232, 240);
    /// Grid intersection mark / subtle micro-dot color.
    pub const GRID_DOT: Color = Color::rgba(203, 213, 225, 600);

    /// Card frame background (legacy / waiting card).
    pub const CARD_BG: Color = Color::rgb(248, 250, 252);
    /// Subtle frame border.
    pub const CARD_BORDER: Color = Color::rgb(226, 232, 240);
    /// Inner card section background.
    pub const SECTION_BG: Color = Color::rgb(241, 245, 249);

    /// Primary dark slate text.
    pub const TEXT_PRIMARY: Color = Color::rgb(15, 23, 42);
    /// Secondary medium slate text.
    pub const TEXT_SECONDARY: Color = Color::rgb(71, 85, 105);
    /// Dimmed tertiary grey text (e.g. rulers).
    pub const TEXT_MUTED: Color = Color::rgb(148, 163, 184);

    /// Accent cyan/blue adapted for light backgrounds.
    pub const ACCENT_CYAN: Color = Color::rgb(2, 132, 199);
    /// Accent amber adapted for light backgrounds.
    pub const ACCENT_AMBER: Color = Color::rgb(217, 119, 6);
    /// Accent hazard red adapted for light backgrounds.
    pub const ACCENT_RED: Color = Color::rgb(220, 38, 38);
    /// Accent success green adapted for light backgrounds.
    pub const ACCENT_GREEN: Color = Color::rgb(22, 163, 74);
    /// Accent tactical purple adapted for light backgrounds.
    pub const ACCENT_PURPLE: Color = Color::rgb(147, 51, 234);

    /// High-energy laser tracer crimson/magenta.
    pub const ACCENT_LASER: Color = Color::rgb(225, 29, 72);
    /// Muzzle flash starflare yellow.
    pub const ACCENT_MUZZLE: Color = Color::rgb(250, 204, 21);
    /// Kinetic impact burst orange.
    pub const ACCENT_IMPACT: Color = Color::rgb(234, 88, 12);

    /// Returns the color palette for a terrain cell in light mode.
    #[must_use]
    pub const fn terrain_palette(terrain: Terrain) -> TerrainPalette {
        match terrain {
            Terrain::Empty => TerrainPalette {
                fill: Color::rgb(248, 250, 252),
                stroke: Color::rgba(226, 232, 240, 400),
                emblem: Color::rgba(148, 163, 184, 400),
            },
            Terrain::Wall => TerrainPalette {
                fill: Color::rgb(71, 85, 105),
                stroke: Color::rgb(51, 65, 85),
                emblem: Color::rgb(241, 245, 249),
            },
            Terrain::Crate => TerrainPalette {
                fill: Color::rgb(254, 215, 170),
                stroke: Color::rgb(234, 88, 12),
                emblem: Color::rgb(154, 52, 18),
            },
            Terrain::Water => TerrainPalette {
                fill: Color::rgb(186, 230, 253),
                stroke: Color::rgb(56, 189, 248),
                emblem: Color::rgb(2, 132, 199),
            },
            Terrain::Ice => TerrainPalette {
                fill: Color::rgb(224, 242, 254),
                stroke: Color::rgb(125, 211, 252),
                emblem: Color::rgb(14, 165, 233),
            },
            Terrain::Mine => TerrainPalette {
                fill: Color::rgb(254, 202, 202),
                stroke: Color::rgb(248, 113, 113),
                emblem: Color::rgb(220, 38, 38),
            },
            Terrain::Medkit => TerrainPalette {
                fill: Color::rgb(167, 243, 208),
                stroke: Color::rgb(52, 211, 153),
                emblem: Color::rgb(5, 150, 105),
            },
            Terrain::HighGround => TerrainPalette {
                fill: Color::rgb(254, 240, 138),
                stroke: Color::rgb(250, 204, 21),
                emblem: Color::rgb(161, 98, 7),
            },
        }
    }

    /// Color assignment for players based on seat index (`PlayerId` 1..6).
    /// Modern, vibrant, distinct tones with high contrast on white/light grounds.
    #[must_use]
    pub const fn player_color(player_id: PlayerId) -> Color {
        match player_id.0 {
            1 => Color::rgb(2, 132, 199),  // Ocean / Sky Blue
            2 => Color::rgb(234, 88, 12),  // Flame Orange
            3 => Color::rgb(22, 163, 74),  // Emerald Green
            4 => Color::rgb(147, 51, 234), // Royal Purple
            5 => Color::rgb(217, 119, 6),  // Amber Gold
            _ => Color::rgb(219, 39, 119), // Rose Pink
        }
    }

    /// Returns weather badge styling (icon, display label, and theme color).
    #[must_use]
    pub const fn weather_style(weather: Weather) -> (&'static str, &'static str, Color) {
        match weather {
            Weather::Clear => ("☀️", "晴朗 (视野开阔)", Color::rgb(217, 119, 6)),
            Weather::Blizzard => ("❄️", "暴风雪 (水凝成冰/极滑)", Color::rgb(2, 132, 199)),
            Weather::Heatwave => ("🔥", "酷暑热浪 (薄冰消融)", Color::rgb(234, 88, 12)),
            Weather::DenseFog => ("🌫️", "浓雾弥漫 (视距受限)", Color::rgb(100, 116, 139)),
        }
    }
}
