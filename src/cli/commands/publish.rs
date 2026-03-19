use dialoguer::Confirm;

use crate::client::PpClient;
use crate::config::{self, EnvConfig};

pub async fn run(
    env_config: &EnvConfig,
    slug_or_url: &str,
    force: bool,
    to: Option<&str>,
) -> anyhow::Result<()> {
    let slug = extract_slug(slug_or_url);

    // Step 1: Get API key from 1Password
    let api_key = config::read_api_key(env_config)?;
    let client = PpClient::new(env_config, api_key);

    // Step 2: Fetch post from website
    println!("Fetching post '{}'...", slug);
    let post = client.fetch_post(&slug).await?;
    let subject = post.title.clone();
    let preview_text = post.preview_text.clone();

    // Step 3: Validate with printing-press
    let validation = client.validate(&slug, &post.newsletter).await?;

    println!();
    println!("  Post:       \"{}\"", post.title);
    println!("  Newsletter: {}", capitalize(&post.newsletter));

    // Step 4: Handle --to (test send)
    if let Some(email) = to {
        println!("  Test send:  {}", email);
        println!();

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
            .send_one(email, &slug, &post.newsletter, &subject, &post.email_html, preview_text.as_deref())
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

    println!(
        "  Will send to: {} subscribers",
        validation.eligible_subscribers
    );
    println!("  Already sent: {}", validation.already_sent);
    println!();

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
        .send(&slug, &post.newsletter, &subject, &post.email_html, force, preview_text.as_deref())
        .await?;

    println!("Enqueued {} emails for delivery.", result.enqueued);
    if result.already_sent > 0 {
        println!("({} already sent previously)", result.already_sent);
    }

    Ok(())
}

/// Extract slug from a URL or return as-is if already a slug.
/// Supports full URLs like `https://www.philipithomas.com/my-post`
/// or `http://localhost:3000/my-post`, stripping the trailing slash if present.
fn extract_slug(input: &str) -> String {
    if input.starts_with("http://") || input.starts_with("https://") {
        input
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(input)
            .to_string()
    } else {
        input.to_string()
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_slug_from_production_url() {
        assert_eq!(
            extract_slug("https://www.philipithomas.com/2026-03"),
            "2026-03"
        );
    }

    #[test]
    fn extract_slug_from_production_url_with_trailing_slash() {
        assert_eq!(
            extract_slug("https://www.philipithomas.com/my-post/"),
            "my-post"
        );
    }

    #[test]
    fn extract_slug_from_localhost_url() {
        assert_eq!(
            extract_slug("http://localhost:3000/fresh-coat-of-paint"),
            "fresh-coat-of-paint"
        );
    }

    #[test]
    fn extract_slug_passthrough_plain_slug() {
        assert_eq!(extract_slug("my-post"), "my-post");
    }

    #[test]
    fn extract_slug_passthrough_slug_with_no_scheme() {
        assert_eq!(extract_slug("some-slug"), "some-slug");
    }
}
