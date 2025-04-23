/**
 * Custom map layer provider for TobMap tiles
 */
class TobMapLayer {
    constructor(options = {}) {
        this.baseUrl = options.baseUrl || '/tiles';
        this.maxZoom = options.maxZoom || 10;
        this.minZoom = options.minZoom || 0;
        this.tileSize = options.tileSize || 256;
        this.attribution = options.attribution || 'TobMap';
    }

    /**
     * Create a Leaflet TileLayer for TobMap tiles
     * @returns {L.TileLayer} Leaflet tile layer
     */
    createTileLayer() {
        return L.tileLayer(`${this.baseUrl}/tile_z{z}_x{x}_y{y}.png`, {
            maxZoom: this.maxZoom,
            minZoom: this.minZoom,
            tileSize: this.tileSize,
            attribution: this.attribution,
            crossOrigin: true,
            // When no tile is found, use a transparent image
            errorTileUrl: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII='
        });
    }
}

/**
 * Globe projection system for 3D rendering
 * This can be used for more advanced globe view at zoomed out levels
 */
class GlobeProjection {
    constructor() {
        // Initialize Three.js components if needed
        this.isEnabled = false;
    }

    /**
     * Enable 3D globe view
     * @param {HTMLElement} container - Container element for the 3D view
     */
    enable(container) {
        this.isEnabled = true;
        console.log('3D Globe view enabled - Placeholder for Three.js integration');
        // Actual implementation would initialize a Three.js scene, camera, and renderer
    }

    /**
     * Disable 3D globe view and return to 2D
     */
    disable() {
        this.isEnabled = false;
        console.log('3D Globe view disabled');
        // Cleanup Three.js resources
    }

    /**
     * Update the globe view with current camera position
     * @param {Array} center - [lat, lng] center coordinate
     * @param {Number} zoom - Current zoom level
     */
    update(center, zoom) {
        if (!this.isEnabled) return;
        // Update 3D globe view based on current map center and zoom
    }
}

// Export the classes for use in other modules
window.TobMapLayer = TobMapLayer;
window.GlobeProjection = GlobeProjection;