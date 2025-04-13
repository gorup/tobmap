use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::f64::consts::PI;

use anyhow::{Context, Result};
use clap::Parser;
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_line_segment_mut, draw_cross_mut, draw_filled_circle_mut};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use schema::tobmapgraph::{GraphBlob, LocationBlob};
use thiserror::Error;

#[derive(Error, Debug)]
enum GraphVizError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse graph data: {0}")]
    ParseError(String),
    
    #[error("Failed to generate image: {0}")]
    ImageError(String),
}

type StatusOr<T> = Result<T, GraphVizError>;

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate PNG visualization of graph data")]
struct Args {
    /// Path to the input graph.fbs file
    #[arg(short = 'g', long)]
    graph: PathBuf,
    
    /// Path to the input location.fbs file
    #[arg(short = 'l', long)]
    location: PathBuf,
    
    /// Path to the output PNG file
    #[arg(short, long)]
    output: PathBuf,
    
    /// Maximum width/height of the image in pixels (will use smaller of width/height)
    #[arg(short, long, default_value_t = 12000)]
    max_size: u32,
    
    /// Node size in the visualization
    #[arg(long, default_value_t = 0)]
    node_size: u32,
    
    /// Edge width in the visualization
    #[arg(long, default_value_t = 1.0)]
    edge_width: f32,
    
    /// Show node indices as labels
    #[arg(long, default_value_t = false)]
    show_labels: bool,

    /// Latitude of the center point for zoomed view
    #[arg(long)]
    center_lat: Option<f64>,

    /// Longitude of the center point for zoomed view
    #[arg(long)]
    center_lng: Option<f64>,

    /// Width/Height of the zoomed view in meters
    #[arg(long)]
    zoom_meters: Option<f64>,
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

/// Main function to create PNG visualization from graph data
fn visualize_graph(graph: &GraphBlob, location: &LocationBlob, args: &Args) -> StatusOr<RgbImage> {
    // Extract all nodes and edges
    let nodes = graph.nodes().ok_or_else(|| GraphVizError::ParseError("Failed to get nodes".to_string()))?;
    let edges = graph.edges().ok_or_else(|| GraphVizError::ParseError("Failed to get edges".to_string()))?;
    
    // Get node and edge locations
    let node_locations = location.node_location_items().ok_or_else(|| 
        GraphVizError::ParseError("Failed to get node locations".to_string()))?;
    let edge_locations = location.edge_location_items().ok_or_else(|| 
        GraphVizError::ParseError("Failed to get edge locations".to_string()))?;
    
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
    
    // Calculate bounds of the map
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lng = f64::MAX;
    let mut max_lng = f64::MIN;
    
    // Store all node positions first
    let node_positions: Vec<(f64, f64)> = (0..node_locations.len())
        .map(|i| {
            let node_location = node_locations.get(i);
            let latlng = cell_id_to_latlng(node_location.cell_id());
            (latlng.lng.deg(), latlng.lat.deg()) // x = longitude, y = latitude
        })
        .collect();

    // Determine map bounds: either from zoom args or from all nodes
    if let (Some(center_lat), Some(center_lng), Some(zoom_meters)) = (args.center_lat, args.center_lng, args.zoom_meters) {
        // Calculate bounds based on center and zoom
        let meters_per_lng = meters_per_degree_lng(center_lat);
        if meters_per_lng <= 0.0 { // Avoid division by zero near poles
             return Err(GraphVizError::ImageError("Cannot calculate longitude span near poles.".to_string()));
        }
        let delta_lat = (zoom_meters / 2.0) / METERS_PER_DEGREE_LAT;
        let delta_lng = (zoom_meters / 2.0) / meters_per_lng;

        min_lat = center_lat - delta_lat;
        max_lat = center_lat + delta_lat;
        min_lng = center_lng - delta_lng;
        max_lng = center_lng + delta_lng;

    } else {
        // Calculate bounds based on all nodes (existing behavior)
        for &(lng, lat) in &node_positions {
            min_lat = min_lat.min(lat);
            max_lat = max_lat.max(lat);
            min_lng = min_lng.min(lng);
            max_lng = max_lng.max(lng);
        }
    }
    
    // Determine map dimensions (geographic range)
    let width = max_lng - min_lng;
    let height = max_lat - min_lat;

    // Handle cases where bounds are invalid (e.g., single point, zoom near pole failed)
    if width <= 0.0 || height <= 0.0 {
        return Err(GraphVizError::ImageError(format!(
            "Invalid map bounds calculated: width={}, height={}. Ensure valid zoom or sufficient node spread.",
            width, height
        )));
    }
    
    // Ensure aspect ratio is preserved for the image canvas
    let aspect_ratio = width / height;
    let (img_width, img_height) = if aspect_ratio > 1.0 {
        (args.max_size, (args.max_size as f64 / aspect_ratio) as u32)
    } else {
        ((args.max_size as f64 * aspect_ratio) as u32, args.max_size)
    };
    
    // Create an empty white image
    let mut image = RgbImage::new(img_width, img_height);
    let white = Rgb([255, 255, 255]);
    let gray = Rgb([128, 128, 128]);
    
    // Fill with white
    for pixel in image.pixels_mut() {
        *pixel = white;
    }
    
    // Helper function to convert lat/lng to image coordinates
    let to_img_coords = |lng: f64, lat: f64| -> (f32, f32) {
        let x = (lng - min_lng) / width * img_width as f64;
        // Note: y-axis is inverted (0 at top)
        let y = (max_lat - lat) / height * img_height as f64;
        (x as f32, y as f32)
    };

    // Helper to check if a point is within bounds
    let is_in_bounds = |lng: f64, lat: f64| -> bool {
        lng >= min_lng && lng <= max_lng && lat >= min_lat && lat <= max_lat
    };
    
    // Arrow size for direction indicators (relative to edge width)
    let arrow_size = 6.0 * args.edge_width.max(1.0); // Ensure arrow size scales reasonably even for thin lines
    
    // Add edges to image
    for i in 0..edges.len() {
        let edge = edges.get(i);
        let edge_location = edge_locations.get(i); // Get corresponding edge location data

        let node1_idx = edge.point_1_node_idx() as usize;
        let node2_idx = edge.point_2_node_idx() as usize;
        
        if node1_idx >= node_positions.len() || node2_idx >= node_positions.len() {
            eprintln!("Warning: Edge {} references non-existent node index {} or {}", i, node1_idx, node2_idx);
            continue;
        }
        
        let (lng1, lat1) = node_positions[node1_idx];
        let (lng2, lat2) = node_positions[node2_idx];

        // Calculate overall edge properties (color based on total distance/time)
        let costs_and_flags = edge.costs_and_flags();
        let backwards_allowed = (costs_and_flags & 0b0000_0000_0000_0001) != 0;
        let time_seconds: u16 = (costs_and_flags >> 2) as u16;
        let distance_meters = haversine_distance(lat1, lng1, lat2, lng2);
        let edge_color = get_speed_color(distance_meters, time_seconds);

        // Construct the full path for the edge
        let mut path_coords: Vec<(f64, f64)> = Vec::new();
        path_coords.push((lng1, lat1)); // Start node

        if let Some(cell_ids) = edge_location.points() {
            if cell_ids.len() > 0 {
                for cell_id in cell_ids {
                    let latlng = cell_id_to_latlng(cell_id);
                    path_coords.push((latlng.lng.deg(), latlng.lat.deg()));
                }
            }
        }
        
        path_coords.push((lng2, lat2)); // End node

        // Draw segments of the path
        let mut last_img_coords: Option<(f32, f32)> = None;
        for j in 0..path_coords.len() - 1 {
            let (p1_lng, p1_lat) = path_coords[j];
            let (p2_lng, p2_lat) = path_coords[j+1];

            // Check if segment is potentially visible
            let p1_in_bounds = is_in_bounds(p1_lng, p1_lat);
            let p2_in_bounds = is_in_bounds(p2_lng, p2_lat);

            // Simple visibility check: draw if either point is in bounds or line crosses bounds
            // A more robust check would involve line clipping, but this is often sufficient
            if p1_in_bounds || p2_in_bounds || line_crosses_bounds(p1_lng, p1_lat, p2_lng, p2_lat, min_lng, min_lat, max_lng, max_lat) {
                let (x1, y1) = to_img_coords(p1_lng, p1_lat);
                let (x2, y2) = to_img_coords(p2_lng, p2_lat);

                draw_thick_line_segment_mut(&mut image, (x1, y1), (x2, y2), edge_color, args.edge_width);
                last_img_coords = Some((x2, y2)); // Store the end coords of the last drawn segment
            } else {
                 // If segment is entirely out of bounds, reset last_img_coords for arrow drawing logic
                 // This prevents drawing arrows for edges completely outside the view.
                 // However, we need the *previous* point if the *last* segment ends in bounds.
                 // Let's refine this: we need the image coords of the last *two* points of the path.
            }
        }

        // Draw arrow head for one-way edges at the end of the last segment
        if !backwards_allowed && path_coords.len() >= 2 {
            let (p_last_lng, p_last_lat) = path_coords[path_coords.len() - 1];
            let (p_second_last_lng, p_second_last_lat) = path_coords[path_coords.len() - 2];

            // Check if the end point is within bounds before drawing arrow
            if is_in_bounds(p_last_lng, p_last_lat) || is_in_bounds(p_second_last_lng, p_second_last_lat) {
                let (x_last, y_last) = to_img_coords(p_last_lng, p_last_lat);
                let (x_second_last, y_second_last) = to_img_coords(p_second_last_lng, p_second_last_lat);

                let dx = x_last - x_second_last;
                let dy = y_last - y_second_last;
                let len_sq = dx*dx + dy*dy;

                if len_sq > 0.01 { // Avoid drawing arrows on zero-length segments
                    // Calculate arrow base position slightly back from the end point along the last segment
                    let len = len_sq.sqrt();
                    let arrow_offset = (arrow_size * 1.5).min(len * 0.4); // Place arrow base back from the end point

                    let arrow_base_x = x_last - dx * arrow_offset / len;
                    let arrow_base_y = y_last - dy * arrow_offset / len;

                    draw_arrow_head(&mut image, (arrow_base_x, arrow_base_y), (x_last, y_last), edge_color, arrow_size, args.edge_width);
                }
            }
        }
    }
    
    // Add nodes to image as circles
    for (i, &(lng, lat)) in node_positions.iter().enumerate() {
        if is_in_bounds(lng, lat) { // Use helper function
            let (x, y) = to_img_coords(lng, lat);
            
            draw_filled_circle_mut(&mut image, (x as i32, y as i32), args.node_size as i32, gray);
            
            if args.show_labels {
                // Text rendering in image is more complex and would require additional libraries
                // This is a placeholder for a text rendering implementation
            }
        }
    }
    
    Ok(image)
}

/// Simple line-rectangle intersection check (Axis-Aligned Bounding Box)
/// Returns true if the line segment (p1 -> p2) potentially intersects or is inside the box.
/// This is a basic check and not a full clipping algorithm.
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

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Read and parse the graph file
    let mut graph_file = File::open(&args.graph)
        .with_context(|| format!("Failed to open graph file: {:?}", args.graph))?;
    
    let mut graph_buffer = Vec::new();
    graph_file.read_to_end(&mut graph_buffer)
        .with_context(|| "Failed to read graph file")?;
    
    // Read and parse the location file
    let mut location_file = File::open(&args.location)
        .with_context(|| format!("Failed to open location file: {:?}", args.location))?;
    
    let mut location_buffer = Vec::new();
    location_file.read_to_end(&mut location_buffer)
        .with_context(|| "Failed to read location file")?;
    
    // Use get_root_with_opts instead of root for better error handling and custom verifier options
    let verifier_opts = flatbuffers::VerifierOptions {
        max_tables: 3_000_000_000, // 3 billion tables
        ..Default::default()
    };
    
    let graph = flatbuffers::root_with_opts::<GraphBlob>(&verifier_opts, &graph_buffer)
        .with_context(|| "Failed to parse graph data from buffer")?;
        
    let location = flatbuffers::root_with_opts::<LocationBlob>(&verifier_opts, &location_buffer)
        .with_context(|| "Failed to parse location data from buffer")?;
    
    // Generate the PNG visualization
    let image = visualize_graph(&graph, &location, &args)
        .with_context(|| "Failed to generate PNG visualization")?;
    
    // Save the image
    image.save(&args.output)
        .with_context(|| format!("Failed to save PNG to {:?}", args.output))?;
    
    println!("PNG visualization saved to {:?}", args.output);
    
    Ok(())
}
