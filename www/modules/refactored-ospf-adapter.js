/**
 * Adapter for using the refactored OSPF engine
 * 
 * This module provides a clean interface to gradually migrate
 * from the old implementation to the new refactored one.
 */

export class RefactoredOSPFAdapter {
    constructor(simulator) {
        this.simulator = simulator;
        this.enabled = false;
        this.config = {
            router_id: "1.1.1.1",
            area_id: "0.0.0.0",
            hello_interval: 10,
            dead_interval: 40,
            use_refactored_engine: false
        };
        this.eventHandlers = new Map();
    }

    /**
     * Initialize the refactored engine with config
     * @param {Object} config - OSPF configuration
     * @returns {boolean} Success status
     */
    async initialize(config = {}) {
        this.config = { ...this.config, ...config };
        
        try {
            await this.simulator.enable_refactored_engine(
                JSON.stringify(this.config)
            );
            this.enabled = true;
            console.log('Refactored OSPF engine initialized:', this.config);
            return true;
        } catch (error) {
            console.error('Failed to initialize refactored engine:', error);
            return false;
        }
    }

    /**
     * Enable specific features
     * @param {string} feature - Feature name (hello, dd, lsr, lsu, lsack, all)
     */
    enableFeature(feature) {
        if (!this.enabled) {
            console.warn('Refactored engine not initialized');
            return;
        }

        switch (feature) {
            case 'hello':
                this.simulator.enable_refactored_hello();
                break;
            case 'all':
                this.simulator.enable_all_refactored();
                break;
            default:
                console.warn(`Unknown feature: ${feature}`);
        }
    }

    /**
     * Get current feature flags
     * @returns {Object} Feature flags status
     */
    getFeatureFlags() {
        try {
            const flags = this.simulator.get_feature_flags();
            return JSON.parse(flags);
        } catch (error) {
            console.error('Failed to get feature flags:', error);
            return {};
        }
    }

    /**
     * Process a packet through the refactored engine
     * @param {number} packetType - OSPF packet type (1-5)
     * @param {Object} packetData - Packet data object
     * @param {number} fromRouter - Source router ID
     * @param {number} interfaceId - Interface ID
     * @returns {Array} Generated events
     */
    async processPacket(packetType, packetData, fromRouter, interfaceId) {
        if (!this.enabled) {
            throw new Error('Refactored engine not enabled');
        }

        try {
            const eventsJson = await this.simulator.process_packet_refactored(
                packetType,
                JSON.stringify(packetData),
                fromRouter,
                interfaceId
            );
            
            const events = JSON.parse(eventsJson);
            
            // Process events through handlers
            for (const event of events) {
                this.handleEvent(event);
            }
            
            return events;
        } catch (error) {
            console.error('Failed to process packet:', error);
            throw error;
        }
    }

    /**
     * Register an event handler
     * @param {string} eventType - Event type to handle
     * @param {Function} handler - Handler function
     */
    on(eventType, handler) {
        if (!this.eventHandlers.has(eventType)) {
            this.eventHandlers.set(eventType, []);
        }
        this.eventHandlers.get(eventType).push(handler);
    }

    /**
     * Handle an event
     * @param {Object} event - Event object
     */
    handleEvent(event) {
        const handlers = this.eventHandlers.get(event.event_type) || [];
        for (const handler of handlers) {
            try {
                handler(event);
            } catch (error) {
                console.error(`Error in event handler for ${event.event_type}:`, error);
            }
        }
    }

    /**
     * Create a test hello packet
     * @param {number} routerId - Router ID
     * @param {Array<number>} neighbors - Neighbor IDs
     * @returns {Object} Hello packet data
     */
    createTestHelloPacket(routerId, neighbors = []) {
        return {
            header: {
                version: 2,
                packet_type: 1,
                packet_length: 44,
                router_id: this.ipFromNumber(routerId),
                area_id: this.config.area_id,
                checksum: 0,
                auth_type: 0,
                authentication: [0, 0, 0, 0, 0, 0, 0, 0]
            },
            network_mask: "255.255.255.0",
            hello_interval: this.config.hello_interval,
            options: 2,
            priority: 1,
            dead_interval: this.config.dead_interval,
            designated_router: "0.0.0.0",
            backup_designated_router: "0.0.0.0",
            neighbors: neighbors.map(id => this.ipFromNumber(id))
        };
    }

    /**
     * Convert number to IP address string
     * @param {number} num - Number to convert
     * @returns {string} IP address
     */
    ipFromNumber(num) {
        return `${(num >> 24) & 255}.${(num >> 16) & 255}.${(num >> 8) & 255}.${num & 255}`;
    }

    /**
     * Run a migration test
     * @returns {Object} Test results
     */
    async runMigrationTest() {
        console.log('Running migration test...');
        
        const results = {
            initialized: false,
            featuresEnabled: false,
            helloProcessed: false,
            eventsGenerated: false,
            errors: []
        };

        try {
            // Initialize
            results.initialized = await this.initialize();
            if (!results.initialized) {
                throw new Error('Failed to initialize');
            }

            // Enable features
            this.enableFeature('all');
            const flags = this.getFeatureFlags();
            results.featuresEnabled = Object.values(flags).every(v => v === true);

            // Process a test hello packet
            const helloPacket = this.createTestHelloPacket(2, [1]);
            const events = await this.processPacket(1, helloPacket, 2, 1);
            
            results.helloProcessed = true;
            results.eventsGenerated = events.length > 0;

            console.log('Migration test completed:', results);
            console.log('Generated events:', events);

        } catch (error) {
            results.errors.push(error.message);
            console.error('Migration test failed:', error);
        }

        return results;
    }
}

// Export for use in other modules
export default RefactoredOSPFAdapter;