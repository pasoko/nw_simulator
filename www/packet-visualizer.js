export class PacketVisualizer {
    constructor(canvas, ctx) {
        this.canvas = canvas;
        this.ctx = ctx;
        this.packets = [];
        this.packetSpeed = 200; // pixels per second
        this.packetSize = 8;
        
        // Packet type colors
        this.packetColors = {
            'Hello': '#4CAF50',
            'Database Description': '#2196F3',
            'Link State Request': '#FF9800',
            'Link State Update': '#9C27B0',
            'Link State Acknowledgment': '#00BCD4'
        };
    }
    
    addPacket(fromRouter, toRouter, packetType, timestamp) {
        const packet = {
            id: Date.now() + Math.random(),
            from: fromRouter,
            to: toRouter,
            type: packetType,
            startTime: timestamp,
            progress: 0,
            path: this.calculatePath(fromRouter, toRouter),
            color: this.packetColors[packetType] || '#666666'
        };
        
        this.packets.push(packet);
    }
    
    calculatePath(fromRouter, toRouter) {
        // Simple linear path for now
        return {
            startX: fromRouter.x,
            startY: fromRouter.y,
            endX: toRouter.x,
            endY: toRouter.y,
            distance: Math.sqrt(
                Math.pow(toRouter.x - fromRouter.x, 2) + 
                Math.pow(toRouter.y - fromRouter.y, 2)
            )
        };
    }
    
    update(currentTime) {
        // Update packet positions
        this.packets = this.packets.filter(packet => {
            const elapsed = currentTime - packet.startTime;
            const travelTime = packet.path.distance / this.packetSpeed;
            packet.progress = Math.min(elapsed / travelTime, 1);
            
            // Remove packet when it reaches destination
            return packet.progress < 1;
        });
    }
    
    draw() {
        this.packets.forEach(packet => {
            const x = packet.path.startX + 
                (packet.path.endX - packet.path.startX) * packet.progress;
            const y = packet.path.startY + 
                (packet.path.endY - packet.path.startY) * packet.progress;
            
            // Draw packet
            this.ctx.beginPath();
            this.ctx.arc(x, y, this.packetSize, 0, 2 * Math.PI);
            this.ctx.fillStyle = packet.color;
            this.ctx.fill();
            
            // Draw packet trail
            this.ctx.beginPath();
            this.ctx.moveTo(packet.path.startX, packet.path.startY);
            this.ctx.lineTo(x, y);
            this.ctx.strokeStyle = packet.color;
            this.ctx.globalAlpha = 0.3;
            this.ctx.lineWidth = 2;
            this.ctx.stroke();
            this.ctx.globalAlpha = 1;
            
            // Draw packet type label
            if (packet.progress < 0.5) {
                this.ctx.fillStyle = packet.color;
                this.ctx.font = '10px Arial';
                this.ctx.fillText(packet.type, x + 10, y - 10);
            }
        });
    }
    
    clear() {
        this.packets = [];
    }
    
    getActivePacketCount() {
        return this.packets.length;
    }
    
    getPacketsByType() {
        const counts = {};
        this.packets.forEach(packet => {
            counts[packet.type] = (counts[packet.type] || 0) + 1;
        });
        return counts;
    }
}