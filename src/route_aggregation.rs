use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;
use serde::{Serialize, Deserialize};
use crate::router::{LSA, LSAHeader, LSAType, LSAData, SummaryLSA, ASExternalLSA};
use crate::console_log;

/// Route Aggregation Support for OSPFv2
/// 
/// Implements route aggregation (summarization) for ABRs and ASBRs according to OSPFv2 standards.
/// This reduces the number of LSAs in the OSPF database by combining multiple routes
/// into single summary routes.
/// 
/// Key features:
/// - Inter-area route aggregation (Type 3 Summary LSA)
/// - External route aggregation (Type 5 AS-External LSA)
/// - Automatic metric calculation for aggregated routes
/// - Route suppression to prevent detailed route advertisement

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateRoute {
    /// Network prefix for the aggregate route
    pub network: String,
    /// Network mask for the aggregate route
    pub mask: String,
    /// Whether to suppress more specific routes
    pub suppress: bool,
    /// Metric for the aggregate route (if specified, otherwise calculated)
    pub metric: Option<u32>,
    /// Area ID for inter-area aggregation (None for external routes)
    pub area_id: Option<String>,
    /// Whether this aggregate is currently active
    pub active: bool,
    /// Contributing routes that match this aggregate
    pub contributing_routes: HashSet<String>,
}

impl AggregateRoute {
    pub fn new(network: String, mask: String, suppress: bool) -> Self {
        AggregateRoute {
            network,
            mask,
            suppress,
            metric: None,
            area_id: None,
            active: false,
            contributing_routes: HashSet::new(),
        }
    }
    
    pub fn inter_area(network: String, mask: String, area_id: String, suppress: bool) -> Self {
        AggregateRoute {
            network,
            mask,
            suppress,
            metric: None,
            area_id: Some(area_id),
            active: false,
            contributing_routes: HashSet::new(),
        }
    }
    
    pub fn with_metric(mut self, metric: u32) -> Self {
        self.metric = Some(metric);
        self
    }
    
    /// Check if a route matches this aggregate
    pub fn matches_route(&self, route_network: &str, route_mask: &str) -> bool {
        if let (Ok(agg_net), Ok(agg_mask), Ok(route_net), Ok(route_netmask)) = (
            self.network.parse::<Ipv4Addr>(),
            self.mask.parse::<Ipv4Addr>(),
            route_network.parse::<Ipv4Addr>(),
            route_mask.parse::<Ipv4Addr>(),
        ) {
            // Check if route is within aggregate network
            let agg_net_u32 = u32::from(agg_net);
            let agg_mask_u32 = u32::from(agg_mask);
            let route_net_u32 = u32::from(route_net);
            let route_mask_u32 = u32::from(route_netmask);
            
            // Route must be more specific (longer prefix) than aggregate
            if route_mask_u32 <= agg_mask_u32 {
                return false;
            }
            
            // Route network must be within aggregate range
            (route_net_u32 & agg_mask_u32) == (agg_net_u32 & agg_mask_u32)
        } else {
            false
        }
    }
    
    /// Calculate the metric for this aggregate based on contributing routes
    pub fn calculate_metric(&self, route_metrics: &HashMap<String, u32>) -> u32 {
        if let Some(fixed_metric) = self.metric {
            return fixed_metric;
        }
        
        // Use the minimum metric of contributing routes
        self.contributing_routes
            .iter()
            .filter_map(|route| route_metrics.get(route))
            .min()
            .copied()
            .unwrap_or(0)
    }
}

/// Route Aggregation Manager
/// 
/// Manages route aggregation configuration and generation of aggregate LSAs
pub struct RouteAggregationManager {
    /// Router ID
    router_id: String,
    /// Configured aggregate routes
    aggregates: HashMap<String, AggregateRoute>,
    /// Current routing table entries for metric calculation
    route_metrics: HashMap<String, u32>,
    /// Whether this router is an ABR
    is_abr: bool,
    /// Whether this router is an ASBR
    is_asbr: bool,
}

impl RouteAggregationManager {
    pub fn new(router_id: String) -> Self {
        RouteAggregationManager {
            router_id,
            aggregates: HashMap::new(),
            route_metrics: HashMap::new(),
            is_abr: false,
            is_asbr: false,
        }
    }
    
    /// Configure a new aggregate route
    pub fn configure_aggregate(
        &mut self,
        network: String,
        mask: String,
        area_id: Option<String>,
        suppress: bool,
        metric: Option<u32>,
    ) -> Result<(), String> {
        // Validate network and mask
        if network.parse::<Ipv4Addr>().is_err() {
            return Err(format!("Invalid network address: {}", network));
        }
        if mask.parse::<Ipv4Addr>().is_err() {
            return Err(format!("Invalid network mask: {}", mask));
        }
        
        let aggregate_key = format!("{}:{}", network, mask);
        
        // Check for existing aggregate
        if self.aggregates.contains_key(&aggregate_key) {
            return Err(format!("Aggregate route {} already configured", aggregate_key));
        }
        
        let mut aggregate = if let Some(area) = area_id {
            // Inter-area aggregate (ABR function)
            if !self.is_abr {
                return Err("Router must be ABR to configure inter-area aggregates".to_string());
            }
            AggregateRoute::inter_area(network.clone(), mask.clone(), area, suppress)
        } else {
            // External aggregate (ASBR function)
            if !self.is_asbr {
                return Err("Router must be ASBR to configure external aggregates".to_string());
            }
            AggregateRoute::new(network.clone(), mask.clone(), suppress)
        };
        
        if let Some(m) = metric {
            aggregate = aggregate.with_metric(m);
        }
        
        self.aggregates.insert(aggregate_key.clone(), aggregate);
        
        console_log!(
            "Router {} configured aggregate route {}/{} (suppress: {})",
            self.router_id, network, mask, suppress
        );
        
        Ok(())
    }
    
    /// Remove an aggregate route
    pub fn remove_aggregate(&mut self, network: &str, mask: &str) -> bool {
        let aggregate_key = format!("{}:{}", network, mask);
        if self.aggregates.remove(&aggregate_key).is_some() {
            console_log!(
                "Router {} removed aggregate route {}/{}",
                self.router_id, network, mask
            );
            true
        } else {
            false
        }
    }
    
    /// Update ABR/ASBR status
    pub fn update_router_type(&mut self, is_abr: bool, is_asbr: bool) {
        self.is_abr = is_abr;
        self.is_asbr = is_asbr;
        
        // Deactivate aggregates that are no longer valid
        for aggregate in self.aggregates.values_mut() {
            if aggregate.area_id.is_some() && !is_abr {
                aggregate.active = false;
            } else if aggregate.area_id.is_none() && !is_asbr {
                aggregate.active = false;
            }
        }
    }
    
    /// Update route information for metric calculation
    pub fn update_route_metrics(&mut self, routes: HashMap<String, u32>) {
        self.route_metrics = routes;
        self.update_aggregate_status();
    }
    
    /// Update the status of all aggregates based on current routes
    fn update_aggregate_status(&mut self) {
        for aggregate in self.aggregates.values_mut() {
            aggregate.contributing_routes.clear();
            
            // Find routes that match this aggregate
            for route_key in self.route_metrics.keys() {
                if let Some((network, mask)) = route_key.split_once('/') {
                    if aggregate.matches_route(network, mask) {
                        aggregate.contributing_routes.insert(route_key.clone());
                    }
                }
            }
            
            // Aggregate is active if it has contributing routes
            let was_active = aggregate.active;
            aggregate.active = !aggregate.contributing_routes.is_empty();
            
            if aggregate.active != was_active {
                console_log!(
                    "Router {} aggregate {}/{} became {}",
                    self.router_id, aggregate.network, aggregate.mask,
                    if aggregate.active { "active" } else { "inactive" }
                );
            }
        }
    }
    
    /// Generate Summary LSAs for active inter-area aggregates
    pub fn generate_summary_lsas(&self) -> Vec<LSA> {
        let mut lsas = Vec::new();
        
        for aggregate in self.aggregates.values() {
            if aggregate.active && aggregate.area_id.is_some() {
                let metric = aggregate.calculate_metric(&self.route_metrics);
                
                let header = LSAHeader {
                    ls_age: 0,
                    ls_type: LSAType::SummaryLSA,
                    link_state_id: aggregate.network.clone(),
                    advertising_router: self.router_id.clone(),
                    ls_sequence_number: 0x80000001,
                    ls_checksum: 0, // Will be calculated
                    length: 28, // Summary LSA header + body
                };
                
                let data = LSAData::Summary(SummaryLSA {
                    network_mask: aggregate.mask.clone(),
                    metric,
                    tos: 0,
                    tos_metric: 0,
                });
                
                let mut lsa = LSA { header, data };
                
                // Calculate checksum
                lsa.header.ls_checksum = crate::ospf_checksum::calculate_lsa_checksum(&lsa);
                
                lsas.push(lsa);
                
                console_log!(
                    "Router {} generated Summary LSA for aggregate {}/{} with metric {}",
                    self.router_id, aggregate.network, aggregate.mask, metric
                );
            }
        }
        
        lsas
    }
    
    /// Generate AS-External LSAs for active external aggregates
    pub fn generate_external_lsas(&self) -> Vec<LSA> {
        let mut lsas = Vec::new();
        
        for aggregate in self.aggregates.values() {
            if aggregate.active && aggregate.area_id.is_none() {
                let metric = aggregate.calculate_metric(&self.route_metrics);
                
                let header = LSAHeader {
                    ls_age: 0,
                    ls_type: LSAType::ASExternalLSA,
                    link_state_id: aggregate.network.clone(),
                    advertising_router: self.router_id.clone(),
                    ls_sequence_number: 0x80000001,
                    ls_checksum: 0, // Will be calculated
                    length: 36, // AS-External LSA header + body
                };
                
                let data = LSAData::ASExternal(ASExternalLSA {
                    network_mask: aggregate.mask.clone(),
                    metric,
                    metric_type: 1, // Type 1 metric
                    forwarding_address: "0.0.0.0".to_string(),
                    external_route_tag: 0,
                    tos: 0,
                    tos_metric: 0,
                });
                
                let mut lsa = LSA { header, data };
                
                // Calculate checksum
                lsa.header.ls_checksum = crate::ospf_checksum::calculate_lsa_checksum(&lsa);
                
                lsas.push(lsa);
                
                console_log!(
                    "Router {} generated AS-External LSA for aggregate {}/{} with metric {}",
                    self.router_id, aggregate.network, aggregate.mask, metric
                );
            }
        }
        
        lsas
    }
    
    /// Check if a route should be suppressed due to aggregation
    pub fn should_suppress_route(&self, network: &str, mask: &str) -> bool {
        for aggregate in self.aggregates.values() {
            if aggregate.active && aggregate.suppress && aggregate.matches_route(network, mask) {
                return true;
            }
        }
        false
    }
    
    /// Get all configured aggregates
    pub fn get_aggregates(&self) -> &HashMap<String, AggregateRoute> {
        &self.aggregates
    }
    
    /// Get active aggregates only
    pub fn get_active_aggregates(&self) -> Vec<&AggregateRoute> {
        self.aggregates
            .values()
            .filter(|agg| agg.active)
            .collect()
    }
    
    /// Get aggregation statistics
    pub fn get_statistics(&self) -> AggregationStatistics {
        let total_aggregates = self.aggregates.len();
        let active_aggregates = self.aggregates.values().filter(|agg| agg.active).count();
        let inter_area_aggregates = self.aggregates.values()
            .filter(|agg| agg.area_id.is_some())
            .count();
        let external_aggregates = self.aggregates.values()
            .filter(|agg| agg.area_id.is_none())
            .count();
        let suppressed_routes = self.route_metrics.keys()
            .filter(|route| {
                if let Some((network, mask)) = route.split_once('/') {
                    self.should_suppress_route(network, mask)
                } else {
                    false
                }
            })
            .count();
        
        AggregationStatistics {
            total_aggregates,
            active_aggregates,
            inter_area_aggregates,
            external_aggregates,
            suppressed_routes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationStatistics {
    pub total_aggregates: usize,
    pub active_aggregates: usize,
    pub inter_area_aggregates: usize,
    pub external_aggregates: usize,
    pub suppressed_routes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_aggregate_route_matching() {
        let aggregate = AggregateRoute::new(
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            true,
        );
        
        // Should match more specific routes
        assert!(aggregate.matches_route("192.168.1.0", "255.255.255.0"));
        assert!(aggregate.matches_route("192.168.100.0", "255.255.255.0"));
        
        // Should not match less specific or unrelated routes
        assert!(!aggregate.matches_route("192.168.0.0", "255.255.0.0")); // Same specificity
        assert!(!aggregate.matches_route("192.168.0.0", "255.0.0.0")); // Less specific
        assert!(!aggregate.matches_route("10.0.1.0", "255.255.255.0")); // Different network
    }
    
    #[test]
    fn test_route_aggregation_manager() {
        let mut manager = RouteAggregationManager::new("1.1.1.1".to_string());
        manager.update_router_type(true, true); // ABR and ASBR
        
        // Configure inter-area aggregate
        let result = manager.configure_aggregate(
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            true,
            None,
        );
        assert!(result.is_ok());
        
        // Configure external aggregate
        let result = manager.configure_aggregate(
            "10.0.0.0".to_string(),
            "255.0.0.0".to_string(),
            None,
            true,
            Some(100),
        );
        assert!(result.is_ok());
        
        // Add some routes
        let mut routes = HashMap::new();
        routes.insert("192.168.1.0/255.255.255.0".to_string(), 10);
        routes.insert("192.168.2.0/255.255.255.0".to_string(), 20);
        routes.insert("10.1.0.0/255.255.0.0".to_string(), 50);
        
        manager.update_route_metrics(routes);
        
        // Check aggregates became active
        let stats = manager.get_statistics();
        assert_eq!(stats.total_aggregates, 2);
        assert_eq!(stats.active_aggregates, 2);
        assert_eq!(stats.inter_area_aggregates, 1);
        assert_eq!(stats.external_aggregates, 1);
        
        // Test route suppression
        assert!(manager.should_suppress_route("192.168.1.0", "255.255.255.0"));
        assert!(manager.should_suppress_route("10.1.0.0", "255.255.0.0"));
        assert!(!manager.should_suppress_route("172.16.1.0", "255.255.255.0"));
    }
    
    #[test]
    fn test_metric_calculation() {
        let mut aggregate = AggregateRoute::new(
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            false,
        );
        
        // Add contributing routes
        aggregate.contributing_routes.insert("192.168.1.0/255.255.255.0".to_string());
        aggregate.contributing_routes.insert("192.168.2.0/255.255.255.0".to_string());
        
        let mut route_metrics = HashMap::new();
        route_metrics.insert("192.168.1.0/255.255.255.0".to_string(), 10);
        route_metrics.insert("192.168.2.0/255.255.255.0".to_string(), 5);
        
        // Should use minimum metric
        assert_eq!(aggregate.calculate_metric(&route_metrics), 5);
        
        // With fixed metric
        aggregate.metric = Some(100);
        assert_eq!(aggregate.calculate_metric(&route_metrics), 100);
    }
    
    #[test]
    fn test_abr_asbr_requirements() {
        let mut manager = RouteAggregationManager::new("1.1.1.1".to_string());
        
        // Should fail without ABR status
        let result = manager.configure_aggregate(
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            true,
            None,
        );
        assert!(result.is_err());
        
        // Should fail without ASBR status
        let result = manager.configure_aggregate(
            "10.0.0.0".to_string(),
            "255.0.0.0".to_string(),
            None,
            true,
            None,
        );
        assert!(result.is_err());
        
        // Enable ABR/ASBR and try again
        manager.update_router_type(true, true);
        
        let result = manager.configure_aggregate(
            "192.168.0.0".to_string(),
            "255.255.0.0".to_string(),
            Some("1.0.0.0".to_string()),
            true,
            None,
        );
        assert!(result.is_ok());
    }
}