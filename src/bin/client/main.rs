mod client;

use std::io;

use client::{Client, Params};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> io::Result<()> {
    let params: Params = StructOpt::from_args();
    let client = Client::new(params).await?;
    client.run().await?;
    Ok(())
}
