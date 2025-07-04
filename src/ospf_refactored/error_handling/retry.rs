// Retry Logic Implementation
//
// Provides configurable retry mechanisms with exponential backoff
// and circuit breaker functionality.

use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Configuration for retry behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries (milliseconds)
    pub initial_delay_ms: u32,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u32,
    /// Backoff multiplier
    pub backoff_multiplier: f32,
    /// Whether to add jitter to delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a config for immediate retries
    pub fn immediate() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
            jitter: false,
        }
    }
    
    /// Create a config for aggressive retries
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_ms: 50,
            max_delay_ms: 1000,
            backoff_multiplier: 1.5,
            jitter: true,
        }
    }
    
    /// Create a config for conservative retries
    pub fn conservative() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 3.0,
            jitter: true,
        }
    }
    
    /// Calculate delay for a given attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 || self.initial_delay_ms == 0 {
            return Duration::from_millis(0);
        }
        
        let mut delay = self.initial_delay_ms as f32 * self.backoff_multiplier.powi(attempt as i32 - 1);
        
        if delay > self.max_delay_ms as f32 {
            delay = self.max_delay_ms as f32;
        }
        
        if self.jitter {
            // Add up to 20% jitter
            let jitter_range = delay * 0.2;
            let jitter = (js_sys::Math::random() as f32 - 0.5) * 2.0 * jitter_range;
            delay += jitter;
        }
        
        Duration::from_millis(delay.max(0.0) as u64)
    }
}

/// Result of a retry operation
#[derive(Debug)]
pub enum RetryResult<T, E> {
    /// Operation succeeded
    Success(T),
    /// Operation failed after all retries
    Failed(E, u32), // Error and number of attempts
    /// Operation was aborted (e.g., circuit breaker)
    Aborted(E),
}

/// Policy for determining if an error is retryable
pub trait RetryPolicy<E> {
    /// Check if an error should trigger a retry
    fn should_retry(&self, error: &E, attempt: u32) -> bool;
}

/// Default retry policy that retries all errors
pub struct AlwaysRetryPolicy;

impl<E> RetryPolicy<E> for AlwaysRetryPolicy {
    fn should_retry(&self, _error: &E, attempt: u32) -> bool {
        attempt < 10 // Safety limit
    }
}

/// Retry policy based on error type
pub struct SelectiveRetryPolicy<E> {
    /// Function to check if error is retryable
    is_retryable: Box<dyn Fn(&E) -> bool>,
}

impl<E> SelectiveRetryPolicy<E> {
    pub fn new<F>(is_retryable: F) -> Self
    where
        F: Fn(&E) -> bool + 'static,
    {
        Self {
            is_retryable: Box::new(is_retryable),
        }
    }
}

impl<E> RetryPolicy<E> for SelectiveRetryPolicy<E> {
    fn should_retry(&self, error: &E, _attempt: u32) -> bool {
        (self.is_retryable)(error)
    }
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Failure threshold before opening
    pub failure_threshold: u32,
    /// Time to wait before attempting reset (milliseconds)
    pub reset_timeout_ms: u32,
    /// Current failure count
    pub failure_count: u32,
    /// Last failure timestamp
    pub last_failure: Option<f64>,
    /// Circuit state
    pub state: CircuitState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing, rejecting requests
    HalfOpen,  // Testing if service recovered
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout_ms: u32) -> Self {
        Self {
            failure_threshold,
            reset_timeout_ms,
            failure_count: 0,
            last_failure: None,
            state: CircuitState::Closed,
        }
    }
    
    /// Record a success
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.state = CircuitState::Closed;
    }
    
    /// Record a failure
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(js_sys::Date::now());
        
        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }
    
    /// Check if requests should be allowed
    pub fn should_allow_request(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure {
                    let elapsed = js_sys::Date::now() - last_failure;
                    if elapsed >= self.reset_timeout_ms as f64 {
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
}

/// Synchronous retry executor
pub fn retry_sync<T, E, F>(
    config: &RetryConfig,
    policy: &impl RetryPolicy<E>,
    mut operation: F,
) -> RetryResult<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut attempt = 0;
    let mut last_error = None;
    
    while attempt < config.max_attempts {
        attempt += 1;
        
        match operation() {
            Ok(result) => return RetryResult::Success(result),
            Err(error) => {
                if !policy.should_retry(&error, attempt) || attempt >= config.max_attempts {
                    return RetryResult::Failed(error, attempt);
                }
                
                last_error = Some(error);
                
                if attempt < config.max_attempts {
                    let delay = config.calculate_delay(attempt);
                    if !delay.is_zero() {
                        // In WASM, we can't sleep, so this is a placeholder
                        // In real implementation, this would be async
                        #[cfg(not(target_arch = "wasm32"))]
                        std::thread::sleep(delay);
                    }
                }
            }
        }
    }
    
    RetryResult::Failed(
        last_error.expect("Should have error after retry loop"),
        attempt,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_retry_config_delay_calculation() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            backoff_multiplier: 2.0,
            jitter: false,
        };
        
        assert_eq!(config.calculate_delay(0), Duration::from_millis(0));
        assert_eq!(config.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(config.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(config.calculate_delay(3), Duration::from_millis(400));
        assert_eq!(config.calculate_delay(4), Duration::from_millis(800));
        assert_eq!(config.calculate_delay(5), Duration::from_millis(1000)); // Capped at max
    }
    
    #[test]
    fn test_circuit_breaker() {
        let mut breaker = CircuitBreaker::new(3, 5000);
        
        assert_eq!(breaker.state, CircuitState::Closed);
        assert!(breaker.should_allow_request());
        
        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state, CircuitState::Closed);
        
        breaker.record_failure();
        assert_eq!(breaker.state, CircuitState::Open);
        assert!(!breaker.should_allow_request());
        
        // Success resets
        breaker.state = CircuitState::HalfOpen;
        breaker.record_success();
        assert_eq!(breaker.state, CircuitState::Closed);
        assert_eq!(breaker.failure_count, 0);
    }
}