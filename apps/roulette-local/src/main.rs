//! Local LAN mode binary for Russian Roulette game server.

#![forbid(unsafe_code)]

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    roulette_backend::run_local().await
}
