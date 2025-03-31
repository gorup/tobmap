mod snap;

use snap::MySnapService;
use snap::tobmapapi::snap_service_server::{SnapService, SnapServiceServer};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let snap_service = MySnapService::default();

    Server::builder()
        .add_service(SnapServiceServer::new(snap_service))
        .serve(addr)
        .await?;

    Ok(())
}