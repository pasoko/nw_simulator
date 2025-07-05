/**
 * Animation Effects Module
 * Provides various animation effects for the network simulator
 */

export class AnimationEffects {
    constructor() {
        this.animations = new Map();
        this.frameId = null;
        this.startTime = null;
    }
    
    // Router selection animation
    animateRouterSelection(ctx, router, duration = 300) {
        const animId = `router-select-${router.id}`;
        
        this.animations.set(animId, {
            type: 'router-select',
            target: router,
            startTime: Date.now(),
            duration: duration,
            draw: (progress) => {
                const scale = 1 + Math.sin(progress * Math.PI) * 0.3;
                const opacity = 1 - progress * 0.5;
                
                ctx.save();
                ctx.globalAlpha = opacity;
                ctx.strokeStyle = '#2196F3';
                ctx.lineWidth = 3;
                ctx.beginPath();
                ctx.arc(router.x, router.y, 25 * scale, 0, 2 * Math.PI);
                ctx.stroke();
                ctx.restore();
            }
        });
        
        this.startAnimationLoop();
    }
    
    // Connection state change animation
    animateConnectionChange(ctx, from, to, isConnecting, duration = 500) {
        const animId = `conn-${from.id}-${to.id}`;
        
        this.animations.set(animId, {
            type: 'connection-change',
            startTime: Date.now(),
            duration: duration,
            draw: (progress) => {
                ctx.save();
                
                if (isConnecting) {
                    // Connecting animation - draw expanding line
                    const lineProgress = this.easeOutElastic(progress);
                    const endX = from.x + (to.x - from.x) * lineProgress;
                    const endY = from.y + (to.y - from.y) * lineProgress;
                    
                    ctx.strokeStyle = '#4CAF50';
                    ctx.lineWidth = 3;
                    ctx.setLineDash([5, 5]);
                    ctx.lineDashOffset = -progress * 10;
                    
                    ctx.beginPath();
                    ctx.moveTo(from.x, from.y);
                    ctx.lineTo(endX, endY);
                    ctx.stroke();
                    
                    // Draw pulse at the end
                    ctx.fillStyle = '#4CAF50';
                    ctx.beginPath();
                    ctx.arc(endX, endY, 5 + Math.sin(progress * Math.PI * 4) * 3, 0, 2 * Math.PI);
                    ctx.fill();
                } else {
                    // Disconnecting animation - fade out with particles
                    const opacity = 1 - progress;
                    ctx.globalAlpha = opacity;
                    ctx.strokeStyle = '#F44336';
                    ctx.lineWidth = 2;
                    
                    ctx.beginPath();
                    ctx.moveTo(from.x, from.y);
                    ctx.lineTo(to.x, to.y);
                    ctx.stroke();
                    
                    // Draw breaking particles
                    const particleCount = 5;
                    for (let i = 0; i < particleCount; i++) {
                        const t = i / particleCount;
                        const px = from.x + (to.x - from.x) * t;
                        const py = from.y + (to.y - from.y) * t;
                        const offset = progress * 20;
                        
                        ctx.fillStyle = '#F44336';
                        ctx.beginPath();
                        ctx.arc(
                            px + Math.sin(i * 2) * offset,
                            py + Math.cos(i * 2) * offset,
                            3 * (1 - progress),
                            0, 2 * Math.PI
                        );
                        ctx.fill();
                    }
                }
                
                ctx.restore();
            }
        });
        
        this.startAnimationLoop();
    }
    
    // OSPF state change animation
    animateOSPFStateChange(ctx, router, newState, duration = 600) {
        const animId = `ospf-state-${router.id}`;
        const stateColors = {
            'Down': '#F44336',
            'Init': '#FF9800',
            'TwoWay': '#FFC107',
            'ExStart': '#03A9F4',
            'Exchange': '#2196F3',
            'Loading': '#3F51B5',
            'Full': '#4CAF50'
        };
        
        this.animations.set(animId, {
            type: 'ospf-state',
            target: router,
            startTime: Date.now(),
            duration: duration,
            draw: (progress) => {
                const color = stateColors[newState] || '#666666';
                const waves = 3;
                
                ctx.save();
                
                for (let i = 0; i < waves; i++) {
                    const waveProgress = Math.max(0, progress - i * 0.2);
                    const radius = 25 + waveProgress * 30;
                    const opacity = (1 - waveProgress) * 0.3;
                    
                    if (waveProgress > 0 && waveProgress < 1) {
                        ctx.globalAlpha = opacity;
                        ctx.strokeStyle = color;
                        ctx.lineWidth = 2;
                        ctx.beginPath();
                        ctx.arc(router.x, router.y, radius, 0, 2 * Math.PI);
                        ctx.stroke();
                    }
                }
                
                ctx.restore();
            }
        });
        
        this.startAnimationLoop();
    }
    
    // Packet arrival burst effect
    animatePacketBurst(ctx, x, y, color, duration = 400) {
        const animId = `burst-${Date.now()}-${Math.random()}`;
        const particleCount = 8;
        
        this.animations.set(animId, {
            type: 'packet-burst',
            startTime: Date.now(),
            duration: duration,
            draw: (progress) => {
                ctx.save();
                
                for (let i = 0; i < particleCount; i++) {
                    const angle = (i / particleCount) * Math.PI * 2;
                    const distance = progress * 40;
                    const px = x + Math.cos(angle) * distance;
                    const py = y + Math.sin(angle) * distance;
                    const size = (1 - progress) * 4;
                    
                    ctx.globalAlpha = 1 - progress;
                    ctx.fillStyle = color;
                    ctx.beginPath();
                    ctx.arc(px, py, size, 0, 2 * Math.PI);
                    ctx.fill();
                }
                
                ctx.restore();
            }
        });
        
        this.startAnimationLoop();
    }
    
    // Hover effect for routers
    animateRouterHover(ctx, router, duration = 200) {
        const animId = `hover-${router.id}`;
        
        this.animations.set(animId, {
            type: 'router-hover',
            target: router,
            startTime: Date.now(),
            duration: duration,
            loop: true,
            draw: (progress) => {
                const pulse = Math.sin(progress * Math.PI * 2) * 0.5 + 0.5;
                
                ctx.save();
                ctx.strokeStyle = '#2196F3';
                ctx.lineWidth = 2;
                ctx.globalAlpha = pulse * 0.5;
                ctx.beginPath();
                ctx.arc(router.x, router.y, 30 + pulse * 5, 0, 2 * Math.PI);
                ctx.stroke();
                ctx.restore();
            }
        });
        
        this.startAnimationLoop();
    }
    
    // Stop hover animation
    stopRouterHover(routerId) {
        this.animations.delete(`hover-${routerId}`);
    }
    
    // Animation loop
    startAnimationLoop() {
        if (this.frameId) return;
        
        const animate = () => {
            const now = Date.now();
            const completed = [];
            
            this.animations.forEach((anim, id) => {
                const elapsed = now - anim.startTime;
                let progress = elapsed / anim.duration;
                
                if (anim.loop && progress > 1) {
                    anim.startTime = now;
                    progress = 0;
                } else if (progress > 1) {
                    // Don't draw if animation is complete
                    completed.push(id);
                    return;
                }
                
                // Only draw if progress is valid
                if (progress >= 0 && progress <= 1) {
                    anim.draw(progress);
                }
            });
            
            // Remove completed animations
            completed.forEach(id => this.animations.delete(id));
            
            if (this.animations.size > 0) {
                this.frameId = requestAnimationFrame(animate);
            } else {
                this.frameId = null;
            }
        };
        
        animate();
    }
    
    // Draw all active animations (called from canvas renderer)
    drawAnimations(ctx) {
        const now = Date.now();
        const completed = [];
        
        this.animations.forEach((anim, id) => {
            const elapsed = now - anim.startTime;
            let progress = elapsed / anim.duration;
            
            if (anim.loop && progress > 1) {
                anim.startTime = now;
                progress = 0;
            } else if (progress > 1) {
                completed.push(id);
                return;
            }
            
            // Only draw if progress is valid
            if (progress >= 0 && progress <= 1) {
                anim.draw(progress);
            }
        });
        
        // Remove completed animations
        completed.forEach(id => this.animations.delete(id));
    }
    
    // Easing functions
    easeOutElastic(t) {
        const c4 = (2 * Math.PI) / 3;
        return t === 0
            ? 0
            : t === 1
            ? 1
            : Math.pow(2, -10 * t) * Math.sin((t * 10 - 0.75) * c4) + 1;
    }
    
    easeInOutCubic(t) {
        return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
    }
    
    // Clear all animations
    clear() {
        this.animations.clear();
        if (this.frameId) {
            cancelAnimationFrame(this.frameId);
            this.frameId = null;
        }
    }
}

export default new AnimationEffects();