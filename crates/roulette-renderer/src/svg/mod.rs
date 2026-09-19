//! SVG generation modules for Russian Roulette tactical HUD cards.

#![forbid(unsafe_code)]

pub mod banner;
pub mod board;
pub mod composer;
pub mod header;
pub mod roster;

pub use composer::SvgComposer;
