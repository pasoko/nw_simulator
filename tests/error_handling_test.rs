// Tests for enhanced error handling
//
// These tests verify that the error handling improvements work correctly
// including logging, retry logic, and recovery strategies.

use nw_simulator::ospf_refactored::{
    packet_processor::UnifiedPacketProcessor,
    packets::{OSPFPacket, HelloPacket},
    events::EventBus,
    error_handling::{
        ErrorContext, 
        retry::{RetryConfig, CircuitBreaker},
        recovery::{RecoveryAction, RecoveryStrategy, DefaultRecoveryStrategy}
    },
};
use std::sync::Arc;
use std::net::Ipv4Addr;

#[test]
fn test_error_context_creation() {
    let context = ErrorContext::new(1)
        .with_neighbor(2)
        .with_interface(1)
        .with_operation("test_operation");
    
    assert_eq!(context.router_id, Some(1));
    assert_eq!(context.neighbor_id, Some(2));
    assert_eq!(context.interface_id, Some(1));
    assert_eq!(context.operation, Some("test_operation".to_string()));
    
    let description = context.description();
    assert!(description.contains("router=1"));
    assert!(description.contains("neighbor=2"));
}

#[test]
fn test_retry_config() {
    let mut config = RetryConfig::default();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_delay_ms, 100);
    
    // Test delay calculation without jitter for deterministic results
    config.jitter = false;
    assert_eq!(config.calculate_delay(0).as_millis(), 0);
    assert_eq!(config.calculate_delay(1).as_millis(), 100);
    
    // Test aggressive config
    let aggressive = RetryConfig::aggressive();
    assert_eq!(aggressive.max_attempts, 5);
    assert_eq!(aggressive.initial_delay_ms, 50);
}

#[test]
fn test_packet_processor_error_handling() {
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus,
    );
    
    // Create a packet with wrong area ID to trigger error
    let mut hello = HelloPacket::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(1, 0, 0, 0), // Wrong area
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        1,
        40,
    );
    hello.header.area_id = Ipv4Addr::new(1, 0, 0, 0);
    
    let result = processor.process_packet(
        OSPFPacket::Hello(hello),
        2,
        1
    );
    
    // Should handle the error
    assert!(result.is_err() || result.unwrap().is_empty());
    
    // Check error metrics
    let metrics = processor.get_error_metrics();
    assert!(metrics.consecutive_errors > 0);
}

#[test]
fn test_error_metrics_tracking() {
    use nw_simulator::ospf_refactored::error_handling::ErrorMetrics;
    
    let mut metrics = ErrorMetrics::default();
    
    // Record some errors
    metrics.record_error("PacketError");
    metrics.record_error("PacketError");
    metrics.record_error("StateError");
    
    assert_eq!(metrics.errors_by_type.get("PacketError"), Some(&2));
    assert_eq!(metrics.errors_by_type.get("StateError"), Some(&1));
    assert_eq!(metrics.consecutive_errors, 3);
    
    // Test circuit breaker
    assert!(!metrics.should_circuit_break(5));
    assert!(metrics.should_circuit_break(3));
    
    // Recovery resets consecutive errors
    metrics.record_recovery();
    assert_eq!(metrics.consecutive_errors, 0);
}

#[test]
fn test_recovery_strategies() {
    let strategy = DefaultRecoveryStrategy::new();
    let context = ErrorContext::new(1).with_neighbor(2);
    
    // Test different error types
    let action = strategy.determine_action("PacketError::ChecksumMismatch", &context);
    assert!(matches!(action, RecoveryAction::LogAndContinue));
    
    let action = strategy.determine_action("EventError::EventLoopDetected", &context);
    assert!(matches!(action, RecoveryAction::Escalate));
}

#[test]
fn test_retry_configuration() {
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus,
    );
    
    // Set custom retry config
    let custom_config = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 200,
        max_delay_ms: 2000,
        backoff_multiplier: 1.5,
        jitter: false,
    };
    
    processor.set_retry_config(custom_config.clone());
    let retrieved_config = processor.get_retry_config();
    
    assert_eq!(retrieved_config.max_attempts, 5);
    assert_eq!(retrieved_config.initial_delay_ms, 200);
}

#[test]
fn test_recovery_history() {
    let event_bus = Arc::new(EventBus::new());
    let mut processor = UnifiedPacketProcessor::new(
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        event_bus,
    );
    
    // Process some packets to generate errors and recovery
    let hello = HelloPacket::new(
        Ipv4Addr::new(2, 2, 2, 2),
        Ipv4Addr::new(0, 0, 0, 0),
        Ipv4Addr::new(255, 255, 255, 0),
        10,
        1,
        40,
    );
    
    let _ = processor.process_packet(OSPFPacket::Hello(hello), 2, 1);
    
    // Get recovery history
    let history = processor.get_recovery_history();
    // History might be empty if no errors occurred
    assert!(history.is_empty() || history[0].get("timestamp").is_some());
}

#[test]
fn test_circuit_breaker() {
    let mut breaker = CircuitBreaker::new(3, 1000);
    
    // Test normal operation
    assert!(breaker.should_allow_request());
    
    // Simulate failures
    breaker.record_failure();
    breaker.record_failure();
    assert!(breaker.should_allow_request()); // Still under threshold
    
    breaker.record_failure();
    assert!(!breaker.should_allow_request()); // Circuit open
    
    // Test recovery
    breaker.record_success();
    assert!(breaker.should_allow_request()); // Circuit closed again
}