use dialoguer::Confirm;

use crate::client::PpClient;
use crate::config::{self, EnvConfig};

pub async fn run(
    env_config: &EnvConfig,
    slug: &str,
    force: bool,
    to: Option<&str>,
) -> anyhow::Result<()> {
    // Step 1: Get API key
    let api_key = config::read_api_key(env_config)?;
    let client = PpClient::new(env_config, api_key);

    // Step 2: Fetch post from website
    println!("Fetching post '{}'...", slug);
    let post = client.fetch_post(slug).await?;
    let subject = post.title.clone();

    // Step 3: Validate with printing-press
    let validation = client.mail_validate(slug, &post.newsletter).await?;

    println!();
    println!("  Post:       \"{}\"", post.title);
    println!("  Newsletter: {}", capitalize(&post.newsletter));
    println!(
        "  Mail recipients: {} with valid shipping",
        validation.eligible_recipients
    );
    println!("  Already sent: {}", validation.already_sent);
    println!();

    // Step 4: Handle --to (test send to single address)
    if let Some(email) = to {
        let confirm = Confirm::new()
            .with_prompt(format!(
                "Send test letter for \"{}\" to {} in {}?",
                post.title, email, env_config.name
            ))
            .default(false)
            .interact()?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }

        let result = client
            .mail_send_one(
                email,
                slug,
                &post.newsletter,
                &subject,
                &post.email_html,
                post.subtitle.as_deref(),
                post.published_at.as_deref(),
                post.cover_image.as_deref(),
                post.cover_image_alt.as_deref(),
            )
            .await?;

        if result.status == "sent" {
            println!("Test letter sent for {}", email);
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
            "{} letters have already been sent for this post.\nUse --force to send to the remaining {} recipients.",
            validation.already_sent,
            validation.eligible_recipients
        );
    }

    if validation.eligible_recipients == 0 {
        println!("No eligible mail recipients.");
        return Ok(());
    }

    // Step 6: Confirmation prompt
    let confirm = Confirm::new()
        .with_prompt(format!(
            "Send \"{}\" as physical mail to {} recipients in {}?",
            post.title, validation.eligible_recipients, env_config.name
        ))
        .default(false)
        .interact()?;

    if !confirm {
        println!("Cancelled.");
        return Ok(());
    }

    // Step 7: Send letters
    let result = client
        .mail_send(
            slug,
            &post.newsletter,
            &subject,
            &post.email_html,
            post.subtitle.as_deref(),
            post.published_at.as_deref(),
            post.cover_image.as_deref(),
            post.cover_image_alt.as_deref(),
            force,
        )
        .await?;

    println!("Sent {} letters.", result.sent);
    if result.skipped > 0 {
        println!("({} skipped — already sent)", result.skipped);
    }
    if result.errors > 0 {
        println!("({} errors — check server logs)", result.errors);
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
