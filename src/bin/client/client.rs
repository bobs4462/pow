use std::{io, sync::mpsc};

use pow::{
    codec::Codec,
    hash,
    message::{self, DeserializedMessage, Message},
};
use structopt::StructOpt;
use tokio::net::TcpStream;

#[derive(StructOpt)]
#[structopt(about = "PoW client startup parameters", verbatim_doc_comment)]
pub struct Params {
    /// address and port where server is listening
    #[structopt(short, long)]
    pub server_addr: String,
    /// number of threads to use to compute hash digest for challenge
    #[structopt(short, long)]
    pub threads: u8,
    /// maximum number of request bytes to keep in temporary buffer
    #[structopt(short, long)]
    pub memlimit: usize,
    /// quote number to request
    #[structopt(short, long)]
    pub quote: Option<usize>,
}

pub struct Client {
    codec: Codec<TcpStream>,
    params: Params,
}

impl Client {
    pub async fn new(params: Params) -> io::Result<Self> {
        let stream = TcpStream::connect(&params.server_addr).await?;
        let codec = Codec::new(stream, params.memlimit);
        Ok(Self { codec, params })
    }

    pub async fn run(self) -> io::Result<()> {
        let mut codec = self.codec;
        use DeserializedMessage::*;
        let request = message::Request {
            number: self.params.quote,
        };
        let message = Message::new(request);
        codec.write(message).await?;

        loop {
            match codec.read().await {
                Ok(Challenge(challenge)) => {
                    let session = challenge.session;
                    println!("received challenge for session {session}");
                    let (tx, rx) = mpsc::channel();
                    let data = challenge.string;
                    let cores = self.params.threads;
                    for c in 0..cores {
                        let tx = tx.clone();
                        let len = data.len();
                        let target = challenge.target;
                        let mut original = Vec::with_capacity(data.len() + 128);
                        original.append(&mut data.clone());
                        std::thread::spawn(move || {
                            for i in (c as u128..u128::MAX).step_by(cores as usize) {
                                original.extend(i.to_le_bytes());
                                let nonce = hash::digest(&mut original);
                                if nonce <= target {
                                    let _ = tx.send(i);
                                    break;
                                }
                                original.truncate(len);
                            }
                        });
                    }
                    if let Ok(nonce) = rx.recv() {
                        let solution = message::Solution { session, nonce };
                        let message = Message::new(solution);
                        codec.write(message).await?;
                        drop(rx);
                    } else {
                        panic!("Channel has hang up unexpectedly");
                    }
                }
                Ok(Response(response)) => {
                    let session = response.session;
                    match response.result {
                        Ok(quote) => {
                            println!("\n{quote}");
                            break;
                        }
                        Err(description) => {
                            println!("received error response for session {session}");
                            println!("err: {description}");
                        }
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    eprintln!("received garbage from server: {error}");
                    break;
                }
            }
        }
        Ok(())
    }
}
