use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::info;
use std::path::PathBuf;

mod cache;
mod download;
mod model;
// Import the generated FlatBuffers code
mod generated;

use cache::Cache;
use download::{Downloader, OsmSource};
use model::processor::process_osm_file;
use model::flatbuffer::{write_to_file, read_from_file, parse_flatbuffer};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the cache directory
    #[arg(short, long, default_value = ".cache")]
    cache_dir: String,
    
    /// Path to the output directory
    #[arg(short, long, default_value = "output")]
    output_dir: String,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download and process OpenStreetMap data
    #[command(subcommand)]
    Download(DownloadCommands),
    
    /// Process an existing OpenStreetMap PBF file
    Process {
        /// Path to the input OSM PBF file
        #[arg(short, long)]
        input: String,
        
        /// Output filename for the processed data
        #[arg(short, long, default_value = "map.fb")]
        output: String,
    },
    
    /// Clear the cache
    ClearCache,
}

#[derive(Subcommand)]
enum DownloadCommands {
    /// Download data from the entire planet
    Planet {
        /// Output filename for the processed data
        #[arg(short, long, default_value = "map.fb")]
        output: String,
    },
    
    /// Download data for a specific country
    Country {
        /// Name of the country
        #[arg(short, long)]
        name: String,
        
        /// Output filename for the processed data
        #[arg(short, long, default_value = "map.fb")]
        output: String,
    },
    
    /// Download data for a specific region within a country
    Region {
        /// Name of the country
        #[arg(short, long)]
        country: String,
        
        /// Name of the region
        #[arg(short, long)]
        name: String,
        
        /// Output filename for the processed data
        #[arg(short, long, default_value = "map.fb")]
        output: String,
    },
    
    /// Download data for a specific state
    State {
        /// Name of the state
        #[arg(short, long)]
        name: String,
        
        /// Output filename for the processed data
        #[arg(short, long, default_value = "map.fb")]
        output: String,
    },
    
    /// Download data from a custom URL
    Url {
        /// URL to download from
        #[arg(short, long)]
        url: String,
        
        /// Output filename for the processed data
        #[arg(short, long, default_value = "map.fb")]
        output: String,
    },
    
    /// Process a local OSM PBF file
    File {
        /// Path to the local file
        #[arg(short, long)]
        path: String,
        
        /// Output filename for the processed data
        #[arg(short, long, default_value = "map.fb")]
        output: String,
    },
}

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Create cache directory
    let cache = Cache::new(&cli.cache_dir)
        .context("Failed to create cache")?;
    
    // Create output directory
    std::fs::create_dir_all(&cli.output_dir)
        .context("Failed to create output directory")?;
    
    // Process command
    match &cli.command {
        Commands::Download(download_command) => {
            // Create the downloader
            let downloader = Downloader::new(cache);
            
            // Process the download command
            let (osm_source, output) = match download_command {
                DownloadCommands::Planet { output } => {
                    (OsmSource::Planet, output)
                },
                DownloadCommands::Country { name, output } => {
                    (OsmSource::Country(name.clone()), output)
                },
                DownloadCommands::Region { country, name, output } => {
                    (OsmSource::Region(country.clone(), name.clone()), output)
                },
                DownloadCommands::State { name, output } => {
                    (OsmSource::State(name.clone()), output)
                },
                DownloadCommands::Url { url, output } => {
                    (OsmSource::CustomUrl(url.clone()), output)
                },
                DownloadCommands::File { path, output } => {
                    (OsmSource::LocalFile(path.clone()), output)
                },
            };
            
            // Download the data
            let osm_file = downloader.download(osm_source)
                .context("Failed to download OSM data")?;
            
            // Process the data
            info!("Processing OSM data from {}", osm_file);
            let map_data = process_osm_file(osm_file)
                .context("Failed to process OSM data")?;
            
            // Write the data to a file
            let output_path = PathBuf::from(&cli.output_dir).join(output);
            info!("Writing processed data to {}", output_path.display());
            write_to_file(&map_data, output_path)
                .context("Failed to write processed data to file")?;
            
            info!("Done");
        },
        
        Commands::Process { input, output } => {
            // Process the data
            info!("Processing OSM data from {}", input);
            let map_data = process_osm_file(input)
                .context("Failed to process OSM data")?;
            
            // Write the data to a file
            let output_path = PathBuf::from(&cli.output_dir).join(output);
            info!("Writing processed data to {}", output_path.display());
            write_to_file(&map_data, output_path)
                .context("Failed to write processed data to file")?;
            
            info!("Done");
        },
        
        Commands::ClearCache => {
            info!("Clearing cache");
            cache.clear()
                .context("Failed to clear cache")?;
            info!("Cache cleared");
        },
    }
    
    Ok(())
}
