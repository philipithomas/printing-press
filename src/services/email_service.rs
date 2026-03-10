use crate::config::Config;

#[derive(Clone)]
pub struct EmailService {
    config: Config,
}

impl EmailService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn send_confirmation(
        &self,
        to: &str,
        code: &str,
        magic_link: &str,
        site_url: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let html = crate::templates::render_confirmation(code, magic_link, site_url)?;
        self.send_email(to, "Your sign-in code", &html).await
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.config.aws_region.clone()))
            .load()
            .await;

        let client = aws_sdk_ses::Client::new(&config);

        client
            .send_email()
            .source(&self.config.ses_from_email)
            .destination(
                aws_sdk_ses::types::Destination::builder()
                    .to_addresses(to)
                    .build(),
            )
            .message(
                aws_sdk_ses::types::Message::builder()
                    .subject(
                        aws_sdk_ses::types::Content::builder()
                            .data(subject)
                            .charset("UTF-8")
                            .build()
                            .expect("valid subject"),
                    )
                    .body(
                        aws_sdk_ses::types::Body::builder()
                            .html(
                                aws_sdk_ses::types::Content::builder()
                                    .data(html_body)
                                    .charset("UTF-8")
                                    .build()
                                    .expect("valid html"),
                            )
                            .build(),
                    )
                    .build(),
            )
            .send()
            .await?;

        tracing::info!("Email sent to {}", to);
        Ok(())
    }
}
