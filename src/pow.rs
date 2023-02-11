use std::time::{Duration, Instant};

use rand::RngCore;

use crate::hash;

pub struct PoW {
    challenge: Vec<u8>,
    target: u128,
    ts: Instant,
}

pub struct PoWConfig {
    pub length: usize,
    pub zeroes: u8,
}

impl PoW {
    pub fn new(conf: &PoWConfig) -> Self {
        let mut challenge = vec![0; conf.length];
        rand::thread_rng().fill_bytes(&mut challenge);
        let target = u128::MAX >> conf.zeroes;
        let ts = Instant::now();
        Self {
            challenge,
            target,
            ts,
        }
    }

    pub fn challenge(&self) -> Vec<u8> {
        self.challenge.clone()
    }

    pub fn target(&self) -> u128 {
        self.target
    }

    pub fn elapsed(&self) -> Duration {
        self.ts.elapsed()
    }

    pub fn verify(&mut self, nonce: u128) -> bool {
        let len = self.challenge.len();
        self.challenge.extend(nonce.to_le_bytes());
        let digest = hash::digest(&mut self.challenge);
        self.challenge.truncate(len);
        digest <= self.target
    }
}
