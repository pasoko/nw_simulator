/**
 * Connection Manager Module
 * Handles router connections and related operations
 */

import stateManager from './state-manager.js';

class ConnectionManager {
    constructor() {
        this.defaultCost = 1;
    }
    
    connectRouters(fromRouterId, toRouterId, cost = null) {
        // Validate routers exist
        const fromRouter = stateManager.findRouterById(fromRouterId);
        const toRouter = stateManager.findRouterById(toRouterId);
        
        if (!fromRouter || !toRouter) {
            alert('One or both selected routers no longer exist');
            return false;
        }
        
        // Check if already connected
        if (stateManager.connectionExists(fromRouterId, toRouterId)) {
            alert('Routers are already connected');
            return false;
        }
        
        // Get cost from user if not provided
        const connectionCost = cost !== null ? cost : 
            parseInt(prompt('Enter link cost:', this.defaultCost.toString()) || this.defaultCost.toString());
        
        try {
            stateManager.simulator.connect_routers(fromRouterId, toRouterId, connectionCost);
            
            // Add connection to local state
            // Note: Interface IDs will be updated when we get data from simulator
            stateManager.addConnection({
                from_router_id: fromRouterId,
                from_interface_id: 0,
                to_router_id: toRouterId,
                to_interface_id: 0,
                cost: connectionCost
            });
            
            return true;
        } catch (error) {
            console.error('Error connecting routers:', error);
            alert('Failed to connect routers: ' + error.message);
            return false;
        }
    }
    
    disconnectRouters(fromRouterId, toRouterId) {
        // Check if connection exists
        if (!stateManager.connectionExists(fromRouterId, toRouterId)) {
            alert('No connection exists between these routers');
            return false;
        }
        
        try {
            stateManager.simulator.disconnect_routers(fromRouterId, toRouterId);
            stateManager.removeConnection(fromRouterId, toRouterId);
            return true;
        } catch (error) {
            console.error('Error disconnecting routers:', error);
            alert('Failed to disconnect routers: ' + error.message);
            return false;
        }
    }
    
    handleConnectionMode(clickedRouter) {
        if (!clickedRouter) return null;
        
        stateManager.toggleRouterSelection(clickedRouter.id);
        
        if (stateManager.selectedRouters.length === 2) {
            const result = this.connectRouters(
                stateManager.selectedRouters[0],
                stateManager.selectedRouters[1]
            );
            
            stateManager.clearSelection();
            
            return {
                action: 'connected',
                success: result,
                from: stateManager.selectedRouters[0],
                to: stateManager.selectedRouters[1]
            };
        }
        
        return {
            action: 'selected',
            routerId: clickedRouter.id,
            count: stateManager.selectedRouters.length
        };
    }
    
    handleDisconnectionMode(clickedRouter) {
        if (!clickedRouter) return null;
        
        stateManager.toggleRouterSelection(clickedRouter.id);
        
        if (stateManager.selectedRouters.length === 2) {
            const result = this.disconnectRouters(
                stateManager.selectedRouters[0],
                stateManager.selectedRouters[1]
            );
            
            stateManager.clearSelection();
            
            return {
                action: 'disconnected',
                success: result,
                from: stateManager.selectedRouters[0],
                to: stateManager.selectedRouters[1]
            };
        }
        
        return {
            action: 'selected',
            routerId: clickedRouter.id,
            count: stateManager.selectedRouters.length
        };
    }
    
    updateConnectionsFromSimulator() {
        const connectionsJson = stateManager.simulator.get_connections_json();
        if (connectionsJson) {
            stateManager.connections = JSON.parse(connectionsJson);
        }
    }
}

// Export as singleton
export default new ConnectionManager();