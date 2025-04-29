/**
 * Main map application script
 */

// Global variables
let scene, camera, renderer, controls;
let sphere;
let zoomLevel = 1;
let currentS2Cells = [];
let loadedTiles = {};

// Initialize the 3D scene
function initScene() {
    // Create scene
    scene = new THREE.Scene();
    scene.background = new THREE.Color(0x000000);

    // Create camera
    camera = new THREE.PerspectiveCamera(60, window.innerWidth / window.innerHeight, 0.1, 1000);
    camera.position.z = 5;

    // Create renderer
    renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(window.innerWidth, window.innerHeight);
    document.getElementById('map-container').appendChild(renderer.domElement);

    // Create lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.5);
    scene.add(ambientLight);
    
    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.5);
    directionalLight.position.set(5, 3, 5);
    scene.add(directionalLight);

    // Create the earth sphere
    const earthGeometry = new THREE.SphereGeometry(2, 64, 64);
    const earthMaterial = new THREE.MeshPhongMaterial({
        color: 0x2233ff,
        emissive: 0x112244,
        specular: 0x333333,
        shininess: 25,
        transparent: true,
        opacity: 0.8
    });
    sphere = new THREE.Mesh(earthGeometry, earthMaterial);
    scene.add(sphere);

    // Create orbit controls
    controls = new THREE.OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.05;
    controls.rotateSpeed = 0.2;
    controls.minDistance = 2.001; // Zoom level 10 (closest, ~1 mile view)
    controls.maxDistance = 3.1;  // Zoom level 1 (farthest, ~2500 mile view)
    controls.enableZoom = false; // Disable scroll zoom

    // Add button listeners
    document.getElementById('zoom-in').addEventListener('click', zoomIn);
    document.getElementById('zoom-out').addEventListener('click', zoomOut);
    
    // Listen for control changes (pan/rotate)
    controls.addEventListener('change', onControlsChange);
    
    // Start animation loop
    animate();
    
    // Load initial tiles
    updateVisibleTiles();
    updateS2CellInfo(); // Initial S2 cell update
}

// Animation loop
function animate() {
    requestAnimationFrame(animate);
    controls.update();
    renderer.render(scene, camera);
}

// Update the display based on controls changes (pan/rotate only)
function onControlsChange() {
    // Update S2 cell info based on camera direction
    updateS2CellInfo();
}

// Zoom In function
function zoomIn() {
    if (zoomLevel < 10) {
        zoomLevel++;
        updateZoom();
    }
}

// Zoom Out function
function zoomOut() {
    if (zoomLevel > 1) {
        zoomLevel--;
        updateZoom();
    }
}

// Update zoom level display and camera distance
function updateZoom() {
    document.getElementById('current-zoom').textContent = zoomLevel;
    
    // Adjust camera distance based on new zoom level using an exponential scale
    // Zoom level 1 (farthest) corresponds to maxDistance, Zoom level 10 (closest) corresponds to minDistance
    const minD = controls.minDistance; // e.g., 2.001
    const maxD = controls.maxDistance; // e.g., 3.1
    const maxZoomLevel = 10;
    const minZoomLevel = 1;
    const zoomRange = maxZoomLevel - minZoomLevel; // e.g., 9
    
    // Calculate constants for d(zoom) = a * exp(b * zoom)
    // We want d(minZoomLevel) = maxD and d(maxZoomLevel) = minD
    // maxD = a * exp(b * minZoomLevel)
    // minD = a * exp(b * maxZoomLevel)
    // maxD / minD = exp(b * (minZoomLevel - maxZoomLevel)) = exp(b * -zoomRange)
    // b = -Math.log(maxD / minD) / zoomRange
    // a = maxD / Math.exp(b * minZoomLevel)
    const b = -Math.log(maxD / minD) / zoomRange;
    const a = maxD / Math.exp(b * minZoomLevel);
    
    const newDistance = a * Math.exp(b * zoomLevel);
    
    // Animate camera zoom smoothly (optional, but nice)
    // Get current camera direction
    const direction = new THREE.Vector3();
    camera.getWorldDirection(direction);
    
    // Calculate new position along the direction vector from the target
    const target = controls.target; // Usually the center (0,0,0)
    const newPosition = direction.multiplyScalar(-newDistance).add(target);
    
    // Use GSAP or Tween.js for smooth animation, or just set position directly
    // Simple direct set for now:
    camera.position.copy(newPosition);
    
    // Update controls target if needed (usually center of sphere)
    controls.update(); 
    
    updateVisibleTiles();
    updateS2CellInfo(); // Update S2 cell info after zoom
}

// Update S2 cell info in the UI
function updateS2CellInfo() {
    // Get the direction the camera is looking at (from camera towards target)
    const cameraDirection = new THREE.Vector3();
    camera.getWorldDirection(cameraDirection); 
    
    // Raycast from camera position in the view direction to find the center point on the sphere
    const raycaster = new THREE.Raycaster(camera.position, cameraDirection);
    const intersects = raycaster.intersectObject(sphere);

    if (intersects.length > 0) {
        const intersectionPoint = intersects[0].point;
        // Convert 3D intersection point to lat/lng
        const radius = sphere.geometry.parameters.radius; // Use actual sphere radius (should be 2)
        
        // Inverse calculation based on latLngToPoint function:
        // y = R * cos(phi) => phi = acos(y/R)
        // lat = 90 - phi * 180/PI
        const phi = Math.acos(intersectionPoint.y / radius);
        const lat = 90.0 - (phi * 180.0 / Math.PI);
        
        // tan(theta) = z / (-x) => theta = atan2(z, -x)
        // lng = theta * 180/PI - 180
        const theta = Math.atan2(intersectionPoint.z, -intersectionPoint.x);
        let lng = (theta * 180.0 / Math.PI) - 180.0;

        // Normalize lng to [-180, 180]
        lng = (lng + 180) % 360 - 180; // More robust normalization
        if (lng === -180) lng = 180; // Handle boundary case

        // Get S2 cell at current zoom level
        const s2Cell = S2.latLngToS2(lat, lng, zoomLevel);
        document.getElementById('current-s2-cell').textContent = s2Cell;
    } else {
        // If camera doesn't point at sphere (e.g., looking away)
        document.getElementById('current-s2-cell').textContent = '-';
    }
}

// Calculate intersection of ray with sphere
function calculateSphereIntersection(direction) {
    const raycaster = new THREE.Raycaster(camera.position, direction);
    const intersects = raycaster.intersectObject(sphere);
    
    if (intersects.length > 0) {
        return intersects[0].point;
    }
    
    return null;
}

// Update the visible tiles based on current view
function updateVisibleTiles() {
    // Clear existing tile objects
    clearExistingTiles();
    
    // For level 1, load all 24 face cells at S2 level 1
    if (zoomLevel === 1) {
        // S2 level 1 has 24 cells (4 children for each of the 6 face cells)
        for (let face = 0; face < 6; face++) {
            for (let pos = 0; pos < 4; pos++) {
                loadS2Cell(face.toString() + pos.toString());
            }
        }
    } else {
        // For higher levels, we'd need to determine which cells are visible
        // This is a simplified implementation
        const visibleCells = getVisibleS2Cells();
        
        for (const cellId of visibleCells) {
            loadS2Cell(cellId);
        }
    }
}

// Clear existing tile objects from the scene
function clearExistingTiles() {
    for (const cellId of currentS2Cells) {
        const cellObject = loadedTiles[cellId];
        if (cellObject) {
            sphere.remove(cellObject);
            delete loadedTiles[cellId];
        }
    }
    currentS2Cells = [];
}

// Determine which S2 cells are visible
function getVisibleS2Cells() {
    // This is a simplified implementation
    // In a real app, you'd determine visibility based on camera position and orientation
    
    // For demo purposes, we'll just create some sample cell IDs
    const baseCells = ["0", "1", "2", "3", "4", "5"];
    let cells = [];
    
    for (const baseCell of baseCells) {
        let cell = baseCell;
        // Add random child cells to reach the current zoom level
        for (let i = 1; i < zoomLevel; i++) {
            cell += Math.floor(Math.random() * 4).toString();
        }
        cells.push(cell);
    }
    
    return cells;
}

// Load the vector tile for an S2 cell
function loadS2Cell(cellId) {
    // Add to tracking arrays
    currentS2Cells.push(cellId);
    
    // Fetch the vector tile from the server
    // Adjust level: UI shows 1-10, API expects 0-9 for levels
    fetch(`/api/tiles/${zoomLevel}/${cellId}.pb`)
        .then(response => {
            if (!response.ok) {
                // If tile doesn't exist, render a placeholder
                renderPlaceholderCell(cellId);
                return;
            }
            return response.arrayBuffer();
        })
        .then(data => {
            if (data) {
                renderVectorTile(cellId, data);
            }
        })
        .catch(error => {
            console.error("Error loading tile:", error);
            renderPlaceholderCell(cellId);
        });
}

// Render a placeholder for a cell when data is not available
function renderPlaceholderCell(cellId) {
    const vertices = S2.getCellBoundary(cellId);
    
    if (vertices.length === 0) return;
    
    // Create a geometry for the cell
    const geometry = new THREE.BufferGeometry();
    const positionArray = [];
    
    // Convert lat/lng to 3D coordinates
    for (const vertex of vertices) {
        const point = latLngToPoint(vertex.lat, vertex.lng);
        positionArray.push(point.x, point.y, point.z);
    }
    
    // Create a line loop
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(positionArray, 3));
    const material = new THREE.LineBasicMaterial({ 
        color: 0xffff00, 
        linewidth: 2 
    });
    
    const lineLoop = new THREE.LineLoop(geometry, material);
    sphere.add(lineLoop);
    loadedTiles[cellId] = lineLoop;
}

// Render a vector tile with actual data
function renderVectorTile(cellId, data) {
    // In a real application, you would parse the protobuf data
    // For this demo, we'll just render a placeholder with a different color
    
    const vertices = S2.getCellBoundary(cellId);
    
    if (vertices.length === 0) return;
    
    // Create a geometry for the cell
    const geometry = new THREE.BufferGeometry();
    const positionArray = [];
    
    // Convert lat/lng to 3D coordinates and create faces
    for (let i = 0; i < vertices.length; i++) {
        const vertex = vertices[i];
        const point = latLngToPoint(vertex.lat, vertex.lng);
        positionArray.push(point.x, point.y, point.z);
    }
    
    // Create a polygon
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(positionArray, 3));
    
    // Calculate face normals
    geometry.computeVertexNormals();
    
    // Create a material with a color based on the cell ID
    const hashCode = cellId.split('').reduce((a, b) => {
        a = ((a << 5) - a) + b.charCodeAt(0);
        return a & a;
    }, 0);
    
    const color = new THREE.Color(Math.abs(hashCode) % 0xffffff);
    
    const material = new THREE.MeshPhongMaterial({ 
        color: color,
        transparent: true,
        opacity: 0.7,
        side: THREE.DoubleSide
    });
    
    const mesh = new THREE.Mesh(geometry, material);
    
    // Add a wireframe to highlight the boundaries
    const wireframe = new THREE.LineSegments(
        new THREE.WireframeGeometry(geometry),
        new THREE.LineBasicMaterial({
            color: 0xffffff,
            linewidth: 1,
            transparent: true,
            opacity: 0.5
        })
    );
    
    const group = new THREE.Group();
    group.add(mesh);
    group.add(wireframe);
    
    sphere.add(group);
    loadedTiles[cellId] = group;
}

// Convert latitude and longitude to 3D point on sphere
function latLngToPoint(lat, lng) {
    const phi = (90 - lat) * (Math.PI / 180);
    const theta = (lng + 180) * (Math.PI / 180);
    
    const x = -(2 * Math.sin(phi) * Math.cos(theta));
    const y = 2 * Math.cos(phi);
    const z = 2 * Math.sin(phi) * Math.sin(theta);
    
    return new THREE.Vector3(x, y, z);
}

// Handle window resize
function onWindowResize() {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
}

// Event listeners
window.addEventListener('resize', onWindowResize, false);

// Initialize when the page loads
window.addEventListener('load', initScene);