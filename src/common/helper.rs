use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// use std::hash::{DefaultHasher, Hash, Hasher};
use twox_hash::xxhash32;

pub fn u8_to_u64(input: &[u8]) -> &[u64] {
    let len = input.len() / 8;
    let ptr = input.as_ptr() as *const u64;
    let slice: &[u64] = unsafe { std::slice::from_raw_parts(ptr, len) };
    slice
}

// pub fn hash_to_u32(data: &str) -> u32 {
//     let mut hasher = DefaultHasher::new();
//     data.hash(&mut hasher);
//     let hash_value = hasher.finish();
//     // 截断到 u32
//     (hash_value & 0xFFFF_FFFF) as u32
// }

pub fn hash_to_u32(data: &String) -> u32 {
    xxhash32::Hasher::oneshot(10, data.as_bytes())
}

pub fn get_timestamp() -> u128 {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    timestamp
}


pub fn empty_loop() {
    let mut count = 0;
    loop {
        println!("Main Loop @ {}", count);
        count = count + 1;
        sleep(Duration::from_secs(10));
    }
}