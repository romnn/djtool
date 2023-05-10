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
