//! Visual theme, color palettes, and geometric metrics for the tactical HUD image renderer.

#![forbid(unsafe_code)]

use roulette_domain::{PlayerId, Terrain, Weather};

/// Geometric dimensions and layout metrics for rendering.
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
            cell_gap: 8,
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

/// Visual theme defining the dark cyber-noir tactical aesthetic.
pub struct Theme;

impl Theme {
    /// Canvas dark background color.
    pub const CANVAS_BG: Color = Color::rgb(15, 19, 26);
    /// Card frame background.
    pub const CARD_BG: Color = Color::rgb(22, 27, 38);
    /// Subtle frame border.
    pub const CARD_BORDER: Color = Color::rgb(39, 49, 66);
    /// Inner card section background.
    pub const SECTION_BG: Color = Color::rgb(27, 34, 48);

    /// Primary bright white text.
    pub const TEXT_PRIMARY: Color = Color::rgb(241, 245, 249);
    /// Secondary muted silver text.
    pub const TEXT_SECONDARY: Color = Color::rgb(148, 163, 184);
    /// Dimmed tertiary grey text.
    pub const TEXT_MUTED: Color = Color::rgb(100, 116, 139);

    /// High-visibility cyber cyan accent.
    pub const ACCENT_CYAN: Color = Color::rgb(0, 240, 255);
    /// Warm alert amber accent.
    pub const ACCENT_AMBER: Color = Color::rgb(245, 158, 11);
    /// Critical hazard red accent.
    pub const ACCENT_RED: Color = Color::rgb(239, 68, 68);
    /// Health and success green accent.
    pub const ACCENT_GREEN: Color = Color::rgb(16, 185, 129);
    /// Tactical purple accent.
    pub const ACCENT_PURPLE: Color = Color::rgb(168, 85, 247);

    /// Returns the color palette for a terrain cell.
    #[must_use]
    pub const fn terrain_palette(terrain: Terrain) -> TerrainPalette {
        match terrain {
            Terrain::Empty => TerrainPalette {
                fill: Color::rgb(26, 33, 47),
                stroke: Color::rgb(42, 53, 73),
                emblem: Color::rgba(148, 163, 184, 300),
            },
            Terrain::Wall => TerrainPalette {
                fill: Color::rgb(51, 65, 85),
                stroke: Color::rgb(100, 116, 139),
                emblem: Color::rgb(203, 213, 225),
            },
            Terrain::Crate => TerrainPalette {
                fill: Color::rgb(69, 43, 20),
                stroke: Color::rgb(133, 77, 14),
                emblem: Color::rgb(245, 158, 11),
            },
            Terrain::Water => TerrainPalette {
                fill: Color::rgb(12, 74, 110),
                stroke: Color::rgb(2, 132, 199),
                emblem: Color::rgb(56, 189, 248),
            },
            Terrain::Ice => TerrainPalette {
                fill: Color::rgb(22, 78, 99),
                stroke: Color::rgb(56, 189, 248),
                emblem: Color::rgb(186, 230, 253),
            },
            Terrain::Mine => TerrainPalette {
                fill: Color::rgb(69, 10, 10),
                stroke: Color::rgb(220, 38, 38),
                emblem: Color::rgb(248, 113, 113),
            },
            Terrain::Medkit => TerrainPalette {
                fill: Color::rgb(6, 78, 59),
                stroke: Color::rgb(16, 185, 129),
                emblem: Color::rgb(110, 231, 183),
            },
            Terrain::HighGround => TerrainPalette {
                fill: Color::rgb(63, 49, 20),
                stroke: Color::rgb(202, 138, 4),
                emblem: Color::rgb(250, 204, 21),
            },
        }
    }

    /// Color assignment for players based on seat index (`PlayerId` 1..6).
    #[must_use]
    pub const fn player_color(player_id: PlayerId) -> Color {
        match player_id.0 {
            1 => Color::rgb(0, 229, 255),  // Neon Cyan
            2 => Color::rgb(249, 115, 22), // Flame Orange
            3 => Color::rgb(34, 197, 94),  // Toxic Emerald
            4 => Color::rgb(168, 85, 247), // Cyber Purple
            5 => Color::rgb(234, 179, 8),  // Amber Gold
            _ => Color::rgb(236, 72, 153), // Hot Coral Pink
        }
    }

    /// Returns weather badge styling (icon, display label, and theme color).
    #[must_use]
    pub const fn weather_style(weather: Weather) -> (&'static str, &'static str, Color) {
        match weather {
            Weather::Clear => ("☀️", "晴朗 (视野开阔)", Color::rgb(250, 204, 21)),
            Weather::Blizzard => ("❄️", "暴风雪 (水凝成冰/极滑)", Color::rgb(56, 189, 248)),
            Weather::Heatwave => ("🔥", "酷暑热浪 (薄冰消融)", Color::rgb(249, 115, 22)),
            Weather::DenseFog => ("🌫️", "浓雾弥漫 (视距受限)", Color::rgb(148, 163, 184)),
        }
    }
}
