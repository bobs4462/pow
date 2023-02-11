use std::{io, mem};

use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::message::{DeserializedMessage, Header, Message, Serializeable};

pub struct Codec<S> {
    /// internally wrapped reader and writer of bytes
    stream: S,
    /// buffer for temporary data
    buffer: Vec<u8>,
    /// maximum number of bytes to keep in temporary buffer
    limit: usize,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Codec<S> {
    pub fn new(stream: S, limit: usize) -> Self {
        let buffer = Default::default();
        Self {
            stream,
            buffer,
            limit,
        }
    }

    pub async fn write<T: Serialize + Serializeable>(
        &mut self,
        msg: Message<T>,
    ) -> io::Result<usize> {
        self.buffer.clear();
        let slice: &[u8; mem::size_of::<Header>()] = unsafe { mem::transmute(msg.header()) };
        bincode::serialize_into(&mut self.buffer, msg.payload()).unwrap();
        let mut written = self.stream.write(slice).await?;
        let mut writ = 0;
        written += loop {
            writ += self.stream.write(&self.buffer[writ..]).await?;
            if writ >= self.buffer.len() {
                assert_eq!(msg.header().length, writ as u32);
                break writ;
            }
        };
        self.flush().await?;
        Ok(written)
    }

    pub async fn flush(&mut self) -> io::Result<()> {
        self.stream.flush().await
    }

    pub async fn read(&mut self) -> io::Result<DeserializedMessage> {
        self.buffer.clear();
        let mut header = [0; mem::size_of::<Header>()];
        self.stream.read_exact(&mut header).await?;
        let header: Header = unsafe { mem::transmute(header) };
        let mut buf = [0; 256];
        let mut to_read = header.length as usize;
        while to_read > 0 {
            let read = self.stream.read(&mut buf).await?;
            if self.buffer.len() + read > self.limit {
                return Err(io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    "reader memory limit exceeded",
                ));
            }
            self.buffer.extend_from_slice(&buf[..read]);
            to_read -= read;
        }
        let message = header
            .kind
            .parse_from_bytes(&self.buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(message)
    }
}
