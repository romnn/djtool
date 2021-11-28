use rand::{distributions::Alphanumeric, Rng};
use sanitize_filename as sanitizer;

pub fn sanitize_filename(name: String) -> String {
    sanitizer::sanitize_with_options(
        name,
        sanitizer::Options {
            truncate: true,
            windows: true,
            replacement: "",
        },
    )
    .replace(" ", "_")
}

pub fn random_filename(n: usize) -> String {
    let name = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect();
    sanitize_filename(name)
}
