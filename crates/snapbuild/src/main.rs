use std::path::PathBuf;
use structopt::StructOpt;
use snapbuild::Config;

#[derive(Debug, StructOpt)]
#[structopt(name = "snapbuild", about = "Generate SnapBuckets files from graph and location data")]
struct Opt {
    /// Cell level for truncating cell IDs
    #[structopt(short, long, default_value = "4")]
    cell_level: u8,

    /// Path to the graph blob file
    #[structopt(short, long, default_value = "graph.bin")]
    graph: PathBuf,

    /// Path to the location blob file
    #[structopt(short, long, default_value = "location.bin")]
    location: PathBuf,

    /// Output directory for generated SnapBuckets files
    #[structopt(short, long, default_value = "outputs/snapbuckets")]
    output: PathBuf,
}

fn main() {
    // Parse command line arguments
    let opt = Opt::from_args();
    
    // Create config from command line arguments
    let config = Config {
        cell_level: opt.cell_level,
        graph_path: opt.graph,
        location_path: opt.location,
        output_dir: opt.output,
    };
    
    // Process the data
    match snapbuild::process(&config) {
        Ok(_) => println!("SnapBuckets generated successfully!"),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

