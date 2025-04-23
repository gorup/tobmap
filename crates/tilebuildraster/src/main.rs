use anyhow::{Result, Context};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use clap::Parser;
use log::{info, error};
use tilebuild::{TileBuilder, TileBuildConfig};
use schema::tobmapgraph::{GraphBlob, LocationBlob, DescriptionBlob};

#[derive(Parser, Debug)]
#[clap(name = "tilebuild", about = "Generate map tiles at different zoom levels")]
struct Opt {
    /// Path to graph.fbs file
    #[clap(short, long)]
    graph_file: PathBuf,

    /// Path to location.fbs file
    #[clap(short, long)]
    location_file: PathBuf,

    /// Output directory
    #[clap(short, long, default_value = "outputs/tiles")]
    output_dir: PathBuf,

    /// Maximum zoom level (0-based)
    #[clap(short, long, default_value_t = 5)]
    max_zoom_level: u32,
    
    /// Tile size in pixels (longest edge)
    #[clap(long, default_value_t = 512)]
    tile_size: u32,
    
    /// Overlap between tiles in pixels
    #[clap(long, default_value_t = 8)]
    tile_overlap: u32,
    
    /// Path to description file
    #[clap(short, long)]
    description_file: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    env_logger::Builder::new().filter_level(log::LevelFilter::Debug).init();
    
    println!("Reading graph data from {:?}...", opt.graph_file);
    let mut graph_buf = Vec::new();
    File::open(&opt.graph_file)
        .with_context(|| format!("Failed to open graph file: {:?}", opt.graph_file))?
        .read_to_end(&mut graph_buf)
        .with_context(|| format!("Failed to read graph file: {:?}", opt.graph_file))?;
    
    println!("Reading location data from {:?}...", opt.location_file);
    let mut location_buf = Vec::new();
    File::open(&opt.location_file)
        .with_context(|| format!("Failed to open location file: {:?}", opt.location_file))?
        .read_to_end(&mut location_buf)
        .with_context(|| format!("Failed to read location file: {:?}", opt.location_file))?;
    
    // Read description file if provided
    println!("Reading description data from {:?}...", opt.description_file);
    let mut description_buf = Vec::new();
    File::open(&opt.description_file)
        .with_context(|| format!("Failed to open description file: {:?}", opt.description_file))?
        .read_to_end(&mut description_buf)
        .with_context(|| format!("Failed to read description file: {:?}", opt.description_file))?;

    // Parse FlatBuffers
    // Use get_root_with_opts instead of root for better error handling and custom verifier options
    let verifier_opts = flatbuffers::VerifierOptions {
        max_tables: 3_000_000_000, // 3 billion tables
        ..Default::default()
    };

    let graph = flatbuffers::root_with_opts::<GraphBlob>(&verifier_opts, &graph_buf)
        .with_context(|| "Failed to parse graph data from buffer")?;

    let location = flatbuffers::root_with_opts::<LocationBlob>(&verifier_opts, &location_buf)
        .with_context(|| "Failed to parse location data from buffer")?;

    let description = flatbuffers::root_with_opts::<DescriptionBlob>(&verifier_opts, &description_buf)
        .with_context(|| "Failed to parse description data from buffer")?;
    
    // Set up render flags for each zoom level
    let max_zoom = opt.max_zoom_level;
    let mut show_vertices = vec![false; (max_zoom + 1) as usize];
    let mut min_priority = vec![0; (max_zoom + 1) as usize];
    
    // Configure zoom levels according to requirements
    // Show vertices only for zoom levels 3+
    for level in 0..=max_zoom {
        show_vertices[level as usize] = level >= 3;
    }
    
    // Set minimum priority thresholds for each level
    if max_zoom >= 0 { min_priority[0] = 8; }
    if max_zoom >= 1 { min_priority[1] = 6; }
    if max_zoom >= 2 { min_priority[2] = 4; }
    if max_zoom >= 3 { min_priority[3] = 0; }

    for (i, &priority) in min_priority.iter().enumerate() {
        println!("Zoom level {}: Minimum priority = {}", i, priority);
    }
    
    // Set up configuration
    let config = TileBuildConfig {
        output_dir: opt.output_dir.clone(),
        max_zoom_level: opt.max_zoom_level,
        tile_size: opt.tile_size,
        tile_overlap: opt.tile_overlap,
        show_vertices,
        min_priority,
        viz_config: graphviz::VizConfig {
            max_size: opt.tile_size,
            node_size: Some(0),
            edge_width: 0.0,
            show_labels: false,
            center_lat: None,
            center_lng: None,
            zoom_meters: None,
            highlight_edge_index: None,
            highlight_edge_width: None,
            tile: None,
        },
    };
    
    // Generate tiles
    let tile_builder = TileBuilder::new(config);
    println!("Generating tiles in {:?}...", opt.output_dir);
    println!("This may take a while but will be faster with our parallel processing approach!");
    tile_builder.build_all_tiles(&graph, &location, &description)?;
    
    println!("Done!");
    Ok(())
}
