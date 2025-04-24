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
        
        // Tile caching and management
        this.tileCache = new Map(); // Cache for loaded tiles
        this.maxCacheSize = 200;    // Maximum number of tiles to keep in cache
        this.tileLoadQueue = [];    // Queue for prioritizing tile loading
        this.tileUsageCounter = new Map(); // Track how recently tiles were used
        
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
        
        // Clear the tile load queue
        this.tileLoadQueue = [];
        
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
                
                // Update usage counter for this tile
                this.tileUsageCounter.set(tileId, Date.now());
                
                // Check if the tile is already rendered
                if (!document.getElementById(tileId)) {
                    // Add to load queue with priority based on distance from center
                    const distance = Math.sqrt(x*x + y*y);
                    this.tileLoadQueue.push({
                        tileId, tileX, tileY, offsetTileX: x, offsetTileY: y, 
                        offsetX, offsetY, distance
                    });
                } else {
                    // Update position for existing tile
                    const tile = document.getElementById(tileId);
                    const posX = Math.round(viewportWidth / 2 + (x * this.tileSize) - offsetX);
                    const posY = Math.round(viewportHeight / 2 + (y * this.tileSize) - offsetY);
                    tile.style.transform = `translate(${posX}px, ${posY}px)`;
                    
                    // If the tile was hidden, show it again
                    if (tile.style.display === 'none') {
                        tile.style.display = 'block';
                    }
                }
            }
        }
        
        // Handle tiles that are no longer visible
        this.visibleTiles.forEach(tileId => {
            if (!newVisibleTiles.has(tileId)) {
                const tile = document.getElementById(tileId);
                if (tile) {
                    // Hide the tile instead of removing it completely if it's in our cache
                    if (this.tileCache.has(tileId)) {
                        tile.style.display = 'none';
                    } else {
                        tile.remove();
                    }
                }
            }
        });
        
        // Update the set of visible tiles
        this.visibleTiles = newVisibleTiles;
        
        // Load tiles in order of priority (closest to center first)
        this.tileLoadQueue.sort((a, b) => a.distance - b.distance);
        this.processTileQueue();
        
        // Manage cache size
        this.manageCacheSize();
    }
    
    processTileQueue() {
        // Process the first few items immediately
        const immediateLoad = Math.min(5, this.tileLoadQueue.length);
        for (let i = 0; i < immediateLoad; i++) {
            this.createTile(
                this.tileLoadQueue[i].tileId,
                this.tileLoadQueue[i].tileX,
                this.tileLoadQueue[i].tileY,
                this.tileLoadQueue[i].offsetTileX,
                this.tileLoadQueue[i].offsetTileY,
                this.tileLoadQueue[i].offsetX,
                this.tileLoadQueue[i].offsetY
            );
        }
        
        // Process remaining tiles with delay to prevent browser from freezing
        if (this.tileLoadQueue.length > immediateLoad) {
            setTimeout(() => {
                const tile = this.tileLoadQueue[immediateLoad];
                if (tile && this.visibleTiles.has(tile.tileId)) {
                    this.createTile(
                        tile.tileId, tile.tileX, tile.tileY, 
                        tile.offsetTileX, tile.offsetTileY, 
                        tile.offsetX, tile.offsetY
                    );
                    
                    // Continue processing the queue
                    this.tileLoadQueue.splice(immediateLoad, 1);
                    if (this.tileLoadQueue.length > immediateLoad) {
                        this.processTileQueue();
                    }
                }
            }, 10);
        }
    }
    
    manageCacheSize() {
        // If cache exceeds max size, remove least recently used tiles
        if (this.tileCache.size > this.maxCacheSize) {
            // Convert to array and sort by last usage time
            const tileEntries = Array.from(this.tileUsageCounter.entries());
            tileEntries.sort((a, b) => a[1] - b[1]);
            
            // Remove oldest entries until we're back under the limit
            const tilesToRemove = tileEntries.slice(0, this.tileCache.size - this.maxCacheSize);
            
            tilesToRemove.forEach(([tileId]) => {
                // Remove from cache
                this.tileCache.delete(tileId);
                this.tileUsageCounter.delete(tileId);
                
                // If it's not visible, remove the DOM element too
                if (!this.visibleTiles.has(tileId)) {
                    const tile = document.getElementById(tileId);
                    if (tile) {
                        tile.remove();
                    }
                }
            });
        }
    }
    
    createTile(tileId, tileX, tileY, offsetTileX, offsetTileY, offsetX, offsetY) {
        const viewportWidth = this.mapContainer.clientWidth;
        const viewportHeight = this.mapContainer.clientHeight;
        
        // Check if we have this tile in the cache
        if (this.tileCache.has(tileId)) {
            const cachedTile = this.tileCache.get(tileId);
            cachedTile.style.display = 'block';
            
            // Update position
            const posX = Math.round(viewportWidth / 2 + (offsetTileX * this.tileSize) - offsetX);
            const posY = Math.round(viewportHeight / 2 + (offsetTileY * this.tileSize) - offsetY);
            cachedTile.style.transform = `translate(${posX}px, ${posY}px)`;
            
            return cachedTile;
        }
        
        const tile = document.createElement('div');
        tile.id = tileId;
        tile.className = 'map-tile';
        
        // Calculate position
        const posX = Math.round(viewportWidth / 2 + (offsetTileX * this.tileSize) - offsetX);
        const posY = Math.round(viewportHeight / 2 + (offsetTileY * this.tileSize) - offsetY);
        
        tile.style.transform = `translate(${posX}px, ${posY}px)`;
        
        // Create the tile URL with cache busting parameter
        const tileUrl = `/tile/${this.currentZoom}/${tileX}/${tileY}`;
        
        // Set background image to the tile
        tile.style.backgroundImage = `url('${tileUrl}')`;
        tile.style.backgroundSize = 'cover';

        // Add error handling for tile loading
        const img = new Image();
        img.onload = () => {
            tile.style.backgroundColor = 'transparent';
            // Add to cache once successfully loaded
            this.tileCache.set(tileId, tile);
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
        img.src = tileUrl;
        
        this.mapContainer.appendChild(tile);
        return tile;
    }
}

// Initialize the map viewer when the DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    const mapViewer = new RasterMapViewer();
});