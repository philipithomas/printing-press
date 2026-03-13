use minijinja::{Environment, context};

#[allow(clippy::too_many_arguments)]
pub fn render_letter(
    title: &str,
    subtitle: Option<&str>,
    published_at: Option<&str>,
    html_content: &str,
    cover_image: Option<&str>,
    cover_image_alt: Option<&str>,
    newsletter: &str,
    logo_file: &str,
    site_url: &str,
    qr_svg: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut env = Environment::new();
    env.add_template("letter", include_str!("letter.html"))?;
    let tmpl = env.get_template("letter")?;
    let result = tmpl.render(context! {
        title => title,
        subtitle => subtitle.unwrap_or(""),
        published_at => published_at.unwrap_or(""),
        html_content => html_content,
        cover_image => cover_image.unwrap_or(""),
        cover_image_alt => cover_image_alt.unwrap_or(title),
        newsletter => newsletter,
        logo_file => logo_file,
        site_url => site_url,
        qr_svg => qr_svg,
    })?;
    Ok(result)
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
        site_title => "Philip I. Thomas",
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
    let result = tmpl.render(context! {
        content => content,
        unsubscribe_url => unsubscribe_url,
        site_url => site_url,
        site_title => "Philip I. Thomas",
        newsletter => newsletter.unwrap_or(""),
        current_year => chrono::Utc::now().format("%Y").to_string(),
    })?;
    Ok(result)
}
