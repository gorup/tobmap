use graphbuild::{osm_to_graph_blob, get_graph_blob, get_location_blob, get_description_blob};
use std::env;
use std::path::PathBuf;
use std::fs;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::new().filter_level(log::LevelFilter::Debug).init();
    let mut args = std::env::args().skip(1);
    
    if args.len() < 2 {
        eprintln!("Usage: graphbuild <input_osm_file> <output_graph_file> [output_location_file] [output_description_file]");
        std::process::exit(1);
    }
    
    let input_file = args.next().unwrap();
    let output_graph_file = args.next().unwrap();
    let output_location_file = args.next().unwrap_or_else(|| {
        // If no location file is specified, derive it from the graph file
        let mut location_path = PathBuf::from(&output_graph_file);
        location_path.set_extension("location.fb");
        location_path.to_string_lossy().to_string()
    });
    let output_description_file = args.next().unwrap_or_else(|| {
        // If no description file is specified, derive it from the graph file
        let mut desc_path = PathBuf::from(&output_graph_file);
        desc_path.set_extension("description.fb");
        desc_path.to_string_lossy().to_string()
    });
    
    info!("Reading OSM data from {}", input_file);
    let osm_data = fs::read(&input_file)?;
    
    info!("Building graph...");
    let (graph_data, location_data, description_data) = osm_to_graph_blob(&osm_data)?;
    
    info!("Writing graph blob to {}", output_graph_file);
    fs::write(&output_graph_file, graph_data)?;
    
    info!("Writing location blob to {}", output_location_file);
    fs::write(&output_location_file, location_data)?;
    
    info!("Writing description blob to {}", output_description_file);
    fs::write(&output_description_file, description_data)?;
    
    Ok(())
}

