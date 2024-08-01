use config::{Config, File};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;
use tokio::fs::read;

#[derive(Deserialize)]
struct Settings {
    api_url: String,
    api_token: String,
    backup_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting SQL Uploader version 1.0");
    println!("________________________________");

    let config = Config::builder()
        .add_source(File::from(Path::new("settings.json")))
        .build()
        .unwrap();
    let settings: Settings = config.try_deserialize().unwrap();

    let paths = fs::read_dir(&settings.backup_path)?;

    let mut files: Vec<_> = paths
        .filter_map(Result::ok)
        .filter(|dir_entry| {
            let path = dir_entry.path();
            path.is_file() && path.extension().map_or(false, |ext| ext == "zip")
        })
        .collect();

    files.sort_by_key(|dir_entry| {
        fs::metadata(&dir_entry.path())
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });

    let latest_file = files.last().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "No .zip files in directory",
    ))?;

    let content = read(latest_file.path()).await?;
    let part =
        Part::bytes(content).file_name(latest_file.file_name().to_string_lossy().into_owned());

    let form = Form::new().part("sql", part);

    let response = Client::new()
        .post(&settings.api_url)
        .header("Authorization", &settings.api_token)
        .multipart(form)
        .send()
        .await?;

    println!("Status: {}", response.status());
    println!("Body:\n{}", response.text().await?);

    Ok(())
}
