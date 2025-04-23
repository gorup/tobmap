/**
 * Handles routing functionality between two points
 */
class Router {
    constructor() {
        this.startPoint = null;
        this.endPoint = null;
        this.routeLayer = null;
        this.apiEndpoint = '/api/route'; // Replace with actual API endpoint
        
        // Event callbacks
        this.onRouteCalculated = null;
        this.onRouteError = null;
    }

    /**
     * Set the start point for routing
     * @param {Array} latLng - [latitude, longitude] array
     */
    setStartPoint(latLng) {
        this.startPoint = latLng;
        return this;
    }

    /**
     * Set the end point for routing
     * @param {Array} latLng - [latitude, longitude] array
     */
    setEndPoint(latLng) {
        this.endPoint = latLng;
        return this;
    }

    /**
     * Clear the start point
     */
    clearStartPoint() {
        this.startPoint = null;
        return this;
    }

    /**
     * Clear the end point
     */
    clearEndPoint() {
        this.endPoint = null;
        return this;
    }

    /**
     * Check if route calculation is ready (both points set)
     * @returns {Boolean} True if both points are set
     */
    isReady() {
        return this.startPoint && this.endPoint;
    }

    /**
     * Calculate a route between the start and end points
     * @returns {Promise} A promise that resolves to route data
     */
    async calculateRoute() {
        if (!this.isReady()) {
            throw new Error('Start and end points must be set before calculating a route');
        }

        try {
            // This is a stub for the API call that would be replaced with actual implementation
            // Simulating network delay
            const response = await new Promise(resolve => {
                setTimeout(() => {
                    resolve({
                        success: true,
                        route: this._generateDummyRoute(this.startPoint, this.endPoint),
                        distance: this._calculateHaversineDistance(this.startPoint, this.endPoint),
                        time: Math.floor(this._calculateHaversineDistance(this.startPoint, this.endPoint) / 5) // Assuming 5 m/s
                    });
                }, 500);
            });
            
            if (this.onRouteCalculated) {
                this.onRouteCalculated(response);
            }
            
            return response;
        } catch (error) {
            if (this.onRouteError) {
                this.onRouteError(error);
            }
            throw error;
        }
    }

    /**
     * Create a Leaflet layer with the route
     * @param {Object} routeData - Route data from the API
     * @param {Object} options - Styling options for the route
     * @returns {L.LayerGroup} Leaflet layer with the route
     */
    createRouteLayer(routeData, options = {}) {
        const layerGroup = L.layerGroup();
        
        // Style options with defaults
        const routeOptions = {
            color: options.color || '#3388ff',
            weight: options.weight || 5,
            opacity: options.opacity || 0.7
        };
        
        // Create the main route polyline
        const routeLine = L.polyline(routeData.route, routeOptions);
        layerGroup.addLayer(routeLine);
        
        // Add markers for start and end points
        if (routeData.route.length > 0) {
            const startMarker = L.circleMarker(routeData.route[0], {
                color: '#00c853',
                fillColor: '#00c853',
                fillOpacity: 1,
                radius: 8
            });
            
            const endMarker = L.circleMarker(routeData.route[routeData.route.length - 1], {
                color: '#d81b60',
                fillColor: '#d81b60',
                fillOpacity: 1,
                radius: 8
            });
            
            layerGroup.addLayer(startMarker);
            layerGroup.addLayer(endMarker);
        }
        
        this.routeLayer = layerGroup;
        return layerGroup;
    }

    /**
     * Clear the current route layer
     */
    clearRouteLayer() {
        this.routeLayer = null;
        return this;
    }

    /**
     * Generate a dummy route between two points
     * @param {Array} start - [lat, lng] start point
     * @param {Array} end - [lat, lng] end point
     * @returns {Array} Array of [lat, lng] coordinates representing the route
     * @private
     */
    _generateDummyRoute(start, end) {
        const route = [start];
        
        // Calculate number of intermediate points based on distance
        const distance = this._calculateHaversineDistance(start, end);
        const numPoints = Math.min(Math.max(Math.floor(distance / 100), 5), 20); // Between 5 and 20 points
        
        // Generate slightly randomized intermediate points
        for (let i = 1; i < numPoints; i++) {
            const factor = i / numPoints;
            
            // Linear interpolation with some randomness
            const lat = start[0] + (end[0] - start[0]) * factor + (Math.random() - 0.5) * 0.005;
            const lng = start[1] + (end[1] - start[1]) * factor + (Math.random() - 0.5) * 0.005;
            
            route.push([lat, lng]);
        }
        
        route.push(end);
        return route;
    }

    /**
     * Calculate Haversine distance between two points
     * @param {Array} point1 - [lat, lng] first point
     * @param {Array} point2 - [lat, lng] second point
     * @returns {Number} Distance in meters
     * @private
     */
    _calculateHaversineDistance(point1, point2) {
        const toRad = value => value * Math.PI / 180;
        const R = 6371000; // Earth's radius in meters
        
        const dLat = toRad(point2[0] - point1[0]);
        const dLon = toRad(point2[1] - point1[1]);
        
        const a = Math.sin(dLat/2) * Math.sin(dLat/2) +
                Math.cos(toRad(point1[0])) * Math.cos(toRad(point2[0])) *
                Math.sin(dLon/2) * Math.sin(dLon/2);
        
        const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1-a));
        
        return Math.round(R * c); // Return in meters, rounded
    }
}

// Export the router for use in other modules
window.Router = Router;