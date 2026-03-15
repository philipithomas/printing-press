use minijinja::{Environment, context};
use regex::Regex;

use crate::config::SITE_BASE_URL;

/// Rewrite relative URLs in HTML content to absolute URLs using SITE_BASE_URL.
///
/// Converts `href="/..."` and `src="/..."` (both quote styles) so that links
/// and images work correctly in email clients, which have no base URL context.
pub fn resolve_relative_urls(html: &str) -> String {
    // Match href or src attributes whose value starts with "/" but not "//".
    // Captures the char after "/" to exclude protocol-relative URLs.
    let re = Regex::new(r#"(href|src)(=["'])/([^/])"#).unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        format!("{}{}{}/{}", &caps[1], &caps[2], SITE_BASE_URL, &caps[3])
    })
    .into_owned()
}

pub fn render_confirmation(
    code: &str,
    magic_link: &str,
    site_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut env = Environment::new();
    env.add_template("confirmation", include_str!("confirmation.html"))?;
    let tmpl = env.get_template("confirmation")?;
    let result = tmpl.render(context! {
        code => code,
        magic_link => magic_link,
        site_url => site_url,
        site_title => "philipithomas.com",
        current_year => chrono::Utc::now().format("%Y").to_string(),
    })?;
    Ok(result)
}

pub fn render_newsletter(
    content: &str,
    unsubscribe_url: &str,
    site_url: &str,
    newsletter: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let content = resolve_relative_urls(content);
    let mut env = Environment::new();
    env.add_template("newsletter", include_str!("newsletter.html"))?;
    let tmpl = env.get_template("newsletter")?;
    let bg_color = match newsletter {
        Some("contraption") => "#f2f2f1",
        Some("workshop") => "#f3f0e9",
        Some("postcard") => "#f5f6fa",
        _ => "#f5f3f0",
    };
    let result = tmpl.render(context! {
        content => content,
        unsubscribe_url => unsubscribe_url,
        site_url => site_url,
        site_title => "philipithomas.com",
        newsletter => newsletter.unwrap_or(""),
        bg_color => bg_color,
        current_year => chrono::Utc::now().format("%Y").to_string(),
    })?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_href_with_double_quotes() {
        let html = r#"<a href="/blog/my-post">link</a>"#;
        let result = resolve_relative_urls(html);
        assert_eq!(
            result,
            r#"<a href="https://www.philipithomas.com/blog/my-post">link</a>"#
        );
    }

    #[test]
    fn rewrites_href_with_single_quotes() {
        let html = "<a href='/about'>about</a>";
        let result = resolve_relative_urls(html);
        assert_eq!(
            result,
            "<a href='https://www.philipithomas.com/about'>about</a>"
        );
    }

    #[test]
    fn rewrites_src_attributes() {
        let html = r#"<img src="/images/photo.jpg">"#;
        let result = resolve_relative_urls(html);
        assert_eq!(
            result,
            r#"<img src="https://www.philipithomas.com/images/photo.jpg">"#
        );
    }

    #[test]
    fn leaves_absolute_urls_unchanged() {
        let html = r#"<a href="https://example.com/page">link</a>"#;
        let result = resolve_relative_urls(html);
        assert_eq!(result, html);
    }

    #[test]
    fn leaves_protocol_relative_urls_unchanged() {
        let html = r#"<a href="//cdn.example.com/file.js">link</a>"#;
        let result = resolve_relative_urls(html);
        assert_eq!(result, html);
    }

    #[test]
    fn leaves_mailto_links_unchanged() {
        let html = r#"<a href="mailto:test@example.com">email</a>"#;
        let result = resolve_relative_urls(html);
        assert_eq!(result, html);
    }

    #[test]
    fn leaves_anchor_links_unchanged() {
        let html = r##"<a href="#section">jump</a>"##;
        let result = resolve_relative_urls(html);
        assert_eq!(result, html);
    }

    #[test]
    fn rewrites_multiple_urls_in_same_content() {
        let html = r#"<a href="/blog/one">one</a> and <a href="/blog/two">two</a> and <img src="/img/photo.png">"#;
        let result = resolve_relative_urls(html);
        assert_eq!(
            result,
            r#"<a href="https://www.philipithomas.com/blog/one">one</a> and <a href="https://www.philipithomas.com/blog/two">two</a> and <img src="https://www.philipithomas.com/img/photo.png">"#
        );
    }

    #[test]
    fn handles_root_path() {
        let html = r#"<a href="/">home</a>"#;
        let result = resolve_relative_urls(html);
        assert_eq!(
            result,
            r#"<a href="https://www.philipithomas.com/">home</a>"#
        );
    }

    #[test]
    fn does_not_double_rewrite_already_absolute() {
        let html = r#"<a href="https://www.philipithomas.com/blog/post">link</a>"#;
        let result = resolve_relative_urls(html);
        assert_eq!(result, html);
    }

    #[test]
    fn mixed_absolute_and_relative() {
        let html = r#"<a href="/local">local</a> <a href="https://external.com">ext</a> <img src="/img.png">"#;
        let result = resolve_relative_urls(html);
        assert_eq!(
            result,
            r#"<a href="https://www.philipithomas.com/local">local</a> <a href="https://external.com">ext</a> <img src="https://www.philipithomas.com/img.png">"#
        );
    }
}
