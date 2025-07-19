#[cfg(test)]
mod tests {
    use crate::simulation::NetworkSimulation;
    use crate::performance_tuning::{PerformanceProfiles, PerformanceTuner};
    
    #[test]
    fn test_performance_profile_application() {
        let mut sim = NetworkSimulation::new();
        
        // Create a small network
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        let r3 = sim.add_router("R3".to_string(), 150.0, 200.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.connect_routers(r2, r3, 10).unwrap();
        sim.connect_routers(r3, r1, 10).unwrap();
        
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        sim.enable_ospf(r3).unwrap();
        
        // Apply small network profile
        assert!(sim.set_performance_profile("small_network").is_ok());
        
        // Apply medium network profile
        assert!(sim.set_performance_profile("medium_network").is_ok());
        
        // Apply unknown profile should fail
        assert!(sim.set_performance_profile("unknown_profile").is_err());
    }
    
    #[test]
    fn test_auto_tuning() {
        let mut sim = NetworkSimulation::new();
        
        // Create routers to test auto-tuning
        for i in 0..25 {
            sim.add_router(format!("R{}", i), (i as f64) * 50.0, 100.0);
        }
        
        // Auto-tune should select small_network profile
        sim.auto_tune_performance();
        
        // Add more routers
        for i in 25..100 {
            sim.add_router(format!("R{}", i), (i as f64) * 50.0, 200.0);
        }
        
        // Auto-tune should now select medium_network profile
        sim.auto_tune_performance();
    }
    
    #[test]
    fn test_performance_metrics_collection() {
        let mut sim = NetworkSimulation::new();
        
        // Create a small network
        let r1 = sim.add_router("R1".to_string(), 100.0, 100.0);
        let r2 = sim.add_router("R2".to_string(), 200.0, 100.0);
        
        sim.connect_routers(r1, r2, 10).unwrap();
        sim.enable_ospf(r1).unwrap();
        sim.enable_ospf(r2).unwrap();
        
        // Reset metrics
        sim.reset_performance_metrics();
        
        // Run simulation
        sim.start_simulation();
        for _ in 0..10 {
            sim.step_simulation(0.1);
        }
        
        // Get metrics
        let metrics_json = sim.get_performance_metrics();
        assert!(metrics_json.contains("aggregate"));
        assert!(metrics_json.contains("router_count"));
        
        // Verify JSON is valid
        assert!(serde_json::from_str::<serde_json::Value>(&metrics_json).is_ok());
    }
    
    #[test]
    fn test_performance_recommendations() {
        let mut sim = NetworkSimulation::new();
        
        // Create a large network
        for i in 0..250 {
            sim.add_router(format!("R{}", i), (i as f64) * 50.0, 100.0);
        }
        
        let recommendations = sim.get_performance_recommendations();
        
        // Should recommend large_network profile
        assert!(recommendations.iter().any(|r| r.contains("Large network detected")));
    }
    
    #[test]
    fn test_performance_tuner_metrics() {
        let mut tuner = PerformanceTuner::new();
        
        // Record some packet processing times
        tuner.record_packet_processing(50.0);
        tuner.record_packet_processing(100.0);
        tuner.record_packet_processing(75.0);
        tuner.record_packet_processing(200.0); // Peak
        
        let metrics = tuner.get_metrics();
        assert_eq!(metrics.packets_processed, 4);
        assert_eq!(metrics.avg_packet_processing_us, 106.25); // (50+100+75+200)/4
        assert_eq!(metrics.peak_packet_processing_us, 200.0);
        
        // Record SPF calculations
        tuner.record_spf_calculation(10.0);
        tuner.record_spf_calculation(15.0);
        tuner.record_spf_calculation(12.5);
        
        let metrics = tuner.get_metrics();
        assert_eq!(metrics.spf_calculations, 3);
        assert_eq!(metrics.avg_spf_time_ms, 12.5); // (10+15+12.5)/3
        assert_eq!(metrics.peak_spf_time_ms, 15.0);
    }
    
    #[test]
    fn test_performance_profiles_values() {
        // Test profile configurations
        let small = PerformanceProfiles::small_network();
        assert_eq!(small.max_concurrent_events, 50);
        assert!(!small.packet_aggregation);
        assert!(!small.parallel_processing);
        
        let medium = PerformanceProfiles::medium_network();
        assert_eq!(medium.max_concurrent_events, 100);
        assert!(medium.packet_aggregation);
        assert!(!medium.parallel_processing);
        
        let large = PerformanceProfiles::large_network();
        assert_eq!(large.max_concurrent_events, 200);
        assert!(large.packet_aggregation);
        assert!(large.parallel_processing);
        
        let real_time = PerformanceProfiles::real_time();
        assert_eq!(real_time.spf_throttle_ms, 10);
        assert!(!real_time.lazy_lsa_aging);
        assert!(real_time.parallel_processing);
    }
    
    #[test]
    fn test_route_cache_functionality() {
        let mut tuner = PerformanceTuner::new();
        
        // Enable route caching
        let mut profile = PerformanceProfiles::medium_network();
        profile.route_caching = true;
        profile.route_cache_ttl = 300; // 5 minutes
        tuner.set_profile(profile);
        
        // Test cache miss
        assert!(tuner.get_cached_route("test_route", 0.0).is_none());
        
        // Cache a route
        let routes = vec![];
        tuner.cache_route("test_route".to_string(), routes.clone(), 0.0);
        
        // Test cache hit within TTL
        assert!(tuner.get_cached_route("test_route", 100.0).is_some());
        
        // Test cache miss after TTL expires
        assert!(tuner.get_cached_route("test_route", 400.0).is_none());
        
        // Check hit rate
        tuner.update_metrics(0, 0);
        let metrics = tuner.get_metrics();
        // We have 1 hit and 2 misses, so hit rate is 1/3 = 33.33%
        assert!((metrics.route_cache_hit_rate - 33.33).abs() < 0.1);
    }
}