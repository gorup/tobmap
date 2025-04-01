use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_line_segment_mut, draw_cross_mut};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use schema::tobmapgraph::GraphBlob;
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
    #[arg(short, long)]
    input: PathBuf,
    
    /// Path to the output PNG file
    #[arg(short, long)]
    output: PathBuf,
    
    /// Maximum width/height of the image in pixels (will use smaller of width/height)
    #[arg(short, long, default_value_t = 12000)]
    max_size: u32,
    
    /// Node size in the visualization
    #[arg(long, default_value_t = 2)]
    node_size: u32,
    
    /// Edge width in the visualization
    #[arg(long, default_value_t = 1.0)]
    edge_width: f32,
    
    /// Show node indices as labels
    #[arg(long, default_value_t = false)]
    show_labels: bool,
}

/// Converts S2 CellID to lat/lng
fn cell_id_to_latlng(cell_id: u64) -> LatLng {
    let cell = CellID(cell_id);
    LatLng::from(cell)
}

/// Main function to create PNG visualization from graph data
fn visualize_graph(graph: &GraphBlob, args: &Args) -> StatusOr<RgbImage> {
    // Extract all nodes and edges
    let nodes = graph.nodes().ok_or_else(|| GraphVizError::ParseError("Failed to get nodes".to_string()))?;
    let edges = graph.edges().ok_or_else(|| GraphVizError::ParseError("Failed to get edges".to_string()))?;
    
    // Calculate bounds of the map
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lng = f64::MAX;
    let mut max_lng = f64::MIN;
    
    // Process all nodes to find map bounds
    let node_positions: Vec<(f64, f64)> = (0..nodes.len())
        .map(|i| {
            let node = nodes.get(i);
            let latlng = cell_id_to_latlng(node.cell_id());
            let lat = latlng.lat.deg();
            let lng = latlng.lng.deg();
            
            min_lat = min_lat.min(lat);
            max_lat = max_lat.max(lat);
            min_lng = min_lng.min(lng);
            max_lng = max_lng.max(lng);
            
            (lng, lat) // x = longitude, y = latitude
        })
        .collect();
    
    // Determine map dimensions
    let width = max_lng - min_lng;
    let height = max_lat - min_lat;
    
    // Ensure aspect ratio is preserved
    let aspect_ratio = width / height;
    let (img_width, img_height) = if aspect_ratio > 1.0 {
        (args.max_size, (args.max_size as f64 / aspect_ratio) as u32)
    } else {
        ((args.max_size as f64 * aspect_ratio) as u32, args.max_size)
    };
    
    // Create an empty white image
    let mut image = RgbImage::new(img_width, img_height);
    let white = Rgb([255, 255, 255]);
    let black = Rgb([0, 0, 0]);
    let green = Rgb([0, 153, 51]);
    
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
    
    // Add edges to image
    for i in 0..edges.len() {
        let edge = edges.get(i);
        let node1_idx = edge.point_1_node_idx() as usize;
        let node2_idx = edge.point_2_node_idx() as usize;
        
        if node1_idx >= node_positions.len() || node2_idx >= node_positions.len() {
            eprintln!("Warning: Edge references non-existent node");
            continue;
        }
        
        let (x1, y1) = to_img_coords(node_positions[node1_idx].0, node_positions[node1_idx].1);
        let (x2, y2) = to_img_coords(node_positions[node2_idx].0, node_positions[node2_idx].1);
        
        draw_line_segment_mut(&mut image, (x1, y1), (x2, y2), black);
    }
    
    // Add nodes to image as plus signs
    for (i, (lng, lat)) in node_positions.iter().enumerate() {
        let (x, y) = to_img_coords(*lng, *lat);
        
        // Draw plus sign (draw multiple crosses of different sizes for thicker plus)
        let size = args.node_size as i32;
        
        // Draw a cross
        draw_cross_mut(&mut image, green, x as i32, y as i32);
        
        // If size > 1, draw additional crosses to make it thicker
        if size > 1 {
            for offset in 1..size {
                // Draw horizontal extensions
                draw_cross_mut(&mut image, green, x as i32 + offset, y as i32);
                draw_cross_mut(&mut image, green, x as i32 - offset, y as i32);
                
                // Draw vertical extensions
                draw_cross_mut(&mut image, green, x as i32, y as i32 + offset);
                draw_cross_mut(&mut image, green, x as i32, y as i32 - offset);
            }
        }
        
        // Add labels if requested - omitted for now as drawing text requires more complex handling
        if args.show_labels {
            // Text rendering in image is more complex and would require additional libraries
            // Consider rusttype or other text rendering for images if needed
        }
    }
    
    Ok(image)
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Read and parse the graph file
    let mut input_file = File::open(&args.input)
        .with_context(|| format!("Failed to open file: {:?}", args.input))?;
    
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)
        .with_context(|| "Failed to read input file")?;
    
    // Use get_root_with_opts instead of root for better error handling and custom verifier options
    let verifier_opts = flatbuffers::VerifierOptions {
        max_tables: 3_000_000_000, // 3 billion tables
        ..Default::default()
    };
    let graph = flatbuffers::root_with_opts::<GraphBlob>(&verifier_opts, &buffer)
        .with_context(|| "Failed to parse graph data from buffer")?;
    
    // Generate the PNG visualization
    let image = visualize_graph(&graph, &args)
        .with_context(|| "Failed to generate PNG visualization")?;
    
    // Save the image
    image.save(&args.output)
        .with_context(|| format!("Failed to save PNG to {:?}", args.output))?;
    
    println!("PNG visualization saved to {:?}", args.output);
    
    Ok(())
}
