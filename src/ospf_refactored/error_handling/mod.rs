// Enhanced Error Handling Module
//
// This module provides improved error handling with logging,
// retry logic, and recovery strategies.

pub mod logger;
pub mod retry;
pub mod recovery;
pub mod context;

use std::fmt;
use serde::{Serialize, Deserialize};

pub use logger::{ErrorLogger, LogLevel};
pub use retry::{RetryPolicy, RetryConfig, RetryResult};
pub use recovery::{RecoveryStrategy, RecoveryAction, RecoveryCoordinator};
pub use context::ErrorContext;

/// Enhanced error type that includes context and recovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedError<E> {
    /// The original error
    pub error: E,
    /// Context information
    pub context: ErrorContext,
    /// Suggested recovery action
    pub recovery: Option<RecoveryAction>,
    /// Whether retry is recommended
    pub retryable: bool,
}

impl<E: fmt::Display> fmt::Display for EnhancedError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.context, self.error)?;
        if let Some(recovery) = &self.recovery {
            write!(f, " (Recovery: {:?})", recovery)?;
        }
        Ok(())
    }
}

impl<E: std::error::Error> std::error::Error for EnhancedError<E> {}

/// Trait for errors that can be enhanced with context
pub trait EnhanceableError: Sized {
    /// Add context to this error
    fn with_context(self, context: ErrorContext) -> EnhancedError<Self>;
    
    /// Mark this error as retryable
    fn retryable(self) -> EnhancedError<Self>
    where
        Self: Clone,
    {
        self.clone().with_context(ErrorContext::default()).with_retry(true)
    }
}

impl<E> EnhancedError<E> {
    /// Create a new enhanced error
    pub fn new(error: E, context: ErrorContext) -> Self {
        Self {
            error,
            context,
            recovery: None,
            retryable: false,
        }
    }
    
    /// Set recovery action
    pub fn with_recovery(mut self, action: RecoveryAction) -> Self {
        self.recovery = Some(action);
        self
    }
    
    /// Set retryable flag
    pub fn with_retry(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }
}

/// Error metrics for monitoring
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Total errors by type
    pub errors_by_type: std::collections::HashMap<String, u64>,
    /// Error rate per minute
    pub error_rate: f64,
    /// Last error timestamp
    pub last_error: Option<f64>,
    /// Consecutive error count
    pub consecutive_errors: u32,
    /// Recovery success rate
    pub recovery_success_rate: f64,
}

impl ErrorMetrics {
    /// Record an error
    pub fn record_error(&mut self, error_type: &str) {
        *self.errors_by_type.entry(error_type.to_string()).or_insert(0) += 1;
        self.consecutive_errors += 1;
        self.last_error = Some(get_timestamp());
    }
    
    /// Record successful recovery
    pub fn record_recovery(&mut self) {
        self.consecutive_errors = 0;
    }
    
    /// Check if circuit breaker should trip
    pub fn should_circuit_break(&self, threshold: u32) -> bool {
        self.consecutive_errors >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enhanced_error_display() {
        let context = ErrorContext::new(1)
            .with_neighbor(2)
            .with_interface(1);
        
        let error = EnhancedError::new(
            "Test error",
            context,
        ).with_recovery(RecoveryAction::ResetNeighbor);
        
        let display = format!("{}", error);
        assert!(display.contains("Test error"));
        assert!(display.contains("ResetNeighbor"));
    }
    
    #[test]
    fn test_error_metrics() {
        let mut metrics = ErrorMetrics::default();
        
        metrics.record_error("PacketError");
        metrics.record_error("PacketError");
        metrics.record_error("StateError");
        
        assert_eq!(metrics.errors_by_type.get("PacketError"), Some(&2));
        assert_eq!(metrics.errors_by_type.get("StateError"), Some(&1));
        assert_eq!(metrics.consecutive_errors, 3);
        
        metrics.record_recovery();
        assert_eq!(metrics.consecutive_errors, 0);
    }
}

// Helper function for getting timestamp that works in both WASM and native
fn get_timestamp() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
}