use sha2::digest::Output;
use sha2::{Sha256, Digest};
use regex::Regex;

pub fn u8_to_hex(u8: &[u8]) -> String {
    let hex = u8.iter().map(|byte| format!("{:02x}", byte)).collect::<String>();
    hex
}

pub fn u8_to_base64(u8: &[u8]) -> String {
    let encoded = base64::encode(u8);
    encoded
}

pub fn sha256_hex(input: impl AsRef<[u8]>) -> String {
    let hash = Sha256::digest(input);
    let hex_hash = u8_to_hex(&hash);
    hex_hash
}

///不可逆操作
fn base64_to_base62(input: String) -> String {
    let re = Regex::new(r"[^0-9A-Za-z]").unwrap();
    let result = re.replace_all(&input, "");
    result.into()
}

pub fn sha256_base62(input: impl AsRef<[u8]>) -> String {
    let hash = Sha256::digest(input);
    let base64_hash = u8_to_base64(&hash);
    let base62_hash = base64_to_base62(base64_hash);
    base62_hash
}