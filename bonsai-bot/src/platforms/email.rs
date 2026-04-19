#![cfg(feature = "email")]

use std::sync::Arc;
use async_trait::async_trait;
use tokio::time::{sleep, Duration};

use crate::admin_api::PlatformStates;
use crate::config::EmailConfig;
use crate::dedup::email_fallback_key;
use crate::metrics::SharedMetrics;
use crate::platforms::{InboundMessage, MessagingPlatform, ShedNotice};

pub struct EmailPlatform {
    pub imap_password:   String,
    pub smtp_password:   String,
    pub config:          EmailConfig,
    pub metrics:         SharedMetrics,
    pub platform_states: PlatformStates,
}

impl EmailPlatform {
    pub fn new(
        imap_password:   String,
        smtp_password:   String,
        config:          EmailConfig,
        metrics:         SharedMetrics,
        platform_states: PlatformStates,
    ) -> Arc<Self> {
        Arc::new(Self { imap_password, smtp_password, config, metrics, platform_states })
    }

    fn dedup_key(&self, message_id: Option<&str>, from: &str, date: &str, subject: &str, body: &str) -> String {
        match message_id {
            Some(mid) if !mid.is_empty() => mid.to_string(),
            _ => email_fallback_key(from, date, subject, body),
        }
    }
}

#[async_trait]
impl MessagingPlatform for EmailPlatform {
    fn name(&self) -> &'static str { "email" }

    async fn run(
        self: Arc<Self>,
        tx: tokio::sync::mpsc::Sender<InboundMessage>,
        shed_tx: tokio::sync::mpsc::Sender<ShedNotice>,
    ) {
        self.platform_states.insert("email".to_string(), "polling".to_string());
        loop {
            if let Err(e) = self.poll_once(&tx, &shed_tx).await {
                tracing::warn!("[email] Poll error: {e}; retrying in {}s", self.config.poll_interval_secs);
                self.platform_states.insert("email".to_string(), "error".to_string());
            } else {
                self.platform_states.insert("email".to_string(), "polling".to_string());
            }
            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }

    async fn send_reply(
        &self,
        _chat_id: &str,
        to: &str,
        text: &str,
        in_reply_to: Option<&str>,
    ) -> Result<(), String> {
        use lettre::{Message as EmailMessage, SmtpTransport, Transport};
        use lettre::message::header::ContentType;
        use lettre::transport::smtp::authentication::Credentials;

        let html = crate::formatter::format(text, "email").chunks.join("\n");

        let from_addr = self.config.smtp_from.parse()
            .map_err(|e| format!("from: {e}"))?;
        let to_addr   = to.parse()
            .map_err(|e| format!("to: {e}"))?;

        let mut builder = EmailMessage::builder()
            .from(from_addr)
            .to(to_addr)
            .subject(format!("Re: {}", self.config.subject_prefix));

        if let Some(mid) = in_reply_to {
            builder = builder.in_reply_to(mid.to_string());
        }

        let email = builder
            .header(ContentType::TEXT_HTML)
            .body(html)
            .map_err(|e| format!("build email: {e}"))?;

        let creds = Credentials::new(
            self.config.smtp_username.clone(),
            self.smtp_password.clone(),
        );

        SmtpTransport::relay(&self.config.smtp_host)
            .map_err(|e| format!("smtp relay: {e}"))?
            .credentials(creds)
            .build()
            .send(&email)
            .map_err(|e| format!("smtp send: {e}"))?;

        Ok(())
    }

    async fn send_confirm_prompt(
        &self,
        chat_id: &str,
        user_id: &str,
        token: &str,
        prompt: &str,
        nonce: i64,
    ) -> Result<String, String> {
        let text = format!(
            "⚠️ Confirmation required (ref:{token}:{nonce})\n{prompt}\n\nReply 'yes' to approve or 'no' to deny (expires in 2 minutes)."
        );
        self.send_reply(chat_id, user_id, &text, None).await?;
        Ok(format!("{token}:{nonce}"))
    }
}

impl EmailPlatform {
    async fn poll_once(
        &self,
        tx: &tokio::sync::mpsc::Sender<InboundMessage>,
        shed_tx: &tokio::sync::mpsc::Sender<ShedNotice>,
    ) -> Result<(), String> {
        // IMAP polling using async-imap with native-tls
        // Note: async-imap 0.9 uses tokio + async-native-tls for TLS connections

        use async_native_tls::TlsConnector;
        use async_imap::Client;
        use futures::TryStreamExt;
        use tokio_util::compat::TokioAsyncReadCompatExt;

        let addr = format!("{}:{}", self.config.imap_host, self.config.imap_port);
        let tcp = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("TCP connect: {e}"))?;

        let tls = TlsConnector::new()
            .connect(&self.config.imap_host, tcp.compat())
            .await
            .map_err(|e| format!("TLS: {e}"))?;

        let client = Client::new(tls);
        let mut imap_session = client
            .login(&self.config.imap_username, &self.imap_password)
            .await
            .map_err(|(e, _)| format!("IMAP login: {e}"))?;

        imap_session.select("INBOX").await.map_err(|e| format!("SELECT: {e}"))?;

        // Safe search: UNSEEN SUBJECT only (FROM filter is applied in code)
        let query = format!("UNSEEN SUBJECT \"{}\"", self.config.subject_prefix);
        let uid_set = imap_session
            .uid_search(&query)
            .await
            .map_err(|e| format!("SEARCH: {e}"))?;

        for uid in uid_set.into_iter() {
            let uid_str = uid.to_string();
            let fetch_query = "(BODY[HEADER.FIELDS (FROM SUBJECT DATE MESSAGE-ID)] BODY.PEEK[TEXT]<0.2000>)";

            let messages: Vec<_> = imap_session
                .uid_fetch(&uid_str, fetch_query)
                .await
                .map_err(|e| format!("FETCH {uid}: {e}"))?
                .try_collect()
                .await
                .map_err(|e| format!("FETCH collect {uid}: {e}"))?;

            for msg in messages.iter() {
                let header_bytes = msg.header().unwrap_or_default();
                let body_bytes   = msg.text().unwrap_or_default();

                let headers    = parse_headers(header_bytes);
                let from       = headers.get("from").cloned().unwrap_or_default();
                let subject    = headers.get("subject").cloned().unwrap_or_default();
                let date       = headers.get("date").cloned().unwrap_or_default();
                let message_id = headers.get("message-id").cloned();
                let body       = String::from_utf8_lossy(body_bytes).to_string();

                // Code-side allowlist check
                let from_lower = from.to_lowercase();
                let allowed = self.config.allowed_from_addrs.iter()
                    .any(|a| from_lower.contains(&a.to_lowercase()));

                if !allowed {
                    let _ = imap_session.uid_store(&uid_str, "+FLAGS.SILENT (\\Seen)").await;
                    self.metrics.allowlist_denials.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    continue;
                }

                let event_id = self.dedup_key(message_id.as_deref(), &from, &date, &subject, &body);

                let inbound = InboundMessage {
                    platform:     "email".to_string(),
                    platform_id:  from.clone(),
                    user_id:      from.clone(),
                    display_name: from.clone(),
                    event_id,
                    text:         body,
                    reply_to:     message_id,
                };

                self.metrics.messages_inbound.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                if tx.try_send(inbound).is_err() {
                    self.metrics.messages_queued_full.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let _ = shed_tx.try_send(ShedNotice {
                        platform: "email".to_string(),
                        chat_id:  from.clone(),
                        user_id:  from,
                        reply_to: None,
                    });
                }

                let _ = imap_session.uid_store(&uid_str, "+FLAGS.SILENT (\\Seen)").await;
            }
        }

        imap_session.logout().await.map_err(|e| format!("LOGOUT: {e}"))?;
        Ok(())
    }
}

fn parse_headers(raw: &[u8]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let text = String::from_utf8_lossy(raw);
    for line in text.lines() {
        if let Some(colon) = line.find(':') {
            let key   = line[..colon].trim().to_lowercase();
            let value = line[colon + 1..].trim().to_string();
            map.entry(key).or_insert(value);
        }
    }
    map
}
