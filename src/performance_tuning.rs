use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::console_log;

/// Performance Tuning Module
/// 
/// This module provides performance optimization features for large-scale OSPF network simulations.
/// It includes various tuning parameters and optimization strategies to improve simulation performance.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    /// Profile name
    pub name: String,
    /// Maximum number of concurrent packet events to process
    pub max_concurrent_events: usize,
    /// Batch size for LSA processing
    pub lsa_batch_size: usize,
    /// SPF calculation throttling (ms between calculations)
    pub spf_throttle_ms: u32,
    /// Enable/disable packet aggregation
    pub packet_aggregation: bool,
    /// Maximum aggregation window (ms)
    pub aggregation_window_ms: u32,
    /// Enable/disable lazy LSA aging
    pub lazy_lsa_aging: bool,
    /// LSA aging check interval (seconds)
    pub lsa_aging_interval: u32,
    /// Enable/disable route cache
    pub route_caching: bool,
    /// Route cache TTL (seconds)
    pub route_cache_ttl: u32,
    /// Memory pool size for packet objects
    pub packet_pool_size: usize,
    /// Enable/disable parallel processing
    pub parallel_processing: bool,
    /// Number of worker threads (0 = auto)
    pub worker_threads: usize,
}

impl Default for PerformanceProfile {
    fn default() -> Self {
        PerformanceProfile {
            name: "default".to_string(),
            max_concurrent_events: 100,
            lsa_batch_size: 50,
            spf_throttle_ms: 100,
            packet_aggregation: false,
            aggregation_window_ms: 10,
            lazy_lsa_aging: true,
            lsa_aging_interval: 60,
            route_caching: true,
            route_cache_ttl: 300,
            packet_pool_size: 1000,
            parallel_processing: false,
            worker_threads: 0,
        }
    }
}

/// Predefined performance profiles
pub struct PerformanceProfiles;

impl PerformanceProfiles {
    /// Small network profile (< 50 routers)
    pub fn small_network() -> PerformanceProfile {
        PerformanceProfile {
            name: "small_network".to_string(),
            max_concurrent_events: 50,
            lsa_batch_size: 20,
            spf_throttle_ms: 50,
            packet_aggregation: false,
            aggregation_window_ms: 5,
            lazy_lsa_aging: false,
            lsa_aging_interval: 30,
            route_caching: false,
            route_cache_ttl: 60,
            packet_pool_size: 500,
            parallel_processing: false,
            worker_threads: 0,
        }
    }
    
    /// Medium network profile (50-200 routers)
    pub fn medium_network() -> PerformanceProfile {
        PerformanceProfile {
            name: "medium_network".to_string(),
            max_concurrent_events: 100,
            lsa_batch_size: 50,
            spf_throttle_ms: 100,
            packet_aggregation: true,
            aggregation_window_ms: 10,
            lazy_lsa_aging: true,
            lsa_aging_interval: 60,
            route_caching: true,
            route_cache_ttl: 300,
            packet_pool_size: 1000,
            parallel_processing: false,
            worker_threads: 0,
        }
    }
    
    /// Large network profile (> 200 routers)
    pub fn large_network() -> PerformanceProfile {
        PerformanceProfile {
            name: "large_network".to_string(),
            max_concurrent_events: 200,
            lsa_batch_size: 100,
            spf_throttle_ms: 200,
            packet_aggregation: true,
            aggregation_window_ms: 20,
            lazy_lsa_aging: true,
            lsa_aging_interval: 120,
            route_caching: true,
            route_cache_ttl: 600,
            packet_pool_size: 2000,
            parallel_processing: true,
            worker_threads: 0, // Auto-detect
        }
    }
    
    /// Real-time profile (prioritize responsiveness)
    pub fn real_time() -> PerformanceProfile {
        PerformanceProfile {
            name: "real_time".to_string(),
            max_concurrent_events: 50,
            lsa_batch_size: 10,
            spf_throttle_ms: 10,
            packet_aggregation: false,
            aggregation_window_ms: 1,
            lazy_lsa_aging: false,
            lsa_aging_interval: 10,
            route_caching: true,
            route_cache_ttl: 60,
            packet_pool_size: 500,
            parallel_processing: true,
            worker_threads: 2,
        }
    }
}

/// Performance metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total packets processed
    pub packets_processed: u64,
    /// Average packet processing time (μs)
    pub avg_packet_processing_us: f64,
    /// Peak packet processing time (μs)
    pub peak_packet_processing_us: f64,
    /// Total SPF calculations
    pub spf_calculations: u64,
    /// Average SPF calculation time (ms)
    pub avg_spf_time_ms: f64,
    /// Peak SPF calculation time (ms)
    pub peak_spf_time_ms: f64,
    /// LSA database size
    pub lsa_database_size: usize,
    /// Memory usage (bytes)
    pub memory_usage_bytes: usize,
    /// Packet pool hit rate (%)
    pub packet_pool_hit_rate: f64,
    /// Route cache hit rate (%)
    pub route_cache_hit_rate: f64,
    /// Dropped packets due to overload
    pub dropped_packets: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        PerformanceMetrics {
            packets_processed: 0,
            avg_packet_processing_us: 0.0,
            peak_packet_processing_us: 0.0,
            spf_calculations: 0,
            avg_spf_time_ms: 0.0,
            peak_spf_time_ms: 0.0,
            lsa_database_size: 0,
            memory_usage_bytes: 0,
            packet_pool_hit_rate: 0.0,
            route_cache_hit_rate: 0.0,
            dropped_packets: 0,
        }
    }
}

/// Performance tuning manager
pub struct PerformanceTuner {
    /// Current performance profile
    current_profile: PerformanceProfile,
    /// Custom profiles
    custom_profiles: HashMap<String, PerformanceProfile>,
    /// Performance metrics
    metrics: PerformanceMetrics,
    /// Metric collection samples
    packet_processing_samples: Vec<f64>,
    spf_calculation_samples: Vec<f64>,
    /// Route cache
    route_cache: HashMap<String, (Vec<crate::router::RoutingTableEntry>, f64)>,
    /// Packet pool hit/miss counters
    packet_pool_hits: u64,
    packet_pool_misses: u64,
    /// Route cache hit/miss counters
    route_cache_hits: u64,
    route_cache_misses: u64,
}

impl PerformanceTuner {
    pub fn new() -> Self {
        PerformanceTuner {
            current_profile: PerformanceProfile::default(),
            custom_profiles: HashMap::new(),
            metrics: PerformanceMetrics::default(),
            packet_processing_samples: Vec::with_capacity(1000),
            spf_calculation_samples: Vec::with_capacity(100),
            route_cache: HashMap::new(),
            packet_pool_hits: 0,
            packet_pool_misses: 0,
            route_cache_hits: 0,
            route_cache_misses: 0,
        }
    }
    
    /// Set performance profile
    pub fn set_profile(&mut self, profile: PerformanceProfile) {
        console_log!("Setting performance profile: {}", profile.name);
        self.current_profile = profile;
        // Clear caches when profile changes
        self.route_cache.clear();
    }
    
    /// Get current profile
    pub fn get_profile(&self) -> &PerformanceProfile {
        &self.current_profile
    }
    
    /// Save custom profile
    pub fn save_custom_profile(&mut self, name: String, profile: PerformanceProfile) {
        self.custom_profiles.insert(name, profile);
    }
    
    /// Load custom profile
    pub fn load_custom_profile(&mut self, name: &str) -> Result<(), String> {
        if let Some(profile) = self.custom_profiles.get(name).cloned() {
            self.set_profile(profile);
            Ok(())
        } else {
            Err(format!("Profile '{}' not found", name))
        }
    }
    
    /// Auto-tune based on network size
    pub fn auto_tune(&mut self, router_count: usize) {
        let profile = if router_count < 50 {
            PerformanceProfiles::small_network()
        } else if router_count <= 200 {
            PerformanceProfiles::medium_network()
        } else {
            PerformanceProfiles::large_network()
        };
        
        console_log!("Auto-tuning for {} routers, selected profile: {}", 
            router_count, profile.name);
        self.set_profile(profile);
    }
    
    /// Record packet processing time
    pub fn record_packet_processing(&mut self, time_us: f64) {
        self.metrics.packets_processed += 1;
        self.packet_processing_samples.push(time_us);
        
        // Keep only last 1000 samples
        if self.packet_processing_samples.len() > 1000 {
            self.packet_processing_samples.remove(0);
        }
        
        // Update metrics
        if !self.packet_processing_samples.is_empty() {
            self.metrics.avg_packet_processing_us = 
                self.packet_processing_samples.iter().sum::<f64>() / 
                self.packet_processing_samples.len() as f64;
            
            self.metrics.peak_packet_processing_us = 
                self.packet_processing_samples.iter()
                    .fold(0.0, |max, &x| if x > max { x } else { max });
        }
    }
    
    /// Record SPF calculation time
    pub fn record_spf_calculation(&mut self, time_ms: f64) {
        self.metrics.spf_calculations += 1;
        self.spf_calculation_samples.push(time_ms);
        
        // Keep only last 100 samples
        if self.spf_calculation_samples.len() > 100 {
            self.spf_calculation_samples.remove(0);
        }
        
        // Update metrics
        if !self.spf_calculation_samples.is_empty() {
            self.metrics.avg_spf_time_ms = 
                self.spf_calculation_samples.iter().sum::<f64>() / 
                self.spf_calculation_samples.len() as f64;
            
            self.metrics.peak_spf_time_ms = 
                self.spf_calculation_samples.iter()
                    .fold(0.0, |max, &x| if x > max { x } else { max });
        }
    }
    
    /// Check if SPF should be throttled
    pub fn should_throttle_spf(&self, last_spf_time: f64, current_time: f64) -> bool {
        let elapsed_ms = (current_time - last_spf_time) * 1000.0;
        elapsed_ms < self.current_profile.spf_throttle_ms as f64
    }
    
    /// Get route from cache if available
    pub fn get_cached_route(&mut self, key: &str, current_time: f64) -> Option<Vec<crate::router::RoutingTableEntry>> {
        if !self.current_profile.route_caching {
            return None;
        }
        
        if let Some((routes, cached_time)) = self.route_cache.get(key) {
            if current_time - cached_time < self.current_profile.route_cache_ttl as f64 {
                self.route_cache_hits += 1;
                return Some(routes.clone());
            }
        }
        
        self.route_cache_misses += 1;
        None
    }
    
    /// Cache route calculation result
    pub fn cache_route(&mut self, key: String, routes: Vec<crate::router::RoutingTableEntry>, current_time: f64) {
        if self.current_profile.route_caching {
            self.route_cache.insert(key, (routes, current_time));
            
            // Limit cache size
            if self.route_cache.len() > 10000 {
                // Remove oldest entries
                let mut entries: Vec<_> = self.route_cache.iter()
                    .map(|(k, (_, t))| (k.clone(), *t))
                    .collect();
                entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                
                // Remove oldest 20%
                let remove_count = entries.len() / 5;
                for i in 0..remove_count {
                    self.route_cache.remove(&entries[i].0);
                }
            }
        }
    }
    
    /// Update performance metrics
    pub fn update_metrics(&mut self, lsa_db_size: usize, memory_usage: usize) {
        self.metrics.lsa_database_size = lsa_db_size;
        self.metrics.memory_usage_bytes = memory_usage;
        
        // Calculate hit rates
        let total_pool_accesses = self.packet_pool_hits + self.packet_pool_misses;
        if total_pool_accesses > 0 {
            self.metrics.packet_pool_hit_rate = 
                (self.packet_pool_hits as f64 / total_pool_accesses as f64) * 100.0;
        }
        
        let total_cache_accesses = self.route_cache_hits + self.route_cache_misses;
        if total_cache_accesses > 0 {
            self.metrics.route_cache_hit_rate = 
                (self.route_cache_hits as f64 / total_cache_accesses as f64) * 100.0;
        }
    }
    
    /// Get current performance metrics
    pub fn get_metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }
    
    /// Record packet pool hit
    pub fn record_packet_pool_hit(&mut self) {
        self.packet_pool_hits += 1;
    }
    
    /// Record packet pool miss
    pub fn record_packet_pool_miss(&mut self) {
        self.packet_pool_misses += 1;
    }
    
    /// Record dropped packet
    pub fn record_dropped_packet(&mut self) {
        self.metrics.dropped_packets += 1;
    }
    
    /// Clear all metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = PerformanceMetrics::default();
        self.packet_processing_samples.clear();
        self.spf_calculation_samples.clear();
        self.packet_pool_hits = 0;
        self.packet_pool_misses = 0;
        self.route_cache_hits = 0;
        self.route_cache_misses = 0;
    }
    
    /// Get performance recommendations based on metrics
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Check packet processing time
        if self.metrics.avg_packet_processing_us > 1000.0 {
            recommendations.push("High packet processing time detected. Consider enabling packet aggregation.".to_string());
        }
        
        // Check SPF calculation time
        if self.metrics.avg_spf_time_ms > 100.0 {
            recommendations.push("High SPF calculation time. Consider increasing SPF throttle time.".to_string());
        }
        
        // Check cache hit rates
        if self.metrics.route_cache_hit_rate < 50.0 && self.current_profile.route_caching {
            recommendations.push("Low route cache hit rate. Consider increasing cache TTL.".to_string());
        }
        
        // Check dropped packets
        if self.metrics.dropped_packets > 0 {
            recommendations.push(format!("{} packets dropped. Consider increasing max concurrent events.", 
                self.metrics.dropped_packets));
        }
        
        // Check memory usage
        if self.metrics.memory_usage_bytes > 1_000_000_000 { // 1GB
            recommendations.push("High memory usage detected. Consider enabling lazy LSA aging.".to_string());
        }
        
        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_performance_profiles() {
        let small = PerformanceProfiles::small_network();
        assert_eq!(small.name, "small_network");
        assert_eq!(small.max_concurrent_events, 50);
        
        let large = PerformanceProfiles::large_network();
        assert_eq!(large.name, "large_network");
        assert!(large.parallel_processing);
    }
    
    #[test]
    fn test_auto_tuning() {
        let mut tuner = PerformanceTuner::new();
        
        tuner.auto_tune(30);
        assert_eq!(tuner.get_profile().name, "small_network");
        
        tuner.auto_tune(100);
        assert_eq!(tuner.get_profile().name, "medium_network");
        
        tuner.auto_tune(300);
        assert_eq!(tuner.get_profile().name, "large_network");
    }
    
    #[test]
    fn test_metrics_recording() {
        let mut tuner = PerformanceTuner::new();
        
        // Record packet processing times
        tuner.record_packet_processing(100.0);
        tuner.record_packet_processing(200.0);
        tuner.record_packet_processing(150.0);
        
        assert_eq!(tuner.get_metrics().packets_processed, 3);
        assert_eq!(tuner.get_metrics().avg_packet_processing_us, 150.0);
        assert_eq!(tuner.get_metrics().peak_packet_processing_us, 200.0);
    }
    
    #[test]
    fn test_spf_throttling() {
        let tuner = PerformanceTuner::new();
        
        // Default throttle is 100ms
        assert!(tuner.should_throttle_spf(0.0, 0.05)); // 50ms elapsed
        assert!(!tuner.should_throttle_spf(0.0, 0.15)); // 150ms elapsed
    }
    
    #[test]
    fn test_route_caching() {
        let mut tuner = PerformanceTuner::new();
        let routes = vec![]; // Empty for test
        
        // Cache a route
        tuner.cache_route("test_key".to_string(), routes.clone(), 0.0);
        
        // Should hit cache within TTL
        assert!(tuner.get_cached_route("test_key", 100.0).is_some());
        assert_eq!(tuner.route_cache_hits, 1);
        
        // Should miss cache after TTL
        assert!(tuner.get_cached_route("test_key", 400.0).is_none());
        assert_eq!(tuner.route_cache_misses, 1);
    }
}