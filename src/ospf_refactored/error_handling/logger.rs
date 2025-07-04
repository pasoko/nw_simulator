// Error Logging Implementation
//
// Provides structured logging for errors with different severity levels
// and integration with the console_log macro.

use std::fmt;
use serde::{Serialize, Deserialize};

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

/// Log levels for error messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Trait for types that can log errors
pub trait ErrorLogger {
    /// Log an error with context
    fn log_error(&self, level: LogLevel, context: &str);
    
    /// Log with additional metadata
    fn log_with_metadata(&self, level: LogLevel, context: &str, metadata: serde_json::Value);
}

/// Default error logger implementation
pub struct DefaultErrorLogger {
    /// Minimum log level to output
    pub min_level: LogLevel,
    /// Whether to include timestamp
    pub include_timestamp: bool,
}

impl Default for DefaultErrorLogger {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            include_timestamp: true,
        }
    }
}

impl DefaultErrorLogger {
    /// Create a new logger with specified minimum level
    pub fn new(min_level: LogLevel) -> Self {
        Self {
            min_level,
            include_timestamp: true,
        }
    }
    
    /// Format log message
    fn format_message(&self, level: LogLevel, context: &str, message: &str) -> String {
        if self.include_timestamp {
            let timestamp = get_timestamp();
            format!("[{:.3}] [{}] {}: {}", timestamp, level, context, message)
        } else {
            format!("[{}] {}: {}", level, context, message)
        }
    }
}

impl<T: fmt::Display> ErrorLogger for T {
    fn log_error(&self, level: LogLevel, context: &str) {
        let logger = DefaultErrorLogger::default();
        if level >= logger.min_level {
            let message = logger.format_message(level, context, &self.to_string());
            
            // Use the console_log macro from lib.rs
            #[cfg(target_arch = "wasm32")]
            crate::console_log!("{}", message);
            
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("{}", message);
        }
    }
    
    fn log_with_metadata(&self, level: LogLevel, context: &str, metadata: serde_json::Value) {
        let logger = DefaultErrorLogger::default();
        if level >= logger.min_level {
            let message = format!("{} | metadata: {}", self.to_string(), metadata);
            let formatted = logger.format_message(level, context, &message);
            
            #[cfg(target_arch = "wasm32")]
            crate::console_log!("{}", formatted);
            
            #[cfg(not(target_arch = "wasm32"))]
            eprintln!("{}", formatted);
        }
    }
}

/// Structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: f64,
    pub level: LogLevel,
    pub context: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub error_type: Option<String>,
}

/// Log buffer for collecting errors
pub struct ErrorLogBuffer {
    entries: Vec<LogEntry>,
    max_entries: usize,
}

impl ErrorLogBuffer {
    /// Create a new log buffer
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_entries),
            max_entries,
        }
    }
    
    /// Add a log entry
    pub fn add_entry(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }
    
    /// Get all entries
    pub fn get_entries(&self) -> &[LogEntry] {
        &self.entries
    }
    
    /// Get entries filtered by level
    pub fn get_entries_by_level(&self, min_level: LogLevel) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.level >= min_level)
            .collect()
    }
    
    /// Clear the buffer
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    /// Export entries as JSON
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.entries)
    }
}

/// Macro for easy error logging
#[macro_export]
macro_rules! log_ospf_error {
    ($level:expr, $context:expr, $($arg:tt)*) => {
        {
            use $crate::ospf_refactored::error_handling::ErrorLogger;
            let message = format!($($arg)*);
            message.log_error($level, $context);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warning);
        assert!(LogLevel::Warning < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Critical);
    }
    
    #[test]
    fn test_log_buffer() {
        let mut buffer = ErrorLogBuffer::new(3);
        
        for i in 0..5 {
            buffer.add_entry(LogEntry {
                timestamp: i as f64,
                level: LogLevel::Error,
                context: "test".to_string(),
                message: format!("Error {}", i),
                metadata: None,
                error_type: None,
            });
        }
        
        assert_eq!(buffer.get_entries().len(), 3);
        assert_eq!(buffer.get_entries()[0].message, "Error 2");
    }
}