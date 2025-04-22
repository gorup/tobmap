use std::f64::consts::PI;

use anyhow::Result;
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_line_segment_mut, draw_cross_mut, draw_filled_circle_mut};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use schema::tobmapgraph::{GraphBlob, LocationBlob, DescriptionBlob};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphVizError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse graph data: {0}")]
    ParseError(String),

    #[error("Failed to generate image: {0}")]
    ImageError(String),
}

pub type StatusOr<T> = Result<T, GraphVizError>;

/// Configuration for tile-based rendering
#[derive(Debug, Clone)]
pub struct TileConfig {
    pub rows: u32,         // Total number of rows in the grid
    pub columns: u32,      // Total number of columns in the grid
    pub row_index: u32,    // Current row to render (0-indexed)
    pub column_index: u32, // Current column to render (0-indexed)
    pub overlap_pixels: u32, // Overlap between tiles to avoid edge artifacts
}

/// Configuration for the visualization process.
#[derive(Debug, Clone)]
pub struct VizConfig<'a> {
    pub max_size: u32,
    pub node_size: u32,
    pub edge_width: f32,
    pub show_labels: bool,
    pub center_lat: Option<f64>,
    pub center_lng: Option<f64>,
    pub zoom_meters: Option<f64>,
    pub highlight_edge_index: Option<u32>,
    pub highlight_edge_width: Option<f32>,
    pub tile: Option<TileConfig>, // New field for tiling configuration
    pub description: Option<&'a DescriptionBlob<'a>>, // Make description optional
}

/// Pre-processed world data that can be reused across multiple tile renderings
pub struct WorldData {
    pub node_positions: Vec<(f64, f64)>,      // Longitude, Latitude for each node
    pub edge_paths: Vec<Vec<(f64, f64)>>,     // Paths of points for each edge
    pub edge_properties: Vec<EdgeProperties>, // Properties of each edge
    pub full_bounds: MapBounds,               // Geographic bounds of entire map
    pub full_dimensions: (u32, u32),          // Image dimensions for entire map
    pub nodes_count: usize,                   // Number of nodes
    pub edges_count: usize,                   // Number of edges
}

/// Geographic bounds of a map region
#[derive(Clone, Copy, Debug)]
pub struct MapBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lng: f64,
    pub max_lng: f64,
}

impl MapBounds {
    pub fn width(&self) -> f64 {
        self.max_lng - self.min_lng
    }
    
    pub fn height(&self) -> f64 {
        self.max_lat - self.min_lat
    }
}

/// Properties of an edge
#[derive(Clone, Debug)]
pub struct EdgeProperties {
    pub node1_idx: usize,
    pub node2_idx: usize,
    pub backwards_allowed: bool,
    pub time_seconds: u16,
    pub distance_meters: f64,
    pub priority_multiplier: f32,
    pub color: Rgb<u8>,
}

/// Converts S2 CellID to lat/lng
fn cell_id_to_latlng(cell_id: u64) -> LatLng {
    let cell = CellID(cell_id);
    LatLng::from(cell)
}

/// Helper function to draw a thick line by drawing circles along the path
fn draw_thick_line_segment_mut(
    image: &mut RgbImage,
    start: (f32, f32),
    end: (f32, f32),
    color: Rgb<u8>,
    width: f32,
) {
    if width <= 1.0 {
        // Use the standard thin line for width 1 or less
        draw_line_segment_mut(image, start, end, color);
        return;
    }

    let radius = (width / 2.0).max(1.0) as i32; // Ensure radius is at least 1
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length = (dx * dx + dy * dy).sqrt();

    if length < 0.001 {
        // Draw a circle if start and end are the same
        draw_filled_circle_mut(image, (start.0 as i32, start.1 as i32), radius, color);
        return;
    }

    // Iterate along the line and draw circles
    // Use a step slightly smaller than the radius to ensure coverage without too much overlap
    let step_size = (radius as f32 * 0.5).max(0.5); // Ensure step is at least 0.5 pixels
    let num_steps = (length / step_size).ceil() as i32;
    let step_x = dx * step_size / length;
    let step_y = dy * step_size / length;

    for i in 0..=num_steps {
        let t = i as f32;
        let x = start.0 + t * step_x;
        let y = start.1 + t * step_y;
        draw_filled_circle_mut(image, (x as i32, y as i32), radius, color);
    }
    // Ensure endpoints are covered explicitly as interpolation might miss them slightly
    draw_filled_circle_mut(image, (start.0 as i32, start.1 as i32), radius, color);
    draw_filled_circle_mut(image, (end.0 as i32, end.1 as i32), radius, color);
}

/// Draw an arrow head at a specified point with a given direction
fn draw_arrow_head(image: &mut RgbImage, from: (f32, f32), to: (f32, f32), color: Rgb<u8>, size: f32, line_width: f32) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length = (dx * dx + dy * dy).sqrt();

    if length < 0.001 {
        return;
    }

    // Normalize direction vector
    let direction_x = dx / length;
    let direction_y = dy / length;

    // Calculate perpendicular vector
    let perpendicular_x = -direction_y;
    let perpendicular_y = direction_x;

    // Arrow head points (base of the arrowhead)
    let arrow_base_x = to.0 - direction_x * size;
    let arrow_base_y = to.1 - direction_y * size;

    let point1 = (
        arrow_base_x + perpendicular_x * size/2.0,
        arrow_base_y + perpendicular_y * size/2.0
    );

    let point2 = (
        arrow_base_x - perpendicular_x * size/2.0,
        arrow_base_y - perpendicular_y * size/2.0
    );

    // Draw arrow head using thick lines
    draw_thick_line_segment_mut(image, to, point1, color, line_width);
    draw_thick_line_segment_mut(image, to, point2, color, line_width);
}

/// Calculate color based on speed (distance/time)
/// Slow segments are red, fast segments are green
fn get_speed_color(distance_meters: f64, time_seconds: u16) -> Rgb<u8> {
    // Avoid division by zero
    if time_seconds == 0 {
        return Rgb([0, 255, 0]); // Maximum green for instant travel
    }

    // Calculate speed in m/s
    let speed = distance_meters / time_seconds as f64;

    // Define thresholds for coloring (adjust these based on your data)
    // These values represent walking/cycling/driving speeds roughly
    let slow_threshold = 1.5;  // m/s (walking pace ~5 km/h)
    let fast_threshold = 13.0; // m/s (fast road ~47 km/h)

    // Normalize speed to 0-1 range
    let normalized = if speed <= slow_threshold {
        0.0
    } else if speed >= fast_threshold {
        1.0
    } else {
        (speed - slow_threshold) / (fast_threshold - slow_threshold)
    };

    // Convert to RGB (red to green)
    let red = ((1.0 - normalized) * 255.0) as u8;
    let green = (normalized * 255.0) as u8;

    Rgb([red, green, 0])
}

/// Calculate distance between two lat/lng points in meters
fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let earth_radius = 6371000.0; // Earth radius in meters

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();

    let a = (dlat/2.0).sin() * (dlat/2.0).sin() +
            lat1_rad.cos() * lat2_rad.cos() *
            (dlng/2.0).sin() * (dlng/2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0-a).sqrt());

    earth_radius * c
}

/// Approximate meters per degree of latitude
const METERS_PER_DEGREE_LAT: f64 = 111132.954; // Average

/// Approximate meters per degree of longitude at a given latitude
fn meters_per_degree_lng(latitude: f64) -> f64 {
    111319.488 * latitude.to_radians().cos()
}

// Keeping the old function for compatibility but mark it as deprecated
#[deprecated]
fn get_cost_color(cost: u8) -> Rgb<u8> {
    // Ensure cost is in range 0-15
    let cost = cost.min(15);

    // Calculate red and green components - flipped from previous implementation
    let red = (cost as u32 * 255 / 15) as u8;
    let green = 255 - (cost as u32 * 255 / 15) as u8;

    Rgb([red, green, 0])
}

/// Simple line-rectangle intersection check (Axis-Aligned Bounding Box)
fn line_crosses_bounds(x1: f64, y1: f64, x2: f64, y2: f64, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> bool {
    // Check if both points are outside on the same side
    if (x1 < min_x && x2 < min_x) || (x1 > max_x && x2 > max_x) ||
       (y1 < min_y && y2 < min_y) || (y1 > max_y && y2 > max_y) {
        return false;
    }
    // Basic check passed, assume it might cross or be inside
    // A more precise check (like Liang-Barsky) could be used here if needed.
    true
}

/// Pre-process graph data into reusable WorldData structure
pub fn process_world_data(
    graph: &GraphBlob, 
    location: &LocationBlob, 
    description: Option<&DescriptionBlob>,
    max_size: u32
) -> StatusOr<WorldData> {
    // Extract all nodes and edges
    let nodes = graph.nodes().ok_or_else(|| GraphVizError::ParseError("Failed to get nodes".to_string()))?;
    let edges = graph.edges().ok_or_else(|| GraphVizError::ParseError("Failed to get edges".to_string()))?;

    // Get node and edge locations
    let node_locations = location.node_location_items().ok_or_else(||
        GraphVizError::ParseError("Failed to get node locations".to_string()))?;
    let edge_locations = location.edge_location_items().ok_or_else(||
        GraphVizError::ParseError("Failed to get edge locations".to_string()))?;

    // Get edge descriptions if available
    let edge_descriptions = description.and_then(|desc| desc.edge_descriptions());

    // Verify we have the same number of nodes and node locations
    if nodes.len() != node_locations.len() {
        return Err(GraphVizError::ParseError(format!(
            "Mismatch between nodes count ({}) and node locations count ({})",
            nodes.len(), node_locations.len())));
    }
    // Verify we have the same number of edges and edge locations
    if edges.len() != edge_locations.len() {
        return Err(GraphVizError::ParseError(format!(
            "Mismatch between edges count ({}) and edge locations count ({})",
            edges.len(), edge_locations.len())));
    }

    // Store all node positions and calculate bounds
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lng = f64::MAX;
    let mut max_lng = f64::MIN;

    // Store all node positions
    let node_positions: Vec<(f64, f64)> = (0..node_locations.len())
        .map(|i| {
            let node_location = node_locations.get(i);
            let latlng = cell_id_to_latlng(node_location.cell_id());
            let lng = latlng.lng.deg();
            let lat = latlng.lat.deg();
            
            // Update bounds
            min_lat = min_lat.min(lat);
            max_lat = max_lat.max(lat);
            min_lng = min_lng.min(lng);
            max_lng = max_lng.max(lng);
            
            (lng, lat) // x = longitude, y = latitude
        })
        .collect();

    // Store map bounds
    let bounds = MapBounds {
        min_lat,
        max_lat,
        min_lng,
        max_lng,
    };

    // Calculate full image dimensions
    let aspect_ratio = bounds.width() / bounds.height();
    let (full_img_width, full_img_height) = if aspect_ratio > 1.0 {
        (max_size, (max_size as f64 / aspect_ratio) as u32)
    } else {
        ((max_size as f64 * aspect_ratio) as u32, max_size)
    };

    // Pre-process all edge paths and properties
    let mut edge_paths = Vec::with_capacity(edges.len());
    let mut edge_properties = Vec::with_capacity(edges.len());

    for i in 0..edges.len() {
        let edge = edges.get(i);
        let edge_location = edge_locations.get(i);

        let node1_idx = edge.point_1_node_idx() as usize;
        let node2_idx = edge.point_2_node_idx() as usize;

        if node1_idx >= node_positions.len() || node2_idx >= node_positions.len() {
            eprintln!("Warning: Edge {} references non-existent node index {} or {}", i, node1_idx, node2_idx);
            // Add empty data to maintain indices alignment
            edge_paths.push(Vec::new());
            edge_properties.push(EdgeProperties {
                node1_idx,
                node2_idx,
                backwards_allowed: false,
                time_seconds: 0,
                distance_meters: 0.0,
                priority_multiplier: 1.0,
                color: Rgb([0, 0, 0]),
            });
            continue;
        }

        let (lng1, lat1) = node_positions[node1_idx];
        let (lng2, lat2) = node_positions[node2_idx];

        // Extract edge properties
        let costs_and_flags = edge.costs_and_flags();
        let backwards_allowed = (costs_and_flags & 0b0000_0000_0000_0001) != 0;
        let time_seconds: u16 = (costs_and_flags >> 3) as u16;
        let distance_meters = haversine_distance(lat1, lng1, lat2, lng2);
        
        // Get edge priority from description if available
        let mut priority_multiplier = 1.0;
        if let Some(descriptions) = edge_descriptions {
            if i < descriptions.len() {
                let desc = descriptions.get(i);
                let priority = desc.priority();

                // Classify priority into width categories
                priority_multiplier = match priority {
                    0..=4 => 1.0,  // Lowest priority
                    5..=6 => 2.0,  // Medium-low priority
                    7..=8 => 3.0,  // Medium-high priority
                    _ => 5.0,      // Highest priority
                };
            }
        }

        // Determine edge color
        let color = get_speed_color(distance_meters, time_seconds);

        // Store edge properties
        edge_properties.push(EdgeProperties {
            node1_idx,
            node2_idx,
            backwards_allowed,
            time_seconds,
            distance_meters,
            priority_multiplier,
            color,
        });

        // Construct the full path for the edge
        let mut path = Vec::new();
        path.push((lng1, lat1)); // Start node

        // Add intermediate points if any
        if let Some(cell_ids) = edge_location.points() {
            if cell_ids.len() > 0 {
                for cell_id in cell_ids {
                    let latlng = cell_id_to_latlng(cell_id);
                    path.push((latlng.lng.deg(), latlng.lat.deg()));
                }
            }
        }

        path.push((lng2, lat2)); // End node
        edge_paths.push(path);
    }

    // Return the processed world data
    Ok(WorldData {
        node_positions,
        edge_paths,
        edge_properties,
        full_bounds: bounds,
        full_dimensions: (full_img_width, full_img_height),
        nodes_count: nodes.len(),
        edges_count: edges.len(),
    })
}

/// Render a tile using pre-processed world data
pub fn render_tile(
    world: &WorldData,
    config: &VizConfig,
    min_priority: usize,
) -> StatusOr<RgbImage> {
    // Get base configuration values
    let node_size = config.node_size;
    let base_edge_width = config.edge_width;
    let highlight_edge_index = config.highlight_edge_index;
    let highlight_edge_width = config.highlight_edge_width;
    let show_labels = config.show_labels;

    // Default to full map bounds
    let mut bounds = world.full_bounds;
    let mut img_width = world.full_dimensions.0;
    let mut img_height = world.full_dimensions.1;

    // If zooming is enabled, adjust bounds
    if let (Some(center_lat), Some(center_lng), Some(zoom_meters)) = (config.center_lat, config.center_lng, config.zoom_meters) {
        // Calculate bounds based on center and zoom
        let meters_per_lng = meters_per_degree_lng(center_lat);
        if meters_per_lng <= 0.0 { // Avoid division by zero near poles
             return Err(GraphVizError::ImageError("Cannot calculate longitude span near poles.".to_string()));
        }
        let delta_lat = (zoom_meters / 2.0) / METERS_PER_DEGREE_LAT;
        let delta_lng = (zoom_meters / 2.0) / meters_per_lng;

        bounds.min_lat = center_lat - delta_lat;
        bounds.max_lat = center_lat + delta_lat;
        bounds.min_lng = center_lng - delta_lng;
        bounds.max_lng = center_lng + delta_lng;
    }

    // If we're rendering a tile, adjust bounds and dimensions
    if let Some(tile) = &config.tile {
        // Validate tile configuration
        if tile.row_index >= tile.rows || tile.column_index >= tile.columns {
            return Err(GraphVizError::ImageError(format!(
                "Invalid tile indices: row_index={}, rows={}, column_index={}, columns={}",
                tile.row_index, tile.rows, tile.column_index, tile.columns
            )));
        }

        // Calculate the geographic bounds for this specific tile
        let tile_width = world.full_bounds.width() / tile.columns as f64;
        let tile_height = world.full_bounds.height() / tile.rows as f64;

        // Calculate actual tile bounds with overlap
        let overlap_lng = (tile.overlap_pixels as f64 / world.full_dimensions.0 as f64) * world.full_bounds.width();
        let overlap_lat = (tile.overlap_pixels as f64 / world.full_dimensions.1 as f64) * world.full_bounds.height();

        // Update bounds for this specific tile (with overlap)
        bounds.min_lng = world.full_bounds.min_lng + tile.column_index as f64 * tile_width 
            - (if tile.column_index > 0 { overlap_lng } else { 0.0 });
        bounds.max_lng = world.full_bounds.min_lng + (tile.column_index + 1) as f64 * tile_width 
            + (if tile.column_index + 1 < tile.columns { overlap_lng } else { 0.0 });
        
        // Note: latitude increases northward (upward) but image y-coordinates increase downward
        bounds.max_lat = world.full_bounds.max_lat - tile.row_index as f64 * tile_height 
            + (if tile.row_index > 0 { overlap_lat } else { 0.0 });
        bounds.min_lat = world.full_bounds.max_lat - (tile.row_index + 1) as f64 * tile_height 
            - (if tile.row_index + 1 < tile.rows { overlap_lat } else { 0.0 });

        // Calculate tile image dimensions - keep each tile the same size regardless of zoom level
        // Don't divide by number of tiles; instead use the same image dimensions for each tile
        img_width = world.full_dimensions.0;
        img_height = world.full_dimensions.1;
        
        // Add overlap pixels if needed
        if tile.overlap_pixels > 0 {
            img_width += (if tile.column_index > 0 { tile.overlap_pixels } else { 0 }) + 
                (if tile.column_index + 1 < tile.columns { tile.overlap_pixels } else { 0 });
            
            img_height += (if tile.row_index > 0 { tile.overlap_pixels } else { 0 }) + 
                (if tile.row_index + 1 < tile.rows { tile.overlap_pixels } else { 0 });
        }
    }

    // Create an empty white image
    let mut image = RgbImage::new(img_width, img_height);
    let white = Rgb([255, 255, 255]);
    let gray = Rgb([128, 128, 128]);
    let yellow = Rgb([255, 255, 0]); // Highlight color

    // Fill with white
    for pixel in image.pixels_mut() {
        *pixel = white;
    }

    // Helper function to convert lat/lng to image coordinates
    let to_img_coords = |lng: f64, lat: f64| -> (f32, f32) {
        let x = (lng - bounds.min_lng) / bounds.width() * img_width as f64;
        // Note: y-axis is inverted (0 at top)
        let y = (bounds.max_lat - lat) / bounds.height() * img_height as f64;
        (x as f32, y as f32)
    };

    // Helper to check if a point is within bounds
    let is_in_bounds = |lng: f64, lat: f64| -> bool {
        lng >= bounds.min_lng && lng <= bounds.max_lng && lat >= bounds.min_lat && lat <= bounds.max_lat
    };

    // Arrow size for direction indicators (relative to edge width)
    let arrow_size = 6.0 * base_edge_width.max(1.0);

    // Add edges to image
    for (i, (path, props)) in world.edge_paths.iter().zip(world.edge_properties.iter()).enumerate() {
        if path.is_empty() {
            continue; // Skip edges with empty paths
        }

        // Skip edges with priority lower than min_priority
        let edge_priority = (props.priority_multiplier * 2.0) as usize;
        if edge_priority < min_priority {
            continue;
        }

        // Determine if this is the highlighted edge
        let is_highlighted = highlight_edge_index.map_or(false, |idx| i == idx as usize);

        // Set edge color and width
        let color = if is_highlighted { yellow } else { props.color };
        let width = if is_highlighted {
            highlight_edge_width.unwrap_or(base_edge_width * 2.0 * props.priority_multiplier)
        } else {
            base_edge_width * props.priority_multiplier
        };

        // Draw segments of the path
        let mut last_segment_visible = false;
        for j in 0..path.len() - 1 {
            let (p1_lng, p1_lat) = path[j];
            let (p2_lng, p2_lat) = path[j+1];

            // Check if segment is potentially visible
            let p1_in_bounds = is_in_bounds(p1_lng, p1_lat);
            let p2_in_bounds = is_in_bounds(p2_lng, p2_lat);

            // Draw segment if visible or crossing bounds
            if p1_in_bounds || p2_in_bounds || line_crosses_bounds(
                p1_lng, p1_lat, p2_lng, p2_lat, 
                bounds.min_lng, bounds.min_lat, bounds.max_lng, bounds.max_lat
            ) {
                let (x1, y1) = to_img_coords(p1_lng, p1_lat);
                let (x2, y2) = to_img_coords(p2_lng, p2_lat);

                draw_thick_line_segment_mut(&mut image, (x1, y1), (x2, y2), color, width);
                last_segment_visible = true;
            } else {
                last_segment_visible = false;
            }
        }

        // Draw arrow head for one-way edges at the end of the path if visible
        if !props.backwards_allowed && path.len() >= 2 && last_segment_visible {
            let (p_last_lng, p_last_lat) = path[path.len() - 1];
            let (p_second_last_lng, p_second_last_lat) = path[path.len() - 2];

            if is_in_bounds(p_last_lng, p_last_lat) || is_in_bounds(p_second_last_lng, p_second_last_lat) {
                let (x_last, y_last) = to_img_coords(p_last_lng, p_last_lat);
                let (x_second_last, y_second_last) = to_img_coords(p_second_last_lng, p_second_last_lat);

                let dx = x_last - x_second_last;
                let dy = y_last - y_second_last;
                let len_sq = dx*dx + dy*dy;

                if len_sq > 0.01 { // Avoid drawing arrows on zero-length segments
                    // Calculate arrow base position slightly back from the end point
                    let len = len_sq.sqrt();
                    let arrow_offset = (arrow_size * 1.5).min(len * 0.4);

                    let arrow_base_x = x_last - dx * arrow_offset / len;
                    let arrow_base_y = y_last - dy * arrow_offset / len;

                    draw_arrow_head(&mut image, (arrow_base_x, arrow_base_y), (x_last, y_last), color, arrow_size, width);
                }
            }
        }
    }

    // Add nodes to image as circles if node_size > 0
    if node_size > 0 {
        for (i, &(lng, lat)) in world.node_positions.iter().enumerate() {
            if is_in_bounds(lng, lat) {
                let (x, y) = to_img_coords(lng, lat);
                draw_filled_circle_mut(&mut image, (x as i32, y as i32), node_size as i32, gray);

                if show_labels {
                    // Text rendering placeholder
                }
            }
        }
    }

    Ok(image)
}

/// Main function to create PNG visualization from graph data
/// Legacy function that maintains backwards compatibility
pub fn visualize_graph(graph: &GraphBlob, location: &LocationBlob, config: &VizConfig) -> StatusOr<RgbImage> {
    // Process world data
    let world_data = process_world_data(graph, location, None, config.max_size)?;
    
    // Render the tile/image using the processed data
    render_tile(&world_data, config, 0)
}
