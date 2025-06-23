/**
 * Router Manager Module
 * Handles router-related operations and validation
 */

import stateManager from './state-manager.js';

class RouterManager {
    constructor() {
        this.maxNameLength = 20;
        this.namePattern = /^[a-zA-Z0-9\-_]+$/;
    }
    
    validateRouterName(name) {
        if (!name || !name.trim()) {
            return { valid: false, error: 'Router name is required' };
        }
        
        const trimmedName = name.trim();
        
        if (trimmedName.length > this.maxNameLength) {
            return { valid: false, error: `Router name must be ${this.maxNameLength} characters or less` };
        }
        
        if (!this.namePattern.test(trimmedName)) {
            return { valid: false, error: 'Router name can only contain letters, numbers, hyphens, and underscores' };
        }
        
        return { valid: true, name: trimmedName };
    }
    
    createRouter(name, x, y) {
        const validation = this.validateRouterName(name);
        if (!validation.valid) {
            alert(validation.error);
            return null;
        }
        
        const id = stateManager.simulator.add_router(validation.name, x, y);
        
        // Automatically enable OSPF on new routers
        stateManager.simulator.enable_ospf(id);
        
        const router = {
            id,
            name: validation.name,
            x,
            y,
            ospf_enabled: true
        };
        
        stateManager.addRouter(router);
        
        return router;
    }
    
    deleteRouter(routerId) {
        const router = stateManager.findRouterById(routerId);
        if (!router) return false;
        
        if (confirm(`Are you sure you want to delete router "${router.name}"?`)) {
            stateManager.simulator.delete_router(routerId);
            stateManager.removeRouter(routerId);
            return true;
        }
        
        return false;
    }
    
    updateRouterPosition(routerId, x, y) {
        const router = stateManager.findRouterById(routerId);
        if (!router) return false;
        
        // Constrain position to canvas bounds
        const canvas = document.getElementById('network-canvas');
        const constrainedX = Math.max(20, Math.min(canvas.width - 20, x));
        const constrainedY = Math.max(20, Math.min(canvas.height - 20, y));
        
        stateManager.updateRouterPosition(routerId, constrainedX, constrainedY);
        stateManager.simulator.update_router_position(routerId, constrainedX, constrainedY);
        
        return true;
    }
    
    toggleOSPF(routerId) {
        const router = stateManager.findRouterById(routerId);
        if (!router) return false;
        
        stateManager.simulator.enable_ospf(routerId);
        router.ospf_enabled = true;
        
        return true;
    }
    
    getRouterDetails(routerId) {
        const detailsJson = stateManager.simulator.get_router_details_json(routerId);
        return detailsJson ? JSON.parse(detailsJson) : {};
    }
    
    renderRouterDetails(details) {
        let html = '';
        
        // Interfaces section
        if (details.interfaces && Object.keys(details.interfaces).length > 0) {
            html += '<div class="detail-section"><h5>Interfaces:</h5><div class="router-interfaces">';
            Object.values(details.interfaces).forEach(iface => {
                html += `<div class="interface-item">
                    Interface ${iface.id}: ${iface.ip_address}/${iface.netmask}
                    ${iface.connected_router_id ? ` → Router ${iface.connected_router_id}` : ''}
                    (Cost: ${iface.cost})
                </div>`;
            });
            html += '</div></div>';
        }
        
        // Routing table section
        if (details.routing_table && details.routing_table.length > 0) {
            html += '<div class="detail-section"><h5>Routing Table:</h5>';
            html += '<table class="routing-table">';
            html += '<thead><tr><th>Destination</th><th>Next Hop</th><th>Interface</th><th>Metric</th></tr></thead>';
            html += '<tbody>';
            details.routing_table.forEach(entry => {
                html += `<tr>
                    <td>${entry.destination}/${entry.netmask}</td>
                    <td>${entry.next_hop}</td>
                    <td>if${entry.interface_id}</td>
                    <td>${entry.metric}</td>
                </tr>`;
            });
            html += '</tbody></table></div>';
        } else if (details.ospf_enabled) {
            html += '<div class="detail-section"><p style="color: #666; font-style: italic;">No routes in routing table</p></div>';
        }
        
        // OSPF Status
        if (details.ospf_enabled) {
            html += `<div class="detail-section ospf-status">
                <h5>OSPF Status:</h5>
                <div>Neighbors: ${details.ospf_neighbors || 0}</div>
                <div>LSA Database: ${details.lsa_database_size || 0} entries</div>
            </div>`;
        }
        
        return html;
    }
}

// Export as singleton
export default new RouterManager();