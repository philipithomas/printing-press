use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use hickory_resolver::TokioResolver;
use hickory_resolver::proto::ProtoErrorKind;
use hickory_resolver::proto::rr::RecordType;
use tokio::sync::RwLock;

const CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct MxValidator {
    resolver: TokioResolver,
    cache: Arc<RwLock<HashMap<String, (bool, Instant)>>>,
}

fn is_no_records_found(err: &hickory_resolver::ResolveError) -> bool {
    match err.kind() {
        hickory_resolver::ResolveErrorKind::Proto(proto_err) => {
            matches!(proto_err.kind(), ProtoErrorKind::NoRecordsFound { .. })
        }
        _ => false,
    }
}

impl Default for MxValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MxValidator {
    pub fn new() -> Self {
        let resolver = TokioResolver::builder_tokio()
            .expect("failed to create DNS resolver builder")
            .build();
        Self {
            resolver,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn has_mx_records(&self, email: &str) -> Result<bool, String> {
        let domain = email
            .rsplit_once('@')
            .map(|(_, d)| d)
            .ok_or_else(|| format!("invalid email: {}", email))?;

        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((result, expires)) = cache.get(domain)
                && Instant::now() < *expires
            {
                return Ok(*result);
            }
        }

        let result = self.lookup_mx(domain).await;

        // Cache on success
        if let Ok(has_mx) = &result {
            let mut cache = self.cache.write().await;
            cache.insert(domain.to_string(), (*has_mx, Instant::now() + CACHE_TTL));
        }

        result
    }

    async fn lookup_mx(&self, domain: &str) -> Result<bool, String> {
        match self.resolver.mx_lookup(domain).await {
            Ok(mx) => Ok(mx.iter().next().is_some()),
            Err(e) => {
                if is_no_records_found(&e) {
                    // RFC 5321 §5.1: if no MX, fall back to A record
                    self.lookup_a(domain).await
                } else {
                    Err(format!("DNS lookup failed for {}: {}", domain, e))
                }
            }
        }
    }

    async fn lookup_a(&self, domain: &str) -> Result<bool, String> {
        match self.resolver.lookup(domain, RecordType::A).await {
            Ok(lookup) => Ok(lookup.iter().next().is_some()),
            Err(e) => {
                if is_no_records_found(&e) {
                    Ok(false)
                } else {
                    Err(format!("DNS A lookup failed for {}: {}", domain, e))
                }
            }
        }
    }
}
