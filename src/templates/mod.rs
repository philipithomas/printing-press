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

/// Add inline styles to `<a>` tags so links in email content match the website's
/// visual treatment: dark text with a newsletter-accent-colored underline.
pub fn style_content_links(html: &str, newsletter: Option<&str>) -> String {
    let accent = match newsletter {
        Some("contraption") => "#2b4a3e",
        Some("workshop") => "#6b4d3a",
        Some("postcard") => "#2c3e6b",
        _ => "#3B3834",
    };
    let style = format!(
        "color: #3B3834; text-decoration: underline; text-decoration-color: {}; text-underline-offset: 2px;",
        accent
    );
    let re = Regex::new(r#"<a ([^>]*?)>"#).unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        let attrs = &caps[1];
        if attrs.contains("style=") {
            caps[0].to_string()
        } else {
            format!(r#"<a style="{}" {}>"#, style, attrs)
        }
    })
    .into_owned()
}

/// Add inline styles to `<img>` tags in email content so images are constrained
/// to the content width and don't overflow in narrow reading panes.
pub fn style_content_images(html: &str) -> String {
    let re = Regex::new(r#"<img ([^>]*?)>"#).unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        let attrs = &caps[1];
        if attrs.contains("style=") {
            caps[0].to_string()
        } else {
            format!(
                r#"<img style="max-width: 100%; height: auto; display: block;" {}>"#,
                attrs
            )
        }
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

pub fn render_new_subscriber(
    subscriber_email: &str,
    subscriber_name: Option<&str>,
    subscriber_source: Option<&str>,
    site_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut env = Environment::new();
    env.add_template("new_subscriber", include_str!("new_subscriber.html"))?;
    let tmpl = env.get_template("new_subscriber")?;
    let result = tmpl.render(context! {
        subscriber_email => subscriber_email,
        subscriber_name => subscriber_name,
        subscriber_source => subscriber_source,
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
    let content = style_content_links(&content, newsletter);
    let content = style_content_images(&content);
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

    #[test]
    fn styles_links_with_contraption_accent() {
        let html = r#"<a href="https://example.com">link</a>"#;
        let result = style_content_links(html, Some("contraption"));
        assert_eq!(
            result,
            r#"<a style="color: #3B3834; text-decoration: underline; text-decoration-color: #2b4a3e; text-underline-offset: 2px;" href="https://example.com">link</a>"#
        );
    }

    #[test]
    fn styles_links_with_workshop_accent() {
        let html = r#"<a href="https://example.com">link</a>"#;
        let result = style_content_links(html, Some("workshop"));
        assert_eq!(
            result,
            r#"<a style="color: #3B3834; text-decoration: underline; text-decoration-color: #6b4d3a; text-underline-offset: 2px;" href="https://example.com">link</a>"#
        );
    }

    #[test]
    fn styles_links_with_postcard_accent() {
        let html = r#"<a href="https://example.com">link</a>"#;
        let result = style_content_links(html, Some("postcard"));
        assert_eq!(
            result,
            r#"<a style="color: #3B3834; text-decoration: underline; text-decoration-color: #2c3e6b; text-underline-offset: 2px;" href="https://example.com">link</a>"#
        );
    }

    #[test]
    fn styles_links_with_default_accent() {
        let html = r#"<a href="https://example.com">link</a>"#;
        let result = style_content_links(html, None);
        assert_eq!(
            result,
            r#"<a style="color: #3B3834; text-decoration: underline; text-decoration-color: #3B3834; text-underline-offset: 2px;" href="https://example.com">link</a>"#
        );
    }

    #[test]
    fn skips_links_with_existing_style() {
        let html = r#"<a style="color: red;" href="https://example.com">link</a>"#;
        let result = style_content_links(html, Some("contraption"));
        assert_eq!(result, html);
    }

    #[test]
    fn styles_multiple_links() {
        let html = r#"<a href="https://one.com">one</a> and <a href="https://two.com">two</a>"#;
        let result = style_content_links(html, Some("workshop"));
        let expected = r#"<a style="color: #3B3834; text-decoration: underline; text-decoration-color: #6b4d3a; text-underline-offset: 2px;" href="https://one.com">one</a> and <a style="color: #3B3834; text-decoration: underline; text-decoration-color: #6b4d3a; text-underline-offset: 2px;" href="https://two.com">two</a>"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn styles_img_without_style() {
        let html = r#"<img src="https://example.com/photo.jpg" alt="photo">"#;
        let result = style_content_images(html);
        assert_eq!(
            result,
            r#"<img style="max-width: 100%; height: auto; display: block;" src="https://example.com/photo.jpg" alt="photo">"#
        );
    }

    #[test]
    fn skips_img_with_existing_style() {
        let html = r#"<img style="width: 100px;" src="https://example.com/photo.jpg">"#;
        let result = style_content_images(html);
        assert_eq!(result, html);
    }

    #[test]
    fn styles_multiple_images() {
        let html = r#"<img src="a.jpg"> and <img src="b.jpg">"#;
        let result = style_content_images(html);
        assert_eq!(
            result,
            r#"<img style="max-width: 100%; height: auto; display: block;" src="a.jpg"> and <img style="max-width: 100%; height: auto; display: block;" src="b.jpg">"#
        );
    }

    #[test]
    fn styles_images_mixed_with_styled() {
        let html = r#"<img src="a.jpg"> and <img style="width: 50px;" src="b.jpg">"#;
        let result = style_content_images(html);
        assert_eq!(
            result,
            r#"<img style="max-width: 100%; height: auto; display: block;" src="a.jpg"> and <img style="width: 50px;" src="b.jpg">"#
        );
    }
}
