/**
 * Resizable Panel Module
 * Handles resizing of sidebar panel with drag functionality
 */

class ResizablePanel {
    constructor() {
        this.sidebar = null;
        this.resizer = null;
        this.isResizing = false;
        this.minWidth = 280;
        this.maxWidth = 800;
        this.startX = 0;
        this.startWidth = 0;
        this.STORAGE_KEY = 'network-simulator-sidebar-width';
    }

    init() {
        this.sidebar = document.getElementById('sidebar');
        if (!this.sidebar) {
            console.error('Sidebar element not found!');
            return;
        }

        // Create resizer element
        this.createResizer();
        
        // Restore saved width
        this.restoreWidth();
        
        // Setup event listeners
        this.setupEventListeners();
    }

    createResizer() {
        this.resizer = document.createElement('div');
        this.resizer.className = 'sidebar-resizer';
        this.resizer.innerHTML = `
            <div class="resizer-handle">
                <div class="resizer-line"></div>
                <div class="resizer-line"></div>
                <div class="resizer-line"></div>
            </div>
        `;
        
        // Insert resizer to body
        document.body.appendChild(this.resizer);
        
        // Set initial position based on sidebar width
        const sidebarWidth = this.sidebar.offsetWidth || 632;
        this.resizer.style.left = (sidebarWidth - 8) + 'px'; // Adjust for margin
    }

    setupEventListeners() {
        // Mouse events
        this.resizer.addEventListener('mousedown', this.startResize.bind(this));
        document.addEventListener('mousemove', this.doResize.bind(this));
        document.addEventListener('mouseup', this.stopResize.bind(this));
        
        // Touch events for mobile
        this.resizer.addEventListener('touchstart', this.handleTouchStart.bind(this));
        document.addEventListener('touchmove', this.handleTouchMove.bind(this));
        document.addEventListener('touchend', this.stopResize.bind(this));
        
        // Double click to reset
        this.resizer.addEventListener('dblclick', this.resetWidth.bind(this));
    }

    startResize(e) {
        this.isResizing = true;
        this.startX = e.clientX;
        this.startWidth = parseInt(document.defaultView.getComputedStyle(this.sidebar).width, 10);
        
        // Add resizing class for visual feedback
        document.body.classList.add('resizing');
        this.resizer.classList.add('active');
        
        // Prevent text selection while resizing
        e.preventDefault();
    }

    doResize(e) {
        if (!this.isResizing) return;
        
        const width = this.startWidth + e.clientX - this.startX;
        const clampedWidth = Math.min(Math.max(width, this.minWidth), this.maxWidth);
        
        this.sidebar.style.width = clampedWidth + 'px';
        this.resizer.style.left = (clampedWidth - 8) + 'px'; // Adjust for margin
        
        // Dispatch resize event for other components to respond
        window.dispatchEvent(new CustomEvent('sidebarResized', { 
            detail: { width: clampedWidth } 
        }));
    }

    stopResize() {
        if (!this.isResizing) return;
        
        this.isResizing = false;
        document.body.classList.remove('resizing');
        this.resizer.classList.remove('active');
        
        // Save the width
        this.saveWidth();
    }

    handleTouchStart(e) {
        const touch = e.touches[0];
        const mouseEvent = new MouseEvent('mousedown', {
            clientX: touch.clientX,
            clientY: touch.clientY
        });
        this.startResize(mouseEvent);
    }

    handleTouchMove(e) {
        if (!this.isResizing) return;
        
        const touch = e.touches[0];
        const mouseEvent = new MouseEvent('mousemove', {
            clientX: touch.clientX,
            clientY: touch.clientY
        });
        this.doResize(mouseEvent);
    }

    resetWidth() {
        const defaultWidth = 632;
        this.sidebar.style.width = defaultWidth + 'px';
        this.resizer.style.left = (defaultWidth - 8) + 'px'; // Adjust for margin
        this.saveWidth();
        
        window.dispatchEvent(new CustomEvent('sidebarResized', { 
            detail: { width: defaultWidth } 
        }));
    }

    saveWidth() {
        const width = parseInt(this.sidebar.style.width, 10);
        try {
            localStorage.setItem(this.STORAGE_KEY, width.toString());
        } catch (error) {
            console.error('Error saving sidebar width:', error);
        }
    }

    restoreWidth() {
        try {
            const savedWidth = localStorage.getItem(this.STORAGE_KEY);
            if (savedWidth) {
                const width = parseInt(savedWidth, 10);
                if (width >= this.minWidth && width <= this.maxWidth) {
                    this.sidebar.style.width = width + 'px';
                    this.resizer.style.left = (width - 8) + 'px'; // Adjust for margin
                }
            }
        } catch (error) {
            console.error('Error restoring sidebar width:', error);
        }
    }

    getWidth() {
        return parseInt(this.sidebar.style.width, 10) || 632;
    }
}

// Export singleton instance
export default new ResizablePanel();