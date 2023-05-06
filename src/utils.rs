pub use rand::distributions::Alphanumeric;
use rand::{distributions::Distribution, Rng};
use serde_json::{self, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub struct PKCECodeVerifier;

impl Distribution<u8> for PKCECodeVerifier {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        const RANGE: u32 = 26 + 26 + 10 + 4;
        /// From https://datatracker.ietf.org/doc/html/rfc7636#section-4.1
        const GEN_ASCII_STR_CHARSET: &[u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
        debug_assert!(RANGE as usize == GEN_ASCII_STR_CHARSET.len());
        // We can pick from 66 characters. This is so close to a power of 2, 128,
        // that we can do better than Uniform. Use a simple bitshift and
        // rejection sampling. We do not use a bitmask, because for small RNGs
        // the most significant bits are usually of higher quality.
        // GEN_ASCII_STR_CHARSET[rng.next_u32() as usize]
        loop {
            let var = rng.next_u32() >> (32 - 7); // 2**6 < 66 < 2**7
            if var < RANGE {
                return GEN_ASCII_STR_CHARSET[var as usize];
            }
        }
    }
}

pub async fn save_json_response<P: AsRef<Path> + Send + Sync>(
    output_file: P,
    response: &Value,
) -> Result<(), std::io::Error> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output_file)?;
    serde_json::to_writer(&file, response)?;
    Ok(())
}

pub fn sanitize_filename(name: &String) -> String {
    sanitize_filename::sanitize_with_options(
        name,
        sanitize_filename::Options {
            truncate: true,
            windows: true,
            replacement: "",
        },
    )
}

pub fn random_string(length: usize, dist: impl Distribution<u8>) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}
