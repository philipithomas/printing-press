use std::path::Path;

use printing_press::models::subscriber::{ImportResult, ImportSubscriberEntry};

use crate::client::PpClient;
use crate::config::{self, EnvConfig};

const BATCH_SIZE: usize = 1000;

pub async fn run(env_config: &EnvConfig, format: &str, csv_path: &str) -> anyhow::Result<()> {
    let path = Path::new(csv_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", csv_path);
    }

    let entries = match format {
        "ghost-members" => parse_ghost_members(path)?,
        "postcard" => parse_postcard(path)?,
        _ => anyhow::bail!(
            "Unknown format: {}. Use 'ghost-members' or 'postcard'.",
            format
        ),
    };

    if entries.is_empty() {
        println!("No eligible subscribers found in {}", csv_path);
        return Ok(());
    }

    println!(
        "Parsed {} eligible subscribers from {} (format: {})",
        entries.len(),
        csv_path,
        format
    );
    println!("Importing to {} ...", env_config.name);

    let api_key = config::read_api_key(env_config)?;
    let client = PpClient::new(env_config, api_key);

    let mut total_result = ImportResult {
        created: 0,
        updated: 0,
        total: 0,
    };

    for (i, chunk) in entries.chunks(BATCH_SIZE).enumerate() {
        let batch_num = i + 1;
        let batch_count = entries.len().div_ceil(BATCH_SIZE);
        if batch_count > 1 {
            println!(
                "  Sending batch {}/{} ({} subscribers)...",
                batch_num,
                batch_count,
                chunk.len()
            );
        }

        let result = client.import_subscribers(chunk.to_vec()).await?;
        total_result.created += result.created;
        total_result.updated += result.updated;
        total_result.total += result.total;
    }

    println!(
        "Done! Created: {}, Updated: {}, Total: {}",
        total_result.created, total_result.updated, total_result.total
    );

    Ok(())
}

fn parse_ghost_members(path: &Path) -> anyhow::Result<Vec<ImportSubscriberEntry>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();

    let email_idx = header_index(&headers, "email")?;
    let name_idx = header_index(&headers, "name")?;
    let subscribed_idx = header_index(&headers, "subscribed_to_emails")?;
    let labels_idx = header_index(&headers, "labels")?;
    let deleted_idx = header_index(&headers, "deleted_at")?;

    let mut entries = Vec::new();

    for result in reader.records() {
        let record = result?;

        let deleted_at = record.get(deleted_idx).unwrap_or("");
        if !deleted_at.is_empty() {
            continue;
        }

        let subscribed = record.get(subscribed_idx).unwrap_or("false");
        if subscribed == "false" {
            continue;
        }

        let email = record.get(email_idx).unwrap_or("").to_string();
        if email.is_empty() {
            continue;
        }

        let name = record.get(name_idx).unwrap_or("");
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };

        let labels_raw = record.get(labels_idx).unwrap_or("");
        let source = parse_ghost_source(labels_raw);

        entries.push(ImportSubscriberEntry {
            email,
            name,
            source,
            newsletters: vec!["contraption".to_string(), "workshop".to_string()],
        });
    }

    Ok(entries)
}

fn parse_ghost_source(labels: &str) -> Option<String> {
    if labels.is_empty() {
        return None;
    }
    for label in labels.split(',') {
        let label = label.trim();
        if !label.is_empty() && !label.starts_with("Import") {
            return Some(label.to_string());
        }
    }
    None
}

fn parse_postcard(path: &Path) -> anyhow::Result<Vec<ImportSubscriberEntry>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();

    let email_idx = header_index(&headers, "Email")?;
    let unsub_idx = header_index(&headers, "Unsubscribed at")?;
    let source_idx = header_index(&headers, "Source")?;

    let mut entries = Vec::new();

    for result in reader.records() {
        let record = result?;

        let unsub_at = record.get(unsub_idx).unwrap_or("");
        if !unsub_at.is_empty() {
            continue;
        }

        let email = record.get(email_idx).unwrap_or("").to_string();
        if email.is_empty() {
            continue;
        }

        let source_raw = record.get(source_idx).unwrap_or("");
        let source = if source_raw.is_empty() {
            None
        } else {
            Some(source_raw.to_string())
        };

        entries.push(ImportSubscriberEntry {
            email,
            name: None,
            source,
            newsletters: vec!["postcard".to_string()],
        });
    }

    Ok(entries)
}

fn header_index(headers: &csv::StringRecord, name: &str) -> anyhow::Result<usize> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| anyhow::anyhow!("Missing required column: {}", name))
}
