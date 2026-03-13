use minijinja::{Environment, context};

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
