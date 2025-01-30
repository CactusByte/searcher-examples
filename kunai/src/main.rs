pub mod jito;
pub mod pump;
pub mod raydium;
pub mod listener;
pub mod provider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    listener::listen_pump_new_coins_discord().await?;
    println!("Hello, world!");
    Ok(())
}
