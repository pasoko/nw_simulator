use serde::{Serialize, Deserialize};
use crate::ospf::OSPFPacket;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolPacket {
    OSPF(OSPFPacket),
    // Future protocols can be added here:
    // BGP(BGPPacket),
    // RIP(RIPPacket),
}

pub trait RoutingProtocol: Send {
    fn process_packet(&mut self, packet: ProtocolPacket, from_router_id: u32) -> Vec<PacketEvent>;
    fn generate_packets(&mut self, current_time: f64) -> Vec<PacketEvent>;
    fn get_protocol_name(&self) -> &str;
    fn start(&mut self);
    fn stop(&mut self);
    fn update_time(&mut self, time: f64);
    fn get_router_id(&self) -> u32;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketEvent {
    pub timestamp: f64,
    pub from_router_id: u32,
    pub to_router_id: u32,
    pub packet: ProtocolPacket,
}

#[derive(Debug)]
pub struct ProtocolEngine {
    pub events: Vec<PacketEvent>,
    pub current_time: f64,
}

impl ProtocolEngine {
    pub fn new() -> Self {
        ProtocolEngine {
            events: Vec::new(),
            current_time: 0.0,
        }
    }

    pub fn schedule_event(&mut self, event: PacketEvent) {
        let insert_pos = self.events
            .binary_search_by(|e| e.timestamp.partial_cmp(&event.timestamp).unwrap())
            .unwrap_or_else(|pos| pos);
        self.events.insert(insert_pos, event);
    }

    pub fn process_next_event(&mut self) -> Option<PacketEvent> {
        if self.events.is_empty() {
            return None;
        }
        let event = self.events.remove(0);
        self.current_time = event.timestamp;
        Some(event)
    }

}