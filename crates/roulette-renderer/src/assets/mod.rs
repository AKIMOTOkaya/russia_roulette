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
    <!-- Glow and shadow filters for minimalist light theme -->
    <filter id="fx-glow-cyan" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="2.5" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
    <filter id="fx-glow-amber" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="2.5" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
    <filter id="fx-glow-laser" x="-30%" y="-30%" width="160%" height="160%">
      <feGaussianBlur stdDeviation="3" result="blur" />
      <feMerge>
        <feMergeNode in="blur" />
        <feMergeNode in="SourceGraphic" />
      </feMerge>
    </filter>
    <filter id="fx-drop-shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#0f172a" flood-opacity="0.10" />
    </filter>
    <filter id="fx-token-shadow" x="-30%" y="-30%" width="160%" height="160%">
      <feDropShadow dx="0" dy="3" stdDeviation="4" flood-color="#0f172a" flood-opacity="0.16" />
    </filter>

    <!-- Light Minimalist Gradients -->
    <linearGradient id="grad-wall" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#64748b" />
      <stop offset="100%" stop-color="#475569" />
    </linearGradient>
    <linearGradient id="grad-crate" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#fed7aa" />
      <stop offset="100%" stop-color="#fdba74" />
    </linearGradient>
    <linearGradient id="grad-water" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#e0f2fe" />
      <stop offset="100%" stop-color="#bae6fd" />
    </linearGradient>
    <linearGradient id="grad-ice" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#f0f9ff" />
      <stop offset="100%" stop-color="#e0f2fe" />
    </linearGradient>
    <linearGradient id="grad-mine" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#fee2e2" />
      <stop offset="100%" stop-color="#fecaca" />
    </linearGradient>
    <linearGradient id="grad-medkit" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#d1fae5" />
      <stop offset="100%" stop-color="#a7f3d0" />
    </linearGradient>
    <linearGradient id="grad-highground" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#fef9c3" />
      <stop offset="100%" stop-color="#fef08a" />
    </linearGradient>
    <linearGradient id="grad-laser" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#e11d48" />
      <stop offset="50%" stop-color="#ffe4e6" />
      <stop offset="100%" stop-color="#e11d48" />
    </linearGradient>
  </defs>
"##
}
