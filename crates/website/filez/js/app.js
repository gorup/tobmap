/**
 * Main application logic for TobMap viewer
 */
document.addEventListener('DOMContentLoaded', () => {
    // Initialize the map
    initMap();
});

// Global variables
let map;
let router;
let currentRoutingMode = false;
let mapLayer;
let globeProjection;
let startMarker, endMarker;

/**
 * Initialize the map and all related components
 */
function initMap() {
    // Create map instance
    map = L.map('map', {
        center: [0, 0], // Default center
        zoom: 2,        // Default zoom
        minZoom: 0,
        maxZoom: 10,
        maxBounds: [[-90, -180], [90, 180]], // Restrict panning to the world
        worldCopyJump: true // Jumps to copy of the world when panning across date line
    });
    
    // Create and add the TobMap tile layer
    const tobMapLayer = new TobMapLayer({
        baseUrl: '/tiles',
        maxZoom: 10,
        attribution: '© TobMap Contributors'
    });
    
    mapLayer = tobMapLayer.createTileLayer();
    mapLayer.addTo(map);
    
    // Initialize globe projection
    globeProjection = new GlobeProjection();
    
    // Initialize router
    router = new Router();
    
    // Set up event handlers
    setupEventHandlers();
    
    // Update coordinates display on mouse move
    map.on('mousemove', updateCoordinatesDisplay);
    
    // Initially hide the routing panel
    document.getElementById('routing-panel').classList.add('hidden');
    
    // Configure router events
    router.onRouteCalculated = (routeData) => {
        if (router.routeLayer) {
            router.routeLayer.remove();
        }
        
        const routeLayer = router.createRouteLayer(routeData);
        routeLayer.addTo(map);
        
        // Update route info
        document.getElementById('route-distance').textContent = routeData.distance;
        document.getElementById('route-time').textContent = routeData.time;
        document.getElementById('route-info').classList.remove('hidden');
    };
    
    router.onRouteError = (error) => {
        console.error('Route calculation error:', error);
        alert('Unable to calculate route. Please try different points.');
    };
}

/**
 * Set up all the event handlers for UI interaction
 */
function setupEventHandlers() {
    // Reset view button
    document.getElementById('reset-view').addEventListener('click', () => {
        map.setView([0, 0], 2);
    });
    
    // Toggle routing panel
    document.getElementById('toggle-routing').addEventListener('click', toggleRoutingPanel);
    
    // Map click handler for setting route points
    map.on('click', handleMapClick);
    
    // Clear start/end point buttons
    document.getElementById('clear-start').addEventListener('click', () => {
        clearRoutePoint('start');
    });
    
    document.getElementById('clear-end').addEventListener('click', () => {
        clearRoutePoint('end');
    });
    
    // Calculate route button
    document.getElementById('calculate-route').addEventListener('click', async () => {
        if (!router.isReady()) {
            alert('Please set both start and end points first.');
            return;
        }
        
        try {
            await router.calculateRoute();
        } catch (error) {
            console.error('Route calculation failed:', error);
        }
    });
    
    // Handle zoom levels for globe/map view
    map.on('zoomend', () => {
        const currentZoom = map.getZoom();
        
        // At very zoomed out levels, consider showing globe view
        if (currentZoom <= 1) {
            // This is where we would enable the 3D globe view
            // Currently just a stub
            //globeProjection.enable(document.getElementById('map'));
        } else {
            //globeProjection.disable();
        }
    });
}

/**
 * Toggle the routing panel visibility
 */
function toggleRoutingPanel() {
    const panel = document.getElementById('routing-panel');
    const isHidden = panel.classList.contains('hidden');
    
    if (isHidden) {
        panel.classList.remove('hidden');
        currentRoutingMode = true;
    } else {
        panel.classList.add('hidden');
        currentRoutingMode = false;
    }
}

/**
 * Handle map clicks based on current mode
 */
function handleMapClick(e) {
    if (!currentRoutingMode) return;
    
    const latLng = [e.latlng.lat, e.latlng.lng];
    
    // If no start point yet, set start point
    if (!router.startPoint) {
        setRoutePoint('start', latLng);
    } 
    // If no end point yet, set end point
    else if (!router.endPoint) {
        setRoutePoint('end', latLng);
    } 
    // If both points are set, replace the start point and clear the end point
    else {
        clearRoutePoint('end');
        setRoutePoint('start', latLng);
    }
    
    // Update calculate button state
    updateCalculateButtonState();
}

/**
 * Set a route point (start or end)
 */
function setRoutePoint(pointType, latLng) {
    // Update router state
    if (pointType === 'start') {
        router.setStartPoint(latLng);
        
        // Update display
        document.getElementById('start-point').textContent = 
            `${latLng[0].toFixed(6)}, ${latLng[1].toFixed(6)}`;
        
        // Add or move marker
        if (startMarker) {
            startMarker.setLatLng(latLng);
        } else {
            startMarker = L.marker(latLng, {
                icon: L.divIcon({
                    className: 'start-point-marker',
                    html: '<div class="marker-pin start-marker"></div>',
                    iconSize: [30, 42],
                    iconAnchor: [15, 42]
                })
            }).addTo(map);
        }
    } else {
        router.setEndPoint(latLng);
        
        // Update display
        document.getElementById('end-point').textContent = 
            `${latLng[0].toFixed(6)}, ${latLng[1].toFixed(6)}`;
        
        // Add or move marker
        if (endMarker) {
            endMarker.setLatLng(latLng);
        } else {
            endMarker = L.marker(latLng, {
                icon: L.divIcon({
                    className: 'end-point-marker',
                    html: '<div class="marker-pin end-marker"></div>',
                    iconSize: [30, 42],
                    iconAnchor: [15, 42]
                })
            }).addTo(map);
        }
    }
}

/**
 * Clear a route point (start or end)
 */
function clearRoutePoint(pointType) {
    if (pointType === 'start') {
        router.clearStartPoint();
        document.getElementById('start-point').textContent = 'Not set';
        
        if (startMarker) {
            startMarker.remove();
            startMarker = null;
        }
    } else {
        router.clearEndPoint();
        document.getElementById('end-point').textContent = 'Not set';
        
        if (endMarker) {
            endMarker.remove();
            endMarker = null;
        }
    }
    
    // Clear the route if any point is removed
    if (router.routeLayer) {
        router.routeLayer.remove();
        router.clearRouteLayer();
        document.getElementById('route-info').classList.add('hidden');
    }
    
    // Update calculate button state
    updateCalculateButtonState();
}

/**
 * Update the state of the calculate route button
 */
function updateCalculateButtonState() {
    const calculateButton = document.getElementById('calculate-route');
    calculateButton.disabled = !router.isReady();
}

/**
 * Update the coordinate display when mouse moves over map
 */
function updateCoordinatesDisplay(e) {
    const display = document.getElementById('coordinates-display');
    display.textContent = `Lat: ${e.latlng.lat.toFixed(5)}, Lng: ${e.latlng.lng.toFixed(5)}`;
}