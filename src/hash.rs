use crate::helpers::Algorithm;
use md5::Context as Md5Context;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

const BUFFER_SIZE: usize = 1024 * 1024;

trait Hasher {
    fn update(&mut self, data: &[u8]);
    fn finalize(self: Box<Self>) -> String;
}

struct Md5Hasher(Md5Context);
impl Hasher for Md5Hasher {
    fn update(&mut self, data: &[u8]) {
        self.0.consume(data);
    }
    fn finalize(self: Box<Self>) -> String {
        format!("{:x}", self.0.finalize())
    }
}

struct Sha1Hasher(Sha1);
impl Hasher for Sha1Hasher {
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }
    fn finalize(self: Box<Self>) -> String {
        format!("{:x}", Digest::finalize(self.0))
    }
}

struct Sha256Hasher(Sha256);
impl Hasher for Sha256Hasher {
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }
    fn finalize(self: Box<Self>) -> String {
        format!("{:x}", Digest::finalize(self.0))
    }
}

fn make_hasher(algo: Algorithm) -> Box<dyn Hasher> {
    match algo {
        Algorithm::Md5 => Box::new(Md5Hasher(Md5Context::new())),
        Algorithm::Sha1 => Box::new(Sha1Hasher(Sha1::new())),
        Algorithm::Sha256 => Box::new(Sha256Hasher(Sha256::new())),
    }
}

pub fn hash_file(path: &PathBuf, algorithms: &[Algorithm]) -> std::io::Result<(Vec<String>, u64)> {
    let mut hashers: Vec<Box<dyn Hasher>> = algorithms.iter().map(|a| make_hasher(*a)).collect();

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total = 0u64;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        for h in &mut hashers {
            h.update(&buffer[..n]);
        }
    }

    let result_hashes = hashers.into_iter().map(Hasher::finalize).collect();
    Ok((result_hashes, total))
}
