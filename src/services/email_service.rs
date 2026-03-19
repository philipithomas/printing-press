use crate::config::Config;

const FROM_NAME: &str = "Philip I. Thomas";

enum EmailBackend {
    Ses(SesEmailSender),
    Smtp(SmtpEmailSender),
}

struct SesEmailSender {
    client: aws_sdk_ses::Client,
}

impl SesEmailSender {
    async fn new(region: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;
        Self {
            client: aws_sdk_ses::Client::new(&config),
        }
    }

    async fn send_simple(
        &self,
        to: &str,
        from: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .send_email()
            .source(from)
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

        tracing::info!("Email sent to {} via SES", to);
        Ok(())
    }

    async fn send_raw(
        &self,
        message: &lettre::Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .send_raw_email()
            .raw_message(
                aws_sdk_ses::types::RawMessage::builder()
                    .data(aws_sdk_ses::primitives::Blob::new(message.formatted()))
                    .build()
                    .expect("valid raw message"),
            )
            .send()
            .await?;

        Ok(())
    }
}

struct SmtpEmailSender {
    host: String,
    port: u16,
}

impl SmtpEmailSender {
    async fn send(
        &self,
        message: lettre::Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.host)
            .port(self.port)
            .build();

        transport.send(message).await?;
        Ok(())
    }
}

/// List-Unsubscribe email header for one-click unsubscribe (RFC 2369).
#[derive(Clone)]
struct ListUnsubscribeHeader(String);

impl lettre::message::header::Header for ListUnsubscribeHeader {
    fn name() -> lettre::message::header::HeaderName {
        lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe")
    }

    fn parse(s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self(s.to_string()))
    }

    fn display(&self) -> lettre::message::header::HeaderValue {
        lettre::message::header::HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// List-Unsubscribe-Post header for RFC 8058 one-click unsubscribe.
#[derive(Clone)]
struct ListUnsubscribePostHeader;

impl lettre::message::header::Header for ListUnsubscribePostHeader {
    fn name() -> lettre::message::header::HeaderName {
        lettre::message::header::HeaderName::new_from_ascii_str("List-Unsubscribe-Post")
    }

    fn parse(_s: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self)
    }

    fn display(&self) -> lettre::message::header::HeaderValue {
        lettre::message::header::HeaderValue::new(
            Self::name(),
            "List-Unsubscribe=One-Click".to_string(),
        )
    }
}

fn build_simple_message(
    to: &str,
    from: &str,
    subject: &str,
    html_body: &str,
) -> Result<lettre::Message, Box<dyn std::error::Error + Send + Sync>> {
    use lettre::message::header::ContentType;

    let email = lettre::Message::builder()
        .from(from.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html_body.to_string())?;

    Ok(email)
}

fn build_newsletter_message(
    to: &str,
    from: &str,
    subject: &str,
    html_body: &str,
    unsubscribe_url: &str,
    unsubscribe_post_url: &str,
    preview_text: Option<&str>,
) -> Result<lettre::Message, Box<dyn std::error::Error + Send + Sync>> {
    use lettre::message::header::ContentType;
    use lettre::message::{MultiPart, SinglePart};

    let plain_body = match preview_text {
        Some(pt) if !pt.is_empty() => format!("{}\n\n{}", pt, subject),
        _ => subject.to_string(),
    };

    let email = lettre::Message::builder()
        .from(from.parse()?)
        .to(to.parse()?)
        .subject(subject)
        .header(ListUnsubscribeHeader(format!(
            "<{}>, <{}>",
            unsubscribe_post_url, unsubscribe_url
        )))
        .header(ListUnsubscribePostHeader)
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(plain_body),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body.to_string()),
                ),
        )?;

    Ok(email)
}

#[derive(Clone)]
pub struct EmailService {
    backend: std::sync::Arc<EmailBackend>,
    from_email: String,
}

impl EmailService {
    pub async fn new(config: &Config) -> Self {
        let backend = match config.email_backend.as_str() {
            "ses" => EmailBackend::Ses(SesEmailSender::new(&config.aws_region).await),
            _ => EmailBackend::Smtp(SmtpEmailSender {
                host: config.smtp_host.clone(),
                port: config.smtp_port,
            }),
        };
        let from_email = if config.ses_from_email.contains('<') {
            config.ses_from_email.clone()
        } else {
            format!("{} <{}>", FROM_NAME, config.ses_from_email)
        };
        Self {
            backend: std::sync::Arc::new(backend),
            from_email,
        }
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.backend.as_ref() {
            EmailBackend::Ses(ses) => {
                ses.send_simple(to, &self.from_email, subject, html_body)
                    .await
            }
            EmailBackend::Smtp(smtp) => {
                let message = build_simple_message(to, &self.from_email, subject, html_body)?;
                smtp.send(message).await?;
                tracing::info!("Email sent to {} via SMTP", to);
                Ok(())
            }
        }
    }

    /// Send a newsletter email with List-Unsubscribe headers for one-click unsubscribe.
    pub async fn send_newsletter(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        unsubscribe_url: &str,
        unsubscribe_post_url: &str,
        preview_text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = build_newsletter_message(
            to,
            &self.from_email,
            subject,
            html_body,
            unsubscribe_url,
            unsubscribe_post_url,
            preview_text,
        )?;
        match self.backend.as_ref() {
            EmailBackend::Ses(ses) => {
                ses.send_raw(&message).await?;
                tracing::info!("Newsletter sent to {} via SES", to);
            }
            EmailBackend::Smtp(smtp) => {
                smtp.send(message).await?;
                tracing::info!("Newsletter sent to {} via SMTP", to);
            }
        }
        Ok(())
    }

    pub async fn send_confirmation(
        &self,
        to: &str,
        code: &str,
        magic_link: &str,
        site_url: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let html = crate::templates::render_confirmation(code, magic_link, site_url)?;
        self.send_email(to, "Your sign-in code for philipithomas.com", &html)
            .await
    }

    pub async fn send_new_subscriber_notification(
        &self,
        subscriber_email: &str,
        subscriber_name: Option<&str>,
        subscriber_source: Option<&str>,
        site_url: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let html = crate::templates::render_new_subscriber(
            subscriber_email,
            subscriber_name,
            subscriber_source,
            site_url,
        )?;
        let subject = format!("New subscriber: {}", subscriber_email);
        // Send to the same address configured as the from email
        self.send_email(&self.from_email, &subject, &html).await
    }
}
