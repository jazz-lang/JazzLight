use std::mem::transmute;

pub fn hash_int(x: impl Into<u64>) -> u64 {
    let bytes: [u8; 8] = unsafe { transmute(x.into()) };
    hash_bytes(&bytes)
}

pub fn hash_str(x: &str) -> u64 {
    hash_bytes(x.as_bytes())
}
/// FNV hashing algorithm
#[inline(always)]
pub fn hash_bytes(x: &[u8]) -> u64 {
    x.iter().fold(0xcbf29ce484222325, |acc, &byte| {
        (acc ^ (byte as i8 as u64)).wrapping_mul(0x100000001b3)
    })
}
