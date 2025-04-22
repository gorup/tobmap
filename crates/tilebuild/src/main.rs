use anyhow::{Result, Context};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use clap::Parser;
use tilebuild::{TileBuilder, TileBuildConfig};
use schema::tobmapgraph::{GraphBlob, LocationBlob};

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
    #[clap(short, long, default_value_t = 3)]
    max_zoom_level: u32,
    
    /// Tile size in pixels (longest edge)
    #[clap(long, default_value_t = 256)]
    tile_size: u32,
    
    /// Overlap between tiles in pixels
    #[clap(long, default_value_t = 8)]
    tile_overlap: u32,
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    
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
    
    // Set up configuration
    let config = TileBuildConfig {
        output_dir: opt.output_dir.clone(),
        max_zoom_level: opt.max_zoom_level,
        tile_size: opt.tile_size,
        tile_overlap: opt.tile_overlap,
        viz_config: graphviz::VizConfig {
            max_size: opt.tile_size,
            node_size: 0,
            edge_width: 0.0,
            show_labels: false,
            center_lat: None,
            center_lng: None,
            zoom_meters: None,
            highlight_edge_index: None,
            highlight_edge_width: None,
            tile: None,
            description: None,
        },
    };
    
    // Generate tiles
    let tile_builder = TileBuilder::new(config);
    println!("Generating tiles in {:?}...", opt.output_dir);
    println!("This may take a while but will be faster with our optimized approach!");
    tile_builder.build_all_tiles(&graph, &location)?;
    
    println!("Done!");
    Ok(())
}
