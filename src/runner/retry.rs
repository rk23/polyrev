use crate::config::RetryConfig;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::warn;

/// Execute an async operation with jittered exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut backoff_ms = config.backoff_base_ms;

    loop {
        attempts += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts >= config.max_attempts => {
                warn!("All {} attempts failed: {}", attempts, e);
                return Err(e);
            }
            Err(e) => {
                // Jittered backoff: base * 2^attempt + random(0..base)
                let jitter = rand::random::<u64>() % config.backoff_base_ms;
                let delay = Duration::from_millis(backoff_ms + jitter);

                warn!(
                    "Attempt {} failed: {}. Retrying in {:?}...",
                    attempts, e, delay
                );

                sleep(delay).await;
                backoff_ms = backoff_ms.saturating_mul(2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff_base_ms: 10,
        };

        let result: Result<i32, &str> = retry_with_backoff(&config, || async { Ok(42) }).await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff_base_ms: 10,
        };

        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<i32, &str> = retry_with_backoff(&config, || {
            let attempts = attempts_clone.clone();
            async move {
                let n = attempts.fetch_add(1, Ordering::SeqCst);
                if n < 2 {
                    Err("not yet")
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_all_failures() {
        let config = RetryConfig {
            max_attempts: 3,
            backoff_base_ms: 10,
        };

        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<i32, &str> = retry_with_backoff(&config, || {
            let attempts = attempts_clone.clone();
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err("always fails")
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }
}
