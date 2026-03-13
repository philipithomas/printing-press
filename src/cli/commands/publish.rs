use dialoguer::Confirm;

use crate::client::PpClient;
use crate::config::{self, EnvConfig};

pub async fn run(
    env_config: &EnvConfig,
    slug: &str,
    force: bool,
    to: Option<&str>,
) -> anyhow::Result<()> {
    // Step 1: Get API key from 1Password
    let api_key = config::read_api_key(env_config)?;
    let client = PpClient::new(env_config, api_key);

    // Step 2: Fetch post from website
    println!("Fetching post '{}'...", slug);
    let post = client.fetch_post(slug).await?;
    let subject = post.title.clone();

    // Step 3: Validate with printing-press
    let validation = client.validate(slug, &post.newsletter).await?;

    println!();
    println!("  Post:       \"{}\"", post.title);
    println!("  Newsletter: {}", capitalize(&post.newsletter));
    println!(
        "  Will send to: {} subscribers",
        validation.eligible_subscribers
    );
    println!("  Already sent: {}", validation.already_sent);
    println!();

    // Step 4: Handle --to (test send)
    if let Some(email) = to {
        let confirm = Confirm::new()
            .with_prompt(format!(
                "Send test of \"{}\" to {} in {}?",
                post.title, email, env_config.name
            ))
            .default(false)
            .interact()?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }

        let result = client
            .send_one(email, slug, &post.newsletter, &subject, &post.email_html)
            .await?;

        if result.status == "sent" {
            println!("Test email sent to {}", email);
        } else {
            println!(
                "Send failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            );
        }
        return Ok(());
    }

    // Step 5: Safety check for already-sent
    if validation.already_sent > 0 && !force {
        anyhow::bail!(
            "{} subscribers have already received this post.\nUse --force to send to the remaining {} subscribers.",
            validation.already_sent,
            validation.eligible_subscribers
        );
    }

    if validation.eligible_subscribers == 0 {
        println!("No eligible subscribers to send to.");
        return Ok(());
    }

    // Step 6: Confirmation prompt
    let confirm = Confirm::new()
        .with_prompt(format!(
            "Send \"{}\" to {} subscribers in {}?",
            post.title, validation.eligible_subscribers, env_config.name
        ))
        .default(false)
        .interact()?;

    if !confirm {
        println!("Cancelled.");
        return Ok(());
    }

    // Step 7: Enqueue sends
    let result = client
        .send(slug, &post.newsletter, &subject, &post.email_html, force)
        .await?;

    println!("Enqueued {} emails for delivery.", result.enqueued);
    if result.already_sent > 0 {
        println!("({} already sent previously)", result.already_sent);
    }

    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
