use std::collections::{HashMap, BTreeMap};
use crate::network::NetworkTopology;
use crate::router::{RoutingTableEntry, RoutingProtocol, LSAData};
use crate::spf::SPFCalculator;
use crate::event_manager::EventManager;
use crate::ospf_engine::OSPFEngine;
use crate::console_log;

/// Route Calculation Management
/// 
/// Handles all routing calculations including:
/// - SPF algorithm execution
/// - Routing table updates
/// - Route change detection and logging
/// - Performance optimization for route calculations
pub struct RouteCalculator {
    current_time: f64,
    // Cache for recently calculated routes to avoid redundant calculations
    route_cache: HashMap<u32, (f64, Vec<RoutingTableEntry>)>, // router_id -> (timestamp, routes)
    cache_ttl: f64, // Time-to-live for cached routes
}

impl RouteCalculator {
    pub fn new() -> Self {
        RouteCalculator {
            current_time: 0.0,
            route_cache: HashMap::new(),
            cache_ttl: 1.0, // Cache routes for 1 second
        }
    }
    
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
        self.cleanup_expired_cache();
    }
    
    /// Calculate routes for a specific router using OSPF LSA database
    pub fn calculate_routes_for_router(
        &mut self,
        router_id: u32,
        topology: &mut NetworkTopology,
        ospf_engines: &BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) {
        console_log!("=== CALCULATING ROUTES FOR ROUTER {} ===", router_id);
        
        event_manager.log_routing_table_updated(router_id, 
            format!("Router {} starting route calculation", router_id));
        
        // Check cache first
        if let Some((cached_time, cached_routes)) = self.route_cache.get(&router_id) {
            if self.current_time - cached_time < self.cache_ttl {
                console_log!("Router {} using cached routes (age: {:.2}s)", 
                    router_id, self.current_time - cached_time);
                self.apply_routes_to_router(router_id, cached_routes.clone(), topology, event_manager);
                return;
            }
        }
        
        // Get LSA database from OSPF engine and calculate routes
        let (routes, lsa_count) = if let Some(engine) = ospf_engines.get(&router_id) {
            let lsa_count = engine.get_lsa_count();
            console_log!("Router {} has {} LSAs in database", router_id, lsa_count);
            
            if lsa_count > 0 {
                self.log_lsa_database_debug(router_id, engine);
                
                let routes = SPFCalculator::calculate_routes_from_lsa(
                    engine.get_lsa_database(),
                    router_id,
                    topology
                );
                
                console_log!("Router {} SPF returned {} routes", router_id, routes.len());
                self.log_calculated_routes_debug(router_id, &routes);
                
                // Convert HashMap to Vec for easier handling
                let route_vec: Vec<RoutingTableEntry> = routes.into_iter()
                    .map(|(_, route)| route)
                    .collect();
                
                (route_vec, Some(lsa_count))
            } else {
                console_log!("Router {} has no LSAs, no routes available (OSPFv2 compliance: waiting for protocol convergence)", router_id);
                (Vec::new(), Some(0))
            }
        } else {
            console_log!("Router {} has no OSPF engine, no routes calculated (OSPFv2 compliance)", router_id);
            // OSPFv2 compliance: No fallback routing. Routes only from OSPF protocol convergence.
            (Vec::new(), None)
        };
        
        // Cache the calculated routes
        self.route_cache.insert(router_id, (self.current_time, routes.clone()));
        
        // Log LSA count if OSPF is enabled
        if let Some(count) = lsa_count {
            event_manager.log_routing_table_updated(router_id, 
                format!("Router {} has {} LSAs in database", router_id, count));
        }
        
        // Apply routes to router
        self.apply_routes_to_router(router_id, routes, topology, event_manager);
    }
    
    fn apply_routes_to_router(
        &self,
        router_id: u32,
        routes: Vec<RoutingTableEntry>,
        topology: &mut NetworkTopology,
        event_manager: &mut EventManager,
    ) {
        if let Some(router) = topology.routers.get_mut(&router_id) {
            // Store old routing table for comparison
            let old_routes = router.routing_table.clone();
            console_log!("Router {} old routing table has {} entries", router_id, old_routes.len());
            
            // Clear old OSPF routes and update with new ones
            console_log!("Router {} clearing old OSPF routes", router_id);
            router.routing_table.retain(|r| r.protocol != RoutingProtocol::OSPF);
            
            // Update routing table
            console_log!("Router {} updating routing table with {} new routes", router_id, routes.len());
            for route in &routes {
                console_log!("  Adding route: {} -> {} via {} (metric {})", 
                    route.destination, route.next_hop, route.interface_id, route.metric);
                router.update_routing_table(route.clone());
            }
            console_log!("Router {} new routing table has {} entries", router_id, router.routing_table.len());
            
            // Build detailed description of routing table changes
            let change_description = self.analyze_routing_changes(router_id, &old_routes, &routes);
            
            // Log routing table update with details
            event_manager.log_routing_table_updated(router_id, change_description);
        }
    }
    
    fn analyze_routing_changes(
        &self,
        router_id: u32,
        old_routes: &[RoutingTableEntry],
        new_routes: &[RoutingTableEntry],
    ) -> String {
        let mut route_details = Vec::new();
        
        // Check for new or updated routes
        for new_route in new_routes {
            let is_new = !old_routes.iter().any(|r| 
                r.destination == new_route.destination && r.netmask == new_route.netmask
            );
            
            if is_new {
                route_details.push(format!("  + Added: {}/{} via {} metric {}", 
                    new_route.destination, new_route.netmask, 
                    new_route.next_hop, new_route.metric));
            } else {
                // Check if route changed
                if let Some(old_route) = old_routes.iter().find(|r| 
                    r.destination == new_route.destination && r.netmask == new_route.netmask
                ) {
                    if old_route.next_hop != new_route.next_hop || old_route.metric != new_route.metric {
                        route_details.push(format!("  ≈ Updated: {}/{} via {} metric {} (was: via {} metric {})", 
                            new_route.destination, new_route.netmask, 
                            new_route.next_hop, new_route.metric,
                            old_route.next_hop, old_route.metric));
                    }
                }
            }
        }
        
        // Check for removed routes
        for old_route in old_routes {
            let still_exists = new_routes.iter().any(|r| 
                r.destination == old_route.destination && r.netmask == old_route.netmask
            );
            
            if !still_exists {
                route_details.push(format!("  - Removed: {}/{} via {} metric {}", 
                    old_route.destination, old_route.netmask, 
                    old_route.next_hop, old_route.metric));
            }
        }
        
        // Build final description
        if route_details.is_empty() {
            format!("Router {} routing table recalculated (no changes)", router_id)
        } else {
            format!("Router {} routing table updated:\n{}", 
                router_id, route_details.join("\n"))
        }
    }
    
    fn log_lsa_database_debug(&self, router_id: u32, engine: &OSPFEngine) {
        console_log!("=== ROUTER {} SPF DEBUG START ===", router_id);
        console_log!("Router {} LSA database keys:", router_id);
        for (key, lsa) in engine.get_lsa_database() {
            console_log!("  Key: {} - Type: {:?} - Adv Router: {}", 
                key, lsa.header.ls_type, lsa.header.advertising_router);
            if let LSAData::Router(ref rlsa) = lsa.data {
                console_log!("    Router LSA with {} links:", rlsa.links.len());
                for (i, link) in rlsa.links.iter().enumerate() {
                    console_log!("      Link {}: ID={}, Type={:?}, Metric={}", 
                        i, link.link_id, link.link_type, link.metric);
                }
            }
        }
        console_log!("=== ROUTER {} SPF DEBUG END ===", router_id);
    }
    
    fn log_calculated_routes_debug(&self, router_id: u32, routes: &HashMap<u32, RoutingTableEntry>) {
        if routes.is_empty() {
            console_log!("  WARNING: No routes calculated for router {}", router_id);
        } else {
            console_log!("Router {} calculated routes:", router_id);
            for (dest_id, route) in routes {
                console_log!("  Route to {}: {} -> {} (metric {}) interface {}", 
                    dest_id, route.destination, route.next_hop, route.metric, route.interface_id);
            }
        }
    }
    
    /// Calculate routes for multiple routers in batch for efficiency
    pub fn calculate_routes_batch(
        &mut self,
        router_ids: Vec<u32>,
        topology: &mut NetworkTopology,
        ospf_engines: &BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) {
        console_log!("Batch route calculation for {} routers: {:?}", 
            router_ids.len(), router_ids);
        
        for router_id in router_ids {
            self.calculate_routes_for_router(router_id, topology, ospf_engines, event_manager);
        }
    }
    
    /// Force recalculation for all OSPF-enabled routers
    pub fn recalculate_all_routes(
        &mut self,
        topology: &mut NetworkTopology,
        ospf_engines: &BTreeMap<u32, OSPFEngine>,
        event_manager: &mut EventManager,
    ) {
        // Clear cache to force recalculation
        self.route_cache.clear();
        
        let ospf_router_ids: Vec<u32> = ospf_engines.keys().cloned().collect();
        console_log!("Recalculating routes for {} OSPF routers", ospf_router_ids.len());
        
        self.calculate_routes_batch(ospf_router_ids, topology, ospf_engines, event_manager);
    }
    
    /// Get routing table summary for a router
    pub fn get_routing_summary(&self, router_id: u32, topology: &NetworkTopology) -> RoutingSummary {
        if let Some(router) = topology.routers.get(&router_id) {
            let mut summary = RoutingSummary {
                router_id,
                total_routes: router.routing_table.len(),
                direct_routes: 0,
                ospf_routes: 0,
                static_routes: 0,
                unreachable_destinations: Vec::new(),
            };
            
            for route in &router.routing_table {
                match route.protocol {
                    RoutingProtocol::Direct => summary.direct_routes += 1,
                    RoutingProtocol::OSPF => summary.ospf_routes += 1,
                    RoutingProtocol::Static => summary.static_routes += 1,
                }
            }
            
            // Check for unreachable destinations (simplified)
            for other_router_id in topology.routers.keys() {
                if *other_router_id != router_id {
                    let destination = format!("1.1.1.{}", other_router_id);
                    let has_route = router.routing_table.iter()
                        .any(|r| r.destination == destination);
                    
                    if !has_route {
                        summary.unreachable_destinations.push(*other_router_id);
                    }
                }
            }
            
            summary
        } else {
            RoutingSummary::default()
        }
    }
    
    fn cleanup_expired_cache(&mut self) {
        let current_time = self.current_time;
        let ttl = self.cache_ttl;
        
        self.route_cache.retain(|_, (timestamp, _)| {
            current_time - *timestamp < ttl
        });
    }
    
    /// Clear all cached routes
    pub fn clear_cache(&mut self) {
        self.route_cache.clear();
        console_log!("Route cache cleared");
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStatistics {
        CacheStatistics {
            cached_routers: self.route_cache.len(),
            total_cached_routes: self.route_cache.values()
                .map(|(_, routes)| routes.len())
                .sum(),
            oldest_cache_age: self.route_cache.values()
                .map(|(timestamp, _)| self.current_time - timestamp)
                .fold(0.0, f64::max),
        }
    }
}

#[derive(Debug, Default)]
pub struct RoutingSummary {
    pub router_id: u32,
    pub total_routes: usize,
    pub direct_routes: usize,
    pub ospf_routes: usize,
    pub static_routes: usize,
    pub unreachable_destinations: Vec<u32>,
}

#[derive(Debug)]
pub struct CacheStatistics {
    pub cached_routers: usize,
    pub total_cached_routes: usize,
    pub oldest_cache_age: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::NetworkTopology;

    #[test]
    fn test_route_calculator_creation() {
        let calc = RouteCalculator::new();
        assert_eq!(calc.current_time, 0.0);
        assert_eq!(calc.route_cache.len(), 0);
    }
    
    #[test]
    fn test_routing_summary() {
        let calc = RouteCalculator::new();
        let mut topology = NetworkTopology::new();
        
        let router_id = topology.add_router("TestRouter".to_string());
        let summary = calc.get_routing_summary(router_id, &topology);
        
        assert_eq!(summary.router_id, router_id);
        assert_eq!(summary.total_routes, 0);
    }
    
    #[test]
    fn test_cache_management() {
        let mut calc = RouteCalculator::new();
        
        // Add some test routes to cache
        let test_routes = vec![
            RoutingTableEntry {
                destination: "1.1.1.2".to_string(),
                netmask: "255.255.255.255".to_string(),
                next_hop: "1.1.1.2".to_string(),
                interface_id: 1,
                metric: 10,
                protocol: RoutingProtocol::OSPF,
            }
        ];
        
        calc.route_cache.insert(1, (0.0, test_routes));
        assert_eq!(calc.route_cache.len(), 1);
        
        // Advance time and cleanup
        calc.update_time(2.0); // Beyond TTL
        assert_eq!(calc.route_cache.len(), 0);
    }
}