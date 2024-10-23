use sha2::{Digest, Sha256};
use std::any::type_name;

#[expect(
    clippy::unwrap_used,
    reason = "Won't ever panic, unless something is really wrong"
)]
pub fn get_type_name<T>() -> String {
    type_name::<T>()
        .split("::")
        .collect::<Vec<&str>>()
        .last()
        .unwrap()
        .to_lowercase()
}

pub fn hash(buffer: &str) -> String {
    format!("{:x}", Sha256::digest(buffer))
}
