use std::io;

use server::{Params, Server};
use structopt::StructOpt;

mod server;

#[tokio::main]
async fn main() -> io::Result<()> {
    let params: Params = StructOpt::from_args();
    let server = Server::new(params).await?;
    server.run().await?;
    Ok(())
}
