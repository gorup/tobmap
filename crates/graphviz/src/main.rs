use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use flatbuffers::FlatBufferBuilder;
use s2::cellid::CellID;
use s2::latlng::LatLng;
use schema::tobmapgraph::{self, GraphBlob, Node, Edge};
use svg::Document;
use svg::node::element::{Circle, Line, Text};
use svg::node::Text as TextContent;
use thiserror::Error;

#[derive(Error, Debug)]
enum GraphVizError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Failed to parse graph data: {0}")]
    ParseError(String),
    
    #[error("Failed to generate SVG: {0}")]
    SvgError(String),
}

type StatusOr<T> = Result<T, GraphVizError>;

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate SVG visualization of graph data")]
struct Args {
    /// Path to the input graph.fbs file
    #[arg(short, long)]
    input: PathBuf,
    
    /// Path to the output SVG file
    #[arg(short, long)]
    output: PathBuf,
    
    /// Maximum width/height of the SVG in pixels (will use smaller of width/height)
    #[arg(short, long, default_value_t = 8000)]
    max_size: u32,
    
    /// Node size in the visualization
    #[arg(long, default_value_t = 2.0)]
    node_size: f32,
    
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

/// Main function to create SVG visualization from graph data
fn visualize_graph(graph: &GraphBlob, args: &Args) -> StatusOr<Document> {
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
    let (svg_width, svg_height) = if aspect_ratio > 1.0 {
        (args.max_size as f64, (args.max_size as f64) / aspect_ratio)
    } else {
        ((args.max_size as f64) * aspect_ratio, args.max_size as f64)
    };
    
    // Create SVG document
    let mut document = Document::new()
        .set("width", svg_width)
        .set("height", svg_height)
        .set("viewBox", format!("0 0 {} {}", svg_width, svg_height));
    
    // Helper function to convert lat/lng to SVG coordinates
    let to_svg_coords = |lng: f64, lat: f64| -> (f64, f64) {
        let x = (lng - min_lng) / width * svg_width;
        // Note: y-axis is inverted in SVG (0 at top)
        let y = (max_lat - lat) / height * svg_height;
        (x, y)
    };
    
    // Add edges to SVG
    for i in 0..edges.len() {
        let edge = edges.get(i);
        let node1_idx = edge.point_1_node_idx() as usize;
        let node2_idx = edge.point_2_node_idx() as usize;
        
        if node1_idx >= node_positions.len() || node2_idx >= node_positions.len() {
            eprintln!("Warning: Edge references non-existent node");
            continue;
        }
        
        let (x1, y1) = to_svg_coords(node_positions[node1_idx].0, node_positions[node1_idx].1);
        let (x2, y2) = to_svg_coords(node_positions[node2_idx].0, node_positions[node2_idx].1);
        
        let line = Line::new()
            .set("x1", x1)
            .set("y1", y1)
            .set("x2", x2)
            .set("y2", y2)
            .set("stroke", "black")
            .set("stroke-width", args.edge_width);
        
        document = document.add(line);
    }
    
    // Add nodes to SVG
    for (i, (lng, lat)) in node_positions.iter().enumerate() {
        let (x, y) = to_svg_coords(*lng, *lat);
        
        let circle = Circle::new()
            .set("cx", x)
            .set("cy", y)
            .set("r", args.node_size)
            .set("fill", "red");
        
        document = document.add(circle);
        
        // Add labels if requested
        if args.show_labels {
            let text = Text::new()
                .set("x", x + args.node_size as f64 + 1.0)
                .set("y", y - args.node_size as f64 - 1.0)
                .set("font-size", "10")
                .set("text-anchor", "start")
                .add(TextContent::new(i.to_string()));
            
            document = document.add(text);
        }
    }
    
    Ok(document)
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Read and parse the graph file
    let mut input_file = File::open(&args.input)
        .with_context(|| format!("Failed to open file: {:?}", args.input))?;
    
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)
        .with_context(|| "Failed to read input file")?;
    
    // Use get_root instead of root for better error handling
    let graph = flatbuffers::root::<GraphBlob>(&buffer)
        .with_context(|| "Failed to parse graph data from buffer")?;
    
    // Generate the SVG visualization
    let document = visualize_graph(&graph, &args)
        .with_context(|| "Failed to generate SVG visualization")?;
    
    // Write the SVG to output file
    svg::save(&args.output, &document)
        .with_context(|| format!("Failed to save SVG to {:?}", args.output))?;
    
    println!("SVG visualization saved to {:?}", args.output);
    
    Ok(())
}
