//#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//#![allow(dead_code, unused_imports, unused_variables)]
mod releases;
pub use crate::releases::Releases;

#[tokio::main]
async fn main() {
    let mut releases = Releases::new();
    releases.fetch_official_releases().await;
    releases.fetch_lts_releases().await;
    releases.fetch_latest_stable().await;
    releases.fetch_latest_daily().await;
    releases.fetch_experimental_branches().await;
    println!("{:#?}", releases);
}
