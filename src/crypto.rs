use sha2::{Digest, Sha256};

use crate::error::Result;
use std::io::{BufRead, BufReader, Read};

pub fn hash_buf<R: Read>(mut reader: BufReader<R>) -> Result<String> {
    let mut hasher = Sha256::new();

    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            break;
        }
        hasher.update(buffer);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn hash_bytes(data: impl AsRef<[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
