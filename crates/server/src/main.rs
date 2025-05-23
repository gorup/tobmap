mod snap;
mod route;

use clap::Parser;
use route::MyRouteService;
use snap::MySnapService;
use snap::tobmapapi::snap_service_server::SnapServiceServer;
use route::tobmaprouteapi::route_service_server::RouteServiceServer;
use tonic::transport::Server;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about = "TobMap Snap Service")]
struct Args {
    /// Directory containing snapbucket files
    #[clap(short, long)]
    snapbuckets_dir: PathBuf,

    /// Path to the graph blob file
    #[clap(short, long)]
    graph_path: PathBuf,

    /// Outer cell level for S2 cells
    #[clap(short, long, default_value = "4")]
    outer_cell_level: u8,

    /// Inner cell level for S2 cells
    #[clap(short, long, default_value = "8")]
    inner_cell_level: u8,

    /// Server address to listen on
    #[clap(short, long, default_value = "[::1]:50051")]
    address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    env_logger::Builder::new().filter_level(log::LevelFilter::Debug).init();
    
    let addr = args.address.parse()?;

    // Initialize route service with graph data
    let route_service = match MyRouteService::new(&args.graph_path) {
        Ok(service) => service,
        Err(e) => {
            eprintln!("Failed to load graph data: {}", e);
            MyRouteService::default()
        }
    };

    let snap_service = MySnapService::new(
        args.snapbuckets_dir.clone(),
        args.outer_cell_level,
        args.inner_cell_level
    ).map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;

    println!("Starting server on {}", args.address);
    println!("Using snapbuckets directory: {:?}", args.snapbuckets_dir);
    println!("Using graph data from: {:?}", args.graph_path);
    println!("Outer cell level: {}, Inner cell level: {}", args.outer_cell_level, args.inner_cell_level);

    Server::builder()
    .add_service(SnapServiceServer::new(snap_service))
    .add_service(RouteServiceServer::new(route_service))
    .serve(addr)
        .await?;

    Ok(())
}