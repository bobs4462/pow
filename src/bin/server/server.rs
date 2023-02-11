use std::{collections::HashMap, io, net::SocketAddr};

use pow::{
    codec::Codec,
    message::{self, DeserializedMessage, Message},
    pow::{PoW, PoWConfig},
    wow::WORD_OF_WISDOM,
};
use structopt::StructOpt;
use tokio::net::{TcpListener, TcpStream};

#[derive(StructOpt)]
#[structopt(about = "PoW server startup parameters", verbatim_doc_comment)]
pub struct Params {
    /// incoming request listening address
    #[structopt(short, long)]
    pub listen: String,
    /// number of leading zero bits in target nonce
    #[structopt(short, long)]
    pub zeroes: u8,
    /// length, in bytes, of challenge string
    #[structopt(long)]
    pub length: usize,
    /// maximum number of request bytes to keep in temporary buffer
    #[structopt(short, long)]
    pub memlimit: usize,
}

pub struct Server {
    listener: TcpListener,
    params: Params,
}

struct Work {
    codec: Codec<TcpStream>,
    addr: SocketAddr,
    conf: PoWConfig,
}

impl Server {
    pub async fn new(params: Params) -> io::Result<Self> {
        let listener = TcpListener::bind(&params.listen).await?;
        Ok(Self { listener, params })
    }
    pub async fn run(self) -> io::Result<()> {
        while let Ok((stream, addr)) = self.listener.accept().await {
            let work = Work {
                codec: Codec::new(stream, self.params.memlimit),
                addr,
                conf: PoWConfig {
                    zeroes: self.params.zeroes,
                    length: self.params.length,
                },
            };
            tokio::spawn(serve(work));
        }
        Ok(())
    }
}

async fn serve(work: Work) -> io::Result<()> {
    let mut sessions = HashMap::new();
    let mut next = 0;
    let mut codec = work.codec;
    let addr = work.addr;
    use DeserializedMessage::*;
    loop {
        // possibly need to add timeout on read, in order to prevent
        // slow loris attack
        match codec.read().await {
            Ok(Request(request)) => {
                println!("{addr} is requesting quote number {:?}", request.number);
                let session = next;
                let pow = PoW::new(&work.conf);
                let string = pow.challenge();

                let challenge = message::Challenge {
                    session,
                    string,
                    target: pow.target(),
                };
                let message = Message::new(challenge);
                codec.write(message).await?;

                sessions.insert(session, (pow, request.number));
                next += 1;
            }
            Ok(Solution(solution)) => {
                let session = solution.session;
                let pow = match sessions.get_mut(&session) {
                    Some(pow) => pow,
                    None => {
                        let response = message::Response {
                            session,
                            result: Err("session not found".into()),
                        };
                        let message = Message::new(response);
                        codec.write(message).await?;
                        return Ok(()); // possible to continue
                    }
                };
                println!(
                    "{addr} has provided solution for session {session}, elapsed: {} s",
                    pow.0.elapsed().as_secs_f64(),
                );
                let response = if pow.0.verify(solution.nonce) {
                    let number = pow.1.unwrap_or_else(|| rand::random());
                    let quote =
                        (*WORD_OF_WISDOM.get(number % WORD_OF_WISDOM.len()).unwrap()).into();
                    message::Response {
                        session: solution.session,
                        result: Ok(quote),
                    }
                } else {
                    message::Response {
                        session: solution.session,
                        result: Err(format!(
                            "provided solution for session {session} is invalid"
                        )),
                    }
                    // may be should abort
                };
                let message = Message::new(response);
                codec.write(message).await?;
            }
            Ok(_) | Err(_) => {
                break;
            }
        }
    }
    Ok(())
}
