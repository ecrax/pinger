use dotenv::dotenv;

use distances::calculate_distances;
use geolocations::collect_geolocations;
use ips::collect_ips;
use ping::ping_ips;
use plotting::plot_data;

mod distances;
mod geolocations;
mod ips;
mod ping;
mod plotting;
mod structs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;
    // collect_ips()?;
    // collect_geolocations().await?;
    // ping_ips().await?;
    // calculate_distances().await?;

    plot_data()?;

    Ok(())
}
