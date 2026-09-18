//! Russian Roulette unified backend server binary entrypoint.

#![forbid(unsafe_code)]

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    roulette_backend::run_from_env().await
}
