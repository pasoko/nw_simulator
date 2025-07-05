import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import themeManager from './theme-manager.js';

// Mock canvas renderer and router icon since they import theme manager
vi.mock('./canvas-renderer.js', () => ({
    default: {
        updateColors: vi.fn()
    }
}));

vi.mock('./router-icon.js', () => ({
    RouterIcon: vi.fn().mockImplementation(() => ({
        updateColors: vi.fn()
    }))
}));

describe('ThemeManager', () => {
    let originalMatchMedia;
    
    beforeEach(() => {
        // Reset theme manager state
        themeManager.initialized = false;
        themeManager.currentTheme = 'light';
        
        // Clear localStorage
        localStorage.clear();
        
        // Clear DOM
        document.body.innerHTML = '';
        document.documentElement.removeAttribute('data-theme');
        
        // Mock matchMedia
        originalMatchMedia = window.matchMedia;
        window.matchMedia = vi.fn().mockImplementation(query => ({
            matches: false,
            media: query,
            addEventListener: vi.fn(),
            removeEventListener: vi.fn()
        }));
        
        // Mock stateManager
        window.stateManager = {
            canvasRenderer: {
                updateColors: vi.fn()
            }
        };
    });
    
    afterEach(() => {
        window.matchMedia = originalMatchMedia;
        delete window.stateManager;
        vi.clearAllMocks();
    });
    
    describe('Initialization', () => {
        it('should initialize with light theme by default', () => {
            themeManager.init();
            
            expect(themeManager.currentTheme).toBe('light');
            expect(document.documentElement.getAttribute('data-theme')).toBe('light');
        });
        
        it('should initialize with saved theme from localStorage', () => {
            localStorage.setItem('network-simulator-theme', 'dark');
            
            themeManager.init();
            
            expect(themeManager.currentTheme).toBe('dark');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
        });
        
        it('should detect system dark mode preference', () => {
            window.matchMedia = vi.fn().mockImplementation(query => ({
                matches: query === '(prefers-color-scheme: dark)',
                media: query,
                addEventListener: vi.fn(),
                removeEventListener: vi.fn()
            }));
            
            themeManager.init();
            
            expect(themeManager.currentTheme).toBe('dark');
        });
        
        it('should create theme toggle button', () => {
            themeManager.init();
            
            const toggleButton = document.querySelector('.theme-toggle');
            expect(toggleButton).toBeTruthy();
            expect(toggleButton.querySelector('.sun-icon')).toBeTruthy();
            expect(toggleButton.querySelector('.moon-icon')).toBeTruthy();
        });
    });
    
    describe('Theme switching', () => {
        beforeEach(() => {
            themeManager.init();
        });
        
        it('should toggle between light and dark themes', () => {
            expect(themeManager.currentTheme).toBe('light');
            
            themeManager.toggleTheme();
            expect(themeManager.currentTheme).toBe('dark');
            expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
            
            themeManager.toggleTheme();
            expect(themeManager.currentTheme).toBe('light');
            expect(document.documentElement.getAttribute('data-theme')).toBe('light');
        });
        
        it('should save theme preference to localStorage', () => {
            themeManager.setTheme('dark');
            
            expect(localStorage.getItem('network-simulator-theme')).toBe('dark');
        });
        
        it('should not save theme when save parameter is false', () => {
            themeManager.setTheme('dark', false);
            
            expect(localStorage.getItem('network-simulator-theme')).toBeNull();
        });
        
        it('should dispatch themeChanged event', () => {
            const eventSpy = vi.fn();
            window.addEventListener('themeChanged', eventSpy);
            
            themeManager.setTheme('dark');
            
            expect(eventSpy).toHaveBeenCalled();
            expect(eventSpy.mock.calls[0][0].detail.theme).toBe('dark');
            
            window.removeEventListener('themeChanged', eventSpy);
        });
        
        it('should update canvas renderer colors', () => {
            themeManager.setTheme('dark');
            
            expect(window.stateManager.canvasRenderer.updateColors).toHaveBeenCalled();
        });
    });
    
    describe('Theme toggle button', () => {
        beforeEach(() => {
            themeManager.init();
        });
        
        it('should toggle theme when clicked', () => {
            const toggleButton = document.querySelector('.theme-toggle');
            
            toggleButton.click();
            expect(themeManager.currentTheme).toBe('dark');
            
            toggleButton.click();
            expect(themeManager.currentTheme).toBe('light');
        });
        
        it('should update aria-label based on current theme', () => {
            const toggleButton = document.querySelector('.theme-toggle');
            
            expect(toggleButton.getAttribute('aria-label')).toBe('Toggle theme');
            
            themeManager.setTheme('dark');
            expect(toggleButton.getAttribute('aria-label')).toBe('Switch to light mode');
            
            themeManager.setTheme('light');
            expect(toggleButton.getAttribute('aria-label')).toBe('Switch to dark mode');
        });
    });
    
    describe('System theme detection', () => {
        it('should listen for system theme changes', () => {
            const addEventListenerSpy = vi.fn();
            window.matchMedia = vi.fn().mockImplementation(() => ({
                matches: false,
                addEventListener: addEventListenerSpy,
                removeEventListener: vi.fn()
            }));
            
            themeManager.init();
            
            expect(addEventListenerSpy).toHaveBeenCalledWith('change', expect.any(Function));
        });
        
        it('should update theme when system preference changes if no saved preference', () => {
            const mockListener = vi.fn();
            window.matchMedia = vi.fn().mockImplementation(() => ({
                matches: false,
                addEventListener: (event, listener) => {
                    mockListener.mockImplementation(listener);
                },
                removeEventListener: vi.fn()
            }));
            
            themeManager.init();
            
            // Simulate system theme change to dark
            mockListener({ matches: true });
            expect(themeManager.currentTheme).toBe('dark');
            
            // Simulate system theme change to light
            mockListener({ matches: false });
            expect(themeManager.currentTheme).toBe('light');
        });
        
        it('should not update theme on system change if user has saved preference', () => {
            localStorage.setItem('network-simulator-theme', 'light');
            
            const mockListener = vi.fn();
            window.matchMedia = vi.fn().mockImplementation(() => ({
                matches: false,
                addEventListener: (event, listener) => {
                    mockListener.mockImplementation(listener);
                },
                removeEventListener: vi.fn()
            }));
            
            themeManager.init();
            
            // Simulate system theme change to dark
            mockListener({ matches: true });
            expect(themeManager.currentTheme).toBe('light'); // Should remain light
        });
    });
    
    describe('Helper methods', () => {
        beforeEach(() => {
            themeManager.init();
        });
        
        it('should return current theme', () => {
            expect(themeManager.getTheme()).toBe('light');
            
            themeManager.setTheme('dark');
            expect(themeManager.getTheme()).toBe('dark');
        });
        
        it('should check if dark mode is active', () => {
            expect(themeManager.isDarkMode()).toBe(false);
            
            themeManager.setTheme('dark');
            expect(themeManager.isDarkMode()).toBe(true);
        });
    });
    
    describe('Error handling', () => {
        it('should handle localStorage errors gracefully', () => {
            const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
            
            // Mock localStorage to throw error
            const originalGetItem = Storage.prototype.getItem;
            Storage.prototype.getItem = vi.fn().mockImplementation(() => {
                throw new Error('Storage error');
            });
            
            themeManager.init();
            
            expect(consoleErrorSpy).toHaveBeenCalled();
            expect(themeManager.currentTheme).toBe('light'); // Should fall back to default
            
            Storage.prototype.getItem = originalGetItem;
            consoleErrorSpy.mockRestore();
        });
        
        it('should validate theme values', () => {
            themeManager.setTheme('invalid-theme');
            
            expect(themeManager.currentTheme).toBe('light'); // Should fall back to light
        });
    });
});