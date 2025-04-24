class RasterMapViewer {
    constructor() {
        this.mapContainer = document.getElementById('map');
        this.zoomInButton = document.getElementById('zoom-in');
        this.zoomOutButton = document.getElementById('zoom-out');
        this.zoomLevelDisplay = document.getElementById('zoom-level');
        
        // Map state
        this.currentZoom = 1;
        this.tileSize = 256;
        this.tilesX = {};
        this.tilesY = {};
        this.centerX = 0;
        this.centerY = 0;
        this.isDragging = false;
        this.lastMouseX = 0;
        this.lastMouseY = 0;
        this.visibleTiles = new Set();
        
        // Initialize the map
        this.initializeControls();
        this.updateMapView();
        
        // Add event listeners for window resize
        window.addEventListener('resize', () => this.updateMapView());
    }
    
    initializeControls() {
        // Zoom controls
        this.zoomInButton.addEventListener('click', () => this.zoomIn());
        this.zoomOutButton.addEventListener('click', () => this.zoomOut());
        
        // Pan controls
        this.mapContainer.addEventListener('mousedown', (e) => {
            this.isDragging = true;
            this.lastMouseX = e.clientX;
            this.lastMouseY = e.clientY;
            this.mapContainer.style.cursor = 'grabbing';
        });
        
        document.addEventListener('mousemove', (e) => {
            if (this.isDragging) {
                const dx = e.clientX - this.lastMouseX;
                const dy = e.clientY - this.lastMouseY;
                
                this.centerX -= dx;
                this.centerY -= dy;
                
                this.lastMouseX = e.clientX;
                this.lastMouseY = e.clientY;
                
                this.updateMapView();
            }
        });
        
        document.addEventListener('mouseup', () => {
            this.isDragging = false;
            this.mapContainer.style.cursor = 'grab';
        });
        
        // Add mouse wheel zoom
        this.mapContainer.addEventListener('wheel', (e) => {
            e.preventDefault();
            if (e.deltaY < 0) {
                this.zoomIn();
            } else {
                this.zoomOut();
            }
        });
        
        // Set initial cursor
        this.mapContainer.style.cursor = 'grab';
    }
    
    zoomIn() {
        if (this.currentZoom < 10) {
            this.currentZoom++;
            this.zoomLevelDisplay.textContent = `Zoom: ${this.currentZoom}`;
            
            // Adjust center coordinates for zoom
            this.centerX *= 2;
            this.centerY *= 2;
            
            this.updateMapView();
        }
    }
    
    zoomOut() {
        if (this.currentZoom > 1) {
            this.currentZoom--;
            this.zoomLevelDisplay.textContent = `Zoom: ${this.currentZoom}`;
            
            // Adjust center coordinates for zoom
            this.centerX /= 2;
            this.centerY /= 2;
            
            this.updateMapView();
        }
    }
    
    updateMapView() {
        const viewportWidth = this.mapContainer.clientWidth;
        const viewportHeight = this.mapContainer.clientHeight;
        
        // Calculate visible tile range
        const tilesInViewX = Math.ceil(viewportWidth / this.tileSize) + 2;
        const tilesInViewY = Math.ceil(viewportHeight / this.tileSize) + 2;
        
        // Calculate the center tile
        const centerTileX = Math.floor(this.centerX / this.tileSize);
        const centerTileY = Math.floor(this.centerY / this.tileSize);
        
        // Calculate the offset from the center of the screen
        const offsetX = this.centerX % this.tileSize;
        const offsetY = this.centerY % this.tileSize;
        
        // Track which tiles are currently visible
        const newVisibleTiles = new Set();
        
        // Draw visible tiles
        for (let x = -Math.floor(tilesInViewX / 2); x <= Math.ceil(tilesInViewX / 2); x++) {
            for (let y = -Math.floor(tilesInViewY / 2); y <= Math.ceil(tilesInViewY / 2); y++) {
                const tileX = centerTileX + x;
                const tileY = centerTileY + y;
                
                // Skip tiles with negative coordinates (they don't exist in our system)
                if (tileX < 0 || tileY < 0) continue;
                
                // Calculate max tiles for current zoom level
                const maxTilesAtZoom = Math.pow(2, this.currentZoom);
                
                // Skip tiles that are outside the bounds at this zoom level
                if (tileX >= maxTilesAtZoom || tileY >= maxTilesAtZoom) continue;
                
                const tileId = `tile-${this.currentZoom}-${tileX}-${tileY}`;
                newVisibleTiles.add(tileId);
                
                // Check if the tile is already rendered
                if (!document.getElementById(tileId)) {
                    this.createTile(tileId, tileX, tileY, x, y, offsetX, offsetY);
                } else {
                    // Update position for existing tile
                    const tile = document.getElementById(tileId);
                    const posX = Math.round(viewportWidth / 2 + (x * this.tileSize) - offsetX);
                    const posY = Math.round(viewportHeight / 2 + (y * this.tileSize) - offsetY);
                    tile.style.transform = `translate(${posX}px, ${posY}px)`;
                }
            }
        }
        
        // Remove tiles that are no longer visible
        this.visibleTiles.forEach(tileId => {
            if (!newVisibleTiles.has(tileId)) {
                const tile = document.getElementById(tileId);
                if (tile) {
                    tile.remove();
                }
            }
        });
        
        // Update the set of visible tiles
        this.visibleTiles = newVisibleTiles;
    }
    
    createTile(tileId, tileX, tileY, offsetTileX, offsetTileY, offsetX, offsetY) {
        const viewportWidth = this.mapContainer.clientWidth;
        const viewportHeight = this.mapContainer.clientHeight;
        
        const tile = document.createElement('div');
        tile.id = tileId;
        tile.className = 'map-tile';
        
        // Calculate position
        const posX = Math.round(viewportWidth / 2 + (offsetTileX * this.tileSize) - offsetX);
        const posY = Math.round(viewportHeight / 2 + (offsetTileY * this.tileSize) - offsetY);
        
        tile.style.transform = `translate(${posX}px, ${posY}px)`;
        
        // Set background image to the tile
        tile.style.backgroundImage = `url('/tile/${this.currentZoom}/${tileX}/${tileY}')`;
        tile.style.backgroundSize = 'cover';
        
        // Add error handling for tile loading
        const img = new Image();
        img.onload = () => {
            tile.style.backgroundColor = 'transparent';
        };
        img.onerror = () => {
            tile.style.backgroundColor = '#eee';
            tile.textContent = `${this.currentZoom}/${tileX}/${tileY}`;
            tile.style.display = 'flex';
            tile.style.justifyContent = 'center';
            tile.style.alignItems = 'center';
            tile.style.fontSize = '10px';
            tile.style.color = '#999';
        };
        img.src = `/tile/${this.currentZoom}/${tileX}/${tileY}`;
        
        this.mapContainer.appendChild(tile);
    }
}

// Initialize the map viewer when the DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    const mapViewer = new RasterMapViewer();
});