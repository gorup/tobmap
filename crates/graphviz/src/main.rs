use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::ffi::OsStr;

use anyhow::{Context, Result, bail};
use clap::Parser;
use image::ImageFormat;
use schema::tobmapgraph::{GraphBlob, LocationBlob, DescriptionBlob};

// Import from the library crate
use graphviz::{visualize_graph, VizConfig, process_world_data, render_tile, WorldData};

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate PNG/JPG visualization of graph data")]
struct Args {
    /// Path to the input graph.fbs file
    #[arg(short = 'g', long)]
    graph: PathBuf,

    /// Path to the input location.fbs file
    #[arg(short = 'l', long)]
    location: PathBuf,

    /// Path to the optional description.fbs file (for road priorities)
    #[arg(short = 'd', long)]
    description: Option<PathBuf>,

    /// Path to the output image file (e.g., output.png or output.jpg)
    output: PathBuf, // Changed from #[arg(short, long)] to positional

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

    /// Index of an edge to highlight and log details for
    #[arg(long)]
    highlight_edge_index: Option<u32>,

    /// Width for the highlighted edge (defaults to edge_width * 2 if not set)
    #[arg(long)]
    highlight_edge_width: Option<f32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Determine output format from file extension
    let output_format = match args.output.extension().and_then(OsStr::to_str) {
        Some("png") => ImageFormat::Png,
        Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
        Some(ext) => bail!("Unsupported output format: {}. Please use .png or .jpg.", ext),
        None => bail!("Output file must have a .png or .jpg extension."),
    };

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

    // Read and parse the description file if provided
    let mut description_buffer = Vec::new();
    let mut description_option = None;
    
    if let Some(description_path) = &args.description {
        let mut description_file = File::open(description_path)
            .with_context(|| format!("Failed to open description file: {:?}", description_path))?;

        description_file.read_to_end(&mut description_buffer)
            .with_context(|| "Failed to read description file")?;

        // Parse description file
        let verifier_opts = flatbuffers::VerifierOptions {
            max_tables: 3_000_000_000, // 3 billion tables
            ..Default::default()
        };
        
        let description = flatbuffers::root_with_opts::<DescriptionBlob>(&verifier_opts, &description_buffer)
            .with_context(|| "Failed to parse description data from buffer")?;
            
        description_option = Some(description);
    }

    // Use get_root_with_opts instead of root for better error handling and custom verifier options
    let verifier_opts = flatbuffers::VerifierOptions {
        max_tables: 3_000_000_000, // 3 billion tables
        ..Default::default()
    };

    let graph = flatbuffers::root_with_opts::<GraphBlob>(&verifier_opts, &graph_buffer)
        .with_context(|| "Failed to parse graph data from buffer")?;

    let location = flatbuffers::root_with_opts::<LocationBlob>(&verifier_opts, &location_buffer)
        .with_context(|| "Failed to parse location data from buffer")?;

    // Create VizConfig from Args
    let config = VizConfig {
        max_size: args.max_size,
        node_size: args.node_size,
        edge_width: args.edge_width,
        show_labels: args.show_labels,
        center_lat: args.center_lat,
        center_lng: args.center_lng,
        zoom_meters: args.zoom_meters,
        highlight_edge_index: args.highlight_edge_index,
        highlight_edge_width: args.highlight_edge_width,
        tile: None, // Not using tiling in this example
        description: description_option.as_ref(), // Pass the optional description data
    };

    println!("Processing world data...");
    // First process the world data (the optimization)
    let world_data = process_world_data(&graph, &location, description_option.as_ref(), args.max_size)
        .with_context(|| "Failed to process world data")?;
    println!("Processed {} nodes and {} edges", world_data.nodes_count, world_data.edges_count);
    
    // Then render the final image
    println!("Rendering image...");
    let image = render_tile(&world_data, &config)
        .with_context(|| "Failed to render visualization")?;

    // Save the image with the determined format
    println!("Saving image to {:?}...", args.output);
    image.save_with_format(&args.output, output_format)
        .with_context(|| format!("Failed to save image to {:?}", args.output))?;

    println!("Image visualization saved to {:?}", args.output);

    Ok(())
}
