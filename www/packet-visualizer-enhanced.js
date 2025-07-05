/**
 * Enhanced Packet Visualizer with improved animations
 */

export class PacketVisualizerEnhanced {
    constructor(canvas, ctx) {
        this.canvas = canvas;
        this.ctx = ctx;
        this.packets = [];
        this.packetSpeed = 200; // pixels per second
        this.packetSize = 8;
        this.effects = []; // For arrival effects
        
        // Enhanced packet type configurations
        this.packetConfigs = {
            'Hello': {
                color: '#4CAF50',
                gradient: ['#4CAF50', '#81C784'],
                icon: '👋',
                priority: 1
            },
            'Database Description': {
                color: '#2196F3',
                gradient: ['#2196F3', '#64B5F6'],
                icon: '📋',
                priority: 2
            },
            'Link State Request': {
                color: '#FF9800',
                gradient: ['#FF9800', '#FFB74D'],
                icon: '❓',
                priority: 3
            },
            'Link State Update': {
                color: '#9C27B0',
                gradient: ['#9C27B0', '#BA68C8'],
                icon: '📨',
                priority: 4
            },
            'Link State Acknowledgment': {
                color: '#00BCD4',
                gradient: ['#00BCD4', '#4DD0E1'],
                icon: '✅',
                priority: 5
            }
        };
        
        // Animation settings
        this.animations = {
            packetPulse: true,
            smoothTrails: true,
            arrivalEffects: true,
            glowEffect: true
        };
    }
    
    addPacket(fromRouter, toRouter, packetType, timestamp) {
        const config = this.packetConfigs[packetType] || {
            color: '#666666',
            gradient: ['#666666', '#999999'],
            icon: '📦',
            priority: 0
        };
        
        const packet = {
            id: Date.now() + Math.random(),
            from: fromRouter,
            to: toRouter,
            type: packetType,
            startTime: timestamp,
            progress: 0,
            path: this.calculateSmoothPath(fromRouter, toRouter),
            config: config,
            pulsePhase: Math.random() * Math.PI * 2, // Random start phase for pulse
            size: this.packetSize
        };
        
        this.packets.push(packet);
        this.sortPacketsByPriority();
    }
    
    calculateSmoothPath(fromRouter, toRouter) {
        const dx = toRouter.x - fromRouter.x;
        const dy = toRouter.y - fromRouter.y;
        const distance = Math.sqrt(dx * dx + dy * dy);
        
        // Calculate control points for bezier curve
        const midX = (fromRouter.x + toRouter.x) / 2;
        const midY = (fromRouter.y + toRouter.y) / 2;
        
        // Add slight curve to the path
        const perpX = -dy / distance * 20;
        const perpY = dx / distance * 20;
        
        return {
            startX: fromRouter.x,
            startY: fromRouter.y,
            endX: toRouter.x,
            endY: toRouter.y,
            controlX: midX + perpX,
            controlY: midY + perpY,
            distance: distance
        };
    }
    
    update(currentTime) {
        // Update packet positions
        this.packets = this.packets.filter(packet => {
            const elapsed = currentTime - packet.startTime;
            const travelTime = packet.path.distance / this.packetSpeed;
            packet.progress = Math.min(elapsed / travelTime, 1);
            
            // Add easing function for smoother movement
            packet.easedProgress = this.easeInOutCubic(packet.progress);
            
            // Update pulse animation
            if (this.animations.packetPulse) {
                packet.size = this.packetSize + Math.sin(elapsed * 0.01 + packet.pulsePhase) * 2;
            }
            
            // Check if packet reached destination
            if (packet.progress >= 1 && this.animations.arrivalEffects) {
                this.createArrivalEffect(packet.path.endX, packet.path.endY, packet.config.color);
                return false; // Remove packet
            }
            
            return packet.progress < 1;
        });
        
        // Update effects
        this.effects = this.effects.filter(effect => {
            effect.life -= 0.02;
            effect.radius += 2;
            return effect.life > 0;
        });
    }
    
    draw() {
        // Enable smooth rendering
        this.ctx.save();
        
        // Draw packet trails first
        if (this.animations.smoothTrails) {
            this.drawTrails();
        }
        
        // Draw packets
        this.packets.forEach(packet => {
            this.drawPacket(packet);
        });
        
        // Draw effects
        this.effects.forEach(effect => {
            this.drawEffect(effect);
        });
        
        this.ctx.restore();
    }
    
    drawPacket(packet) {
        // Calculate position along bezier curve
        const t = packet.easedProgress;
        const x = this.bezierPoint(
            packet.path.startX,
            packet.path.controlX,
            packet.path.endX,
            t
        );
        const y = this.bezierPoint(
            packet.path.startY,
            packet.path.controlY,
            packet.path.endY,
            t
        );
        
        // Draw glow effect
        if (this.animations.glowEffect) {
            this.ctx.shadowBlur = 15;
            this.ctx.shadowColor = packet.config.color;
        }
        
        // Draw packet with gradient
        const gradient = this.ctx.createRadialGradient(x, y, 0, x, y, packet.size);
        gradient.addColorStop(0, packet.config.gradient[1]);
        gradient.addColorStop(1, packet.config.gradient[0]);
        
        this.ctx.beginPath();
        this.ctx.arc(x, y, packet.size, 0, 2 * Math.PI);
        this.ctx.fillStyle = gradient;
        this.ctx.fill();
        
        // Reset shadow
        this.ctx.shadowBlur = 0;
        
        // Draw packet type label with fade
        if (packet.progress < 0.7) {
            const opacity = packet.progress < 0.5 ? 1 : (0.7 - packet.progress) / 0.2;
            this.ctx.globalAlpha = opacity;
            this.ctx.fillStyle = packet.config.color;
            this.ctx.font = 'bold 11px Arial';
            this.ctx.fillText(packet.type, x + 12, y - 12);
            this.ctx.globalAlpha = 1;
        }
    }
    
    drawTrails() {
        this.packets.forEach(packet => {
            const t = packet.easedProgress;
            
            // Draw smooth trail with gradient
            this.ctx.beginPath();
            
            // Draw bezier curve from start to current position
            this.ctx.moveTo(packet.path.startX, packet.path.startY);
            
            // Sample points along the curve for smooth trail
            const samples = 20;
            for (let i = 1; i <= samples * t; i++) {
                const st = i / samples;
                const sx = this.bezierPoint(
                    packet.path.startX,
                    packet.path.controlX,
                    packet.path.endX,
                    st
                );
                const sy = this.bezierPoint(
                    packet.path.startY,
                    packet.path.controlY,
                    packet.path.endY,
                    st
                );
                this.ctx.lineTo(sx, sy);
            }
            
            // Create gradient for trail
            const gradient = this.ctx.createLinearGradient(
                packet.path.startX, packet.path.startY,
                packet.path.endX, packet.path.endY
            );
            gradient.addColorStop(0, packet.config.color + '00');
            gradient.addColorStop(Math.min(t + 0.1, 1), packet.config.color + '40');
            gradient.addColorStop(Math.min(t + 0.2, 1), packet.config.color + '00');
            
            this.ctx.strokeStyle = gradient;
            this.ctx.lineWidth = 3;
            this.ctx.stroke();
        });
    }
    
    drawEffect(effect) {
        this.ctx.globalAlpha = effect.life;
        this.ctx.strokeStyle = effect.color;
        this.ctx.lineWidth = 2;
        this.ctx.beginPath();
        this.ctx.arc(effect.x, effect.y, effect.radius, 0, 2 * Math.PI);
        this.ctx.stroke();
        this.ctx.globalAlpha = 1;
    }
    
    createArrivalEffect(x, y, color) {
        this.effects.push({
            x: x,
            y: y,
            color: color,
            radius: 5,
            life: 1
        });
    }
    
    // Utility functions
    easeInOutCubic(t) {
        return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
    }
    
    bezierPoint(p0, p1, p2, t) {
        const u = 1 - t;
        return u * u * p0 + 2 * u * t * p1 + t * t * p2;
    }
    
    sortPacketsByPriority() {
        this.packets.sort((a, b) => a.config.priority - b.config.priority);
    }
    
    clear() {
        this.packets = [];
        this.effects = [];
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
    
    // Configuration methods
    setAnimationSettings(settings) {
        Object.assign(this.animations, settings);
    }
    
    setPacketSpeed(speed) {
        this.packetSpeed = speed;
    }
}