use std::path::PathBuf;
use structopt::StructOpt;
use snapbuild::Config;

#[derive(Debug, StructOpt)]
#[structopt(name = "snapbuild", about = "Generate SnapBuckets files from graph and location data")]
struct Opt {
    /// Outer cell level for organizing SnapBuckets files
    #[structopt(short = "o", long = "outer-level", default_value = "4")]
    outer_cell_level: u8,

    /// Inner cell level for organizing edges within SnapBuckets
    #[structopt(short = "i", long = "inner-level", default_value = "8")]
    inner_cell_level: u8,

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
        outer_cell_level: opt.outer_cell_level,
        inner_cell_level: opt.inner_cell_level,
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

