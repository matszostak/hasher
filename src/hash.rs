use std::{fs::File, io::{BufReader, Read}, path::PathBuf};
use md5::Context as Md5Context;
use sha1::Sha1;
use sha2::{Digest, Sha256};

const BUFFER_SIZE: usize = 1024 * 1024;

pub fn hash_file_md5(path: &PathBuf) -> std::io::Result<(String, u64)> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut ctx = Md5Context::new();
    let mut total = 0;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        ctx.consume(&buffer[..n]);
    }

    Ok((format!("{:x}", ctx.finalize()), total))
}


pub fn hash_file_sha1(path: &PathBuf) -> std::io::Result<(String, u64)> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut hasher = Sha1::new();
    let mut total = 0;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        hasher.update(&buffer[..n]);
    }

    Ok((format!("{:x}", hasher.finalize()), total))
}


pub fn hash_file_sha256(path: &PathBuf) -> std::io::Result<(String, u64)> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut hasher = Sha256::new();
    let mut total = 0;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok((format!("{:x}", result), total))
}