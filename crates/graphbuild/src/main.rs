use graphbuild::osm_to_graph_blob;
use std::env;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <osm-pbf-file> [output-file]", args[0]);
        std::process::exit(1);
    }
    
    let osm_path = &args[1];
    let output_path = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        // Create default output filename by replacing the extension with .graph
        let input_path = PathBuf::from(osm_path);
        let stem = input_path.file_stem().unwrap_or_default();
        let mut output = PathBuf::from(input_path.parent().unwrap_or_else(|| Path::new(".")));
        output.push(stem);
        output.set_extension("graph");
        output
    };
    
    println!("Processing OSM file: {}", osm_path);
    
    match osm_to_graph_blob(Path::new(osm_path)) {
        Ok(graph_blob) => {
            println!("Successfully generated graph blob with {} bytes", graph_blob.len());
            
            // Write to output file
            match File::create(&output_path) {
                Ok(mut file) => {
                    match file.write_all(&graph_blob) {
                        Ok(_) => println!("Graph blob written to {}", output_path.display()),
                        Err(e) => {
                            eprintln!("Error writing to file: {}", e);
                            std::process::exit(1);
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Error creating output file: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Err(e) => {
            eprintln!("Error processing OSM file: {}", e);
            std::process::exit(1);
        }
    }
}

