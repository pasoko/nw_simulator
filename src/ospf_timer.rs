use std::collections::HashMap;
use crate::console_log;

/// OSPF Timer Events
#[derive(Debug, Clone, PartialEq)]
pub enum OSPFTimerEvent {
    HelloTimer,
    DeadTimer(u32),  // neighbor_id
    LSARefresh,
    RetransmissionTimer(u32), // neighbor_id
    DDRetransmissionTimer(u32), // neighbor_id - RFC 2328 Section 10.8
    SPFDelay,  // RFC 2328 Section 16.1 - Delay SPF calculation to prevent CPU overload
}

/// OSPF Timer Management
/// 
/// Manages all OSPF timers including:
/// - Hello timers
/// - Dead timers for neighbors
/// - LSA refresh timers
/// - Retransmission timers
/// - DD retransmission timers (RFC 2328 Section 10.8)
pub struct OSPFTimerManager {
    router_id: String,
    hello_interval: f64,
    dead_interval: f64,
    lsa_refresh_interval: f64,
    retransmission_interval: f64,
    dd_retransmission_interval: f64,  // RFC 2328: RxmtInterval for DD packets
    spf_delay_interval: f64,  // RFC 2328 Section 16.1 - SPF calculation delay
    
    next_hello_time: f64,
    next_lsa_refresh: f64,
    next_spf_time: Option<f64>,  // When SPF calculation should run
    neighbor_dead_times: HashMap<u32, f64>,
    neighbor_retransmission_times: HashMap<u32, f64>,
    neighbor_dd_retransmission_times: HashMap<u32, f64>,  // DD retransmission timers
    
    current_time: f64,
}

impl OSPFTimerManager {
    pub fn new(router_id: String) -> Self {
        OSPFTimerManager {
            router_id,
            hello_interval: 10.0,
            dead_interval: 40.0,
            lsa_refresh_interval: 1800.0, // 30 minutes
            retransmission_interval: 5.0,
            dd_retransmission_interval: 5.0,  // RFC 2328 default RxmtInterval
            spf_delay_interval: 5.0,  // RFC 2328 Section 16.1 - delay SPF for stability
            
            next_hello_time: 0.0,
            next_lsa_refresh: 1800.0,
            next_spf_time: None,
            neighbor_dead_times: HashMap::new(),
            neighbor_retransmission_times: HashMap::new(),
            neighbor_dd_retransmission_times: HashMap::new(),
            
            current_time: 0.0,
        }
    }
    
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }
    
    pub fn start_hello_timer(&mut self) {
        // Send first Hello immediately (with small delay to allow initialization)
        self.next_hello_time = self.current_time + 0.1;
        console_log!("Router {} started hello timer, first hello at {:.1}s", 
            self.router_id, self.next_hello_time);
    }
    
    pub fn is_hello_due(&self) -> bool {
        self.current_time >= self.next_hello_time
    }
    
    pub fn schedule_next_hello(&mut self) {
        self.next_hello_time = self.current_time + self.hello_interval;
    }
    
    pub fn start_neighbor_dead_timer(&mut self, neighbor_id: u32) {
        let dead_time = self.current_time + self.dead_interval;
        self.neighbor_dead_times.insert(neighbor_id, dead_time);
        console_log!("Router {} started dead timer for neighbor {}, expires at {:.1}s", 
            self.router_id, neighbor_id, dead_time);
    }
    
    pub fn reset_neighbor_dead_timer(&mut self, neighbor_id: u32) {
        let dead_time = self.current_time + self.dead_interval;
        self.neighbor_dead_times.insert(neighbor_id, dead_time);
    }
    
    pub fn remove_neighbor_dead_timer(&mut self, neighbor_id: u32) {
        self.neighbor_dead_times.remove(&neighbor_id);
        console_log!("Router {} removed dead timer for neighbor {}", 
            self.router_id, neighbor_id);
    }
    
    pub fn get_expired_dead_timers(&self) -> Vec<u32> {
        self.neighbor_dead_times.iter()
            .filter(|(_, &dead_time)| self.current_time >= dead_time)
            .map(|(&neighbor_id, _)| neighbor_id)
            .collect()
    }
    
    pub fn start_retransmission_timer(&mut self, neighbor_id: u32) {
        let retrans_time = self.current_time + self.retransmission_interval;
        self.neighbor_retransmission_times.insert(neighbor_id, retrans_time);
        console_log!("Router {} started retransmission timer for neighbor {}, expires at {:.1}s", 
            self.router_id, neighbor_id, retrans_time);
    }
    
    pub fn stop_retransmission_timer(&mut self, neighbor_id: u32) {
        self.neighbor_retransmission_times.remove(&neighbor_id);
    }
    
    pub fn get_expired_retransmission_timers(&self) -> Vec<u32> {
        self.neighbor_retransmission_times.iter()
            .filter(|(_, &retrans_time)| self.current_time >= retrans_time)
            .map(|(&neighbor_id, _)| neighbor_id)
            .collect()
    }
    
    pub fn start_dd_retransmission_timer(&mut self, neighbor_id: u32) {
        let dd_retrans_time = self.current_time + self.dd_retransmission_interval;
        self.neighbor_dd_retransmission_times.insert(neighbor_id, dd_retrans_time);
        console_log!("Router {} started DD retransmission timer for neighbor {}, expires at {:.1}s", 
            self.router_id, neighbor_id, dd_retrans_time);
    }
    
    pub fn stop_dd_retransmission_timer(&mut self, neighbor_id: u32) {
        self.neighbor_dd_retransmission_times.remove(&neighbor_id);
        console_log!("Router {} stopped DD retransmission timer for neighbor {}", 
            self.router_id, neighbor_id);
    }
    
    pub fn get_expired_dd_retransmission_timers(&self) -> Vec<u32> {
        self.neighbor_dd_retransmission_times.iter()
            .filter(|(_, &dd_retrans_time)| self.current_time >= dd_retrans_time)
            .map(|(&neighbor_id, _)| neighbor_id)
            .collect()
    }
    
    pub fn is_lsa_refresh_due(&self) -> bool {
        self.current_time >= self.next_lsa_refresh
    }
    
    pub fn start_spf_delay_timer(&mut self) {
        // Only start if not already scheduled
        if self.next_spf_time.is_none() {
            let spf_time = self.current_time + self.spf_delay_interval;
            self.next_spf_time = Some(spf_time);
            console_log!("Router {} scheduled SPF calculation for {:.1}s (delay: {:.1}s)", 
                self.router_id, spf_time, self.spf_delay_interval);
        } else {
            console_log!("Router {} SPF already scheduled, not rescheduling", self.router_id);
        }
    }
    
    pub fn cancel_spf_delay_timer(&mut self) {
        self.next_spf_time = None;
        console_log!("Router {} cancelled SPF delay timer", self.router_id);
    }
    
    pub fn is_spf_due(&self) -> bool {
        self.next_spf_time.map_or(false, |time| self.current_time >= time)
    }
    
    pub fn schedule_next_lsa_refresh(&mut self) {
        self.next_lsa_refresh = self.current_time + self.lsa_refresh_interval;
        console_log!("Router {} scheduled next LSA refresh at {:.1}s", 
            self.router_id, self.next_lsa_refresh);
    }
    
    pub fn get_next_timer_event(&self) -> Option<(f64, OSPFTimerEvent)> {
        let mut next_time = f64::MAX;
        let mut next_event = None;
        
        // Check hello timer
        if self.next_hello_time < next_time {
            next_time = self.next_hello_time;
            next_event = Some(OSPFTimerEvent::HelloTimer);
        }
        
        // Check LSA refresh timer
        if self.next_lsa_refresh < next_time {
            next_time = self.next_lsa_refresh;
            next_event = Some(OSPFTimerEvent::LSARefresh);
        }
        
        // Check dead timers
        for (&neighbor_id, &dead_time) in &self.neighbor_dead_times {
            if dead_time < next_time {
                next_time = dead_time;
                next_event = Some(OSPFTimerEvent::DeadTimer(neighbor_id));
            }
        }
        
        // Check retransmission timers
        for (&neighbor_id, &retrans_time) in &self.neighbor_retransmission_times {
            if retrans_time < next_time {
                next_time = retrans_time;
                next_event = Some(OSPFTimerEvent::RetransmissionTimer(neighbor_id));
            }
        }
        
        // Check DD retransmission timers
        for (&neighbor_id, &dd_retrans_time) in &self.neighbor_dd_retransmission_times {
            if dd_retrans_time < next_time {
                next_time = dd_retrans_time;
                next_event = Some(OSPFTimerEvent::DDRetransmissionTimer(neighbor_id));
            }
        }
        
        // Check SPF delay timer
        if let Some(spf_time) = self.next_spf_time {
            if spf_time < next_time {
                next_time = spf_time;
                next_event = Some(OSPFTimerEvent::SPFDelay);
            }
        }
        
        next_event.map(|event| (next_time, event))
    }
    
    pub fn process_expired_timers(&mut self) -> Vec<OSPFTimerEvent> {
        let mut expired_events = Vec::new();
        
        // Check hello timer
        if self.is_hello_due() {
            console_log!("Router {} hello timer due at {:.1}s (next at {:.1}s -> {:.1}s)", 
                self.router_id, self.current_time, self.next_hello_time, 
                self.current_time + self.hello_interval);
            expired_events.push(OSPFTimerEvent::HelloTimer);
            self.schedule_next_hello();
        }
        
        // Check LSA refresh timer
        if self.is_lsa_refresh_due() {
            expired_events.push(OSPFTimerEvent::LSARefresh);
            self.schedule_next_lsa_refresh();
        }
        
        // Check dead timers
        let expired_dead = self.get_expired_dead_timers();
        for neighbor_id in expired_dead {
            expired_events.push(OSPFTimerEvent::DeadTimer(neighbor_id));
            self.neighbor_dead_times.remove(&neighbor_id);
        }
        
        // Check retransmission timers
        let expired_retrans = self.get_expired_retransmission_timers();
        for neighbor_id in expired_retrans {
            expired_events.push(OSPFTimerEvent::RetransmissionTimer(neighbor_id));
            // Don't remove retransmission timer - it will be restarted
        }
        
        // Check DD retransmission timers
        let expired_dd_retrans = self.get_expired_dd_retransmission_timers();
        for neighbor_id in expired_dd_retrans {
            expired_events.push(OSPFTimerEvent::DDRetransmissionTimer(neighbor_id));
            // Don't remove DD retransmission timer - it will be restarted
        }
        
        // Check SPF delay timer
        if self.is_spf_due() {
            expired_events.push(OSPFTimerEvent::SPFDelay);
            self.next_spf_time = None;  // Clear the timer
        }
        
        expired_events
    }
    
    pub fn clear_all_neighbor_timers(&mut self, neighbor_id: u32) {
        self.neighbor_dead_times.remove(&neighbor_id);
        self.neighbor_retransmission_times.remove(&neighbor_id);
        self.neighbor_dd_retransmission_times.remove(&neighbor_id);
        console_log!("Router {} cleared all timers for neighbor {}", 
            self.router_id, neighbor_id);
    }
    
    pub fn get_timer_status(&self) -> String {
        format!("Hello: {:.1}s, LSA Refresh: {:.1}s, Dead Timers: {}, Retrans Timers: {}",
            self.next_hello_time - self.current_time,
            self.next_lsa_refresh - self.current_time,
            self.neighbor_dead_times.len(),
            self.neighbor_retransmission_times.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_timer() {
        let mut timer = OSPFTimerManager::new("1.1.1.1".to_string());
        timer.update_time(0.0);
        
        // Start hello timer
        timer.start_hello_timer();
        assert!(!timer.is_hello_due());
        
        // Advance time
        timer.update_time(10.0);
        assert!(timer.is_hello_due());
    }
    
    #[test]
    fn test_dead_timer() {
        let mut timer = OSPFTimerManager::new("1.1.1.1".to_string());
        timer.update_time(0.0);
        
        // Start dead timer for neighbor
        timer.start_neighbor_dead_timer(2);
        assert!(timer.get_expired_dead_timers().is_empty());
        
        // Advance time past dead interval
        timer.update_time(50.0);
        let expired = timer.get_expired_dead_timers();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], 2);
    }
}