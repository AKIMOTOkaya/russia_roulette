//! Pure procedural vector asset library for Russian Roulette.
//!
//! Provides high-fidelity SVG graphics, tactical glyphs, terrain shaders, and
//! combat effect layers without relying on platform emojis or external fonts.

#![forbid(unsafe_code)]

pub mod effects;
pub mod player;
pub mod terrain;
pub mod tilesheet;

pub use effects::EffectsRenderer;
pub use player::PlayerAssetRenderer;
pub use terrain::TerrainAssetRenderer;
pub use tilesheet::TileSheetRenderer;

/// Generates reusable SVG `<defs>` containing linear gradients, drop shadows, and glow filters.
#[must_use]
pub fn render_shared_defs() -> &'static str {
    r##"  <defs>
    <!-- Glow and shadow filters -->
    <filter id="fx-glow-cyan" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="3" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
    <filter id="fx-glow-amber" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="3" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
    <filter id="fx-glow-laser" x="-30%" y="-30%" width="160%" height="160%">
      <feGaussianBlur stdDeviation="4" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
    <filter id="fx-drop-shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="4" stdDeviation="4" flood-color="#000000" flood-opacity="0.6" />
    </filter>

    <!-- Gradients -->
    <linearGradient id="grad-wall" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#475569" />
      <stop offset="100%" stop-color="#1e293b" />
    </linearGradient>
    <linearGradient id="grad-crate" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#78350f" />
      <stop offset="100%" stop-color="#2d1506" />
    </linearGradient>
    <linearGradient id="grad-water" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#0369a1" />
      <stop offset="100%" stop-color="#082f49" />
    </linearGradient>
    <linearGradient id="grad-ice" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#38bdf8" stop-opacity="0.9" />
      <stop offset="100%" stop-color="#0c4a6e" stop-opacity="0.95" />
    </linearGradient>
    <linearGradient id="grad-mine" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#7f1d1d" />
      <stop offset="100%" stop-color="#250404" />
    </linearGradient>
    <linearGradient id="grad-medkit" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#059669" />
      <stop offset="100%" stop-color="#022c22" />
    </linearGradient>
    <linearGradient id="grad-highground" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#854d0e" />
      <stop offset="100%" stop-color="#2c1a04" />
    </linearGradient>
    <linearGradient id="grad-laser" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#ff2a6d" />
      <stop offset="50%" stop-color="#fffb96" />
      <stop offset="100%" stop-color="#ff2a6d" />
    </linearGradient>
  </defs>
"##
}
