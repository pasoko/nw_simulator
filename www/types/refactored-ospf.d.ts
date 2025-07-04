/**
 * TypeScript definitions for the refactored OSPF WebAssembly interface
 */

export interface OSPFConfig {
  router_id: string;
  area_id: string;
  hello_interval: number;
  dead_interval: number;
  use_refactored_engine: boolean;
}

export interface SimpleEvent {
  event_type: string;
  timestamp: number;
  details: any;
}

export interface FeatureFlags {
  hello: boolean;
  dd: boolean;
  lsr: boolean;
  lsu: boolean;
  lsack: boolean;
}

export class RefactoredOSPFEngine {
  constructor(config_json: string);
  
  /**
   * Process an incoming OSPF packet
   * @param packet_type - OSPF packet type (1-5)
   * @param packet_data - JSON string of packet data
   * @param from_router - Router ID of sender
   * @param interface_id - Interface ID where packet was received
   * @returns JSON string of generated events
   */
  process_packet(
    packet_type: number,
    packet_data: string,
    from_router: number,
    interface_id: number
  ): string;
  
  /**
   * Generate a hello packet
   * @param interface_id - Interface ID to generate hello for
   * @returns JSON string of hello packet
   */
  generate_hello(interface_id: number): string;
  
  /**
   * Get pending events from the event bus
   * @returns JSON string of SimpleEvent array
   */
  get_pending_events(): string;
  
  /**
   * Get current configuration
   * @returns JSON string of OSPFConfig
   */
  get_config(): string;
  
  /**
   * Update configuration
   * @param config_json - JSON string of new OSPFConfig
   */
  update_config(config_json: string): void;
}

export class FeatureFlagController {
  constructor();
  
  /**
   * Enable refactored hello packet processing
   */
  enable_refactored_hello(): void;
  
  /**
   * Enable refactored DD packet processing
   */
  enable_refactored_dd(): void;
  
  /**
   * Enable all refactored packet processing
   */
  enable_all_refactored(): void;
  
  /**
   * Get current feature flags as JSON
   * @returns JSON string of FeatureFlags
   */
  get_flags(): string;
}

export class NetworkSimulator {
  constructor();
  
  // Existing methods...
  add_router(name: string, x: number, y: number): number;
  connect_routers(from_id: number, to_id: number, cost: number): void;
  delete_router(router_id: number): boolean;
  disconnect_routers(from_id: number, to_id: number): boolean;
  update_router_position(router_id: number, x: number, y: number): boolean;
  enable_ospf(router_id: number): void;
  start_simulation(): void;
  stop_simulation(): void;
  step_simulation(time_delta: number): void;
  get_routers_json(): string;
  get_connections_json(): string;
  get_recent_events_json(count: number): string;
  get_router_summary_json(router_id: number): string;
  get_all_events_json(): string;
  get_router_details_json(router_id: number): string;
  get_simulation_stats_json(): string;
  toggle_link_failure(from_id: number, to_id: number): boolean;
  toggle_router_failure(router_id: number): boolean;
  
  // New refactored engine methods
  
  /**
   * Enable the refactored OSPF engine with configuration
   * @param config_json - JSON string of OSPFConfig
   */
  enable_refactored_engine(config_json: string): void;
  
  /**
   * Get feature flags controller state
   * @returns JSON string of FeatureFlags
   */
  get_feature_flags(): string;
  
  /**
   * Enable specific refactored features
   */
  enable_refactored_hello(): void;
  
  /**
   * Enable all refactored features
   */
  enable_all_refactored(): void;
  
  /**
   * Process packet through refactored engine if enabled
   * @param packet_type - OSPF packet type (1-5)
   * @param packet_data - JSON string of packet data
   * @param from_router - Router ID of sender
   * @param interface_id - Interface ID where packet was received
   * @returns JSON string of generated events
   */
  process_packet_refactored(
    packet_type: number,
    packet_data: string,
    from_router: number,
    interface_id: number
  ): string;
}