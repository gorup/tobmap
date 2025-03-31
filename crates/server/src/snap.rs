use tonic::{transport::Server, Request, Response, Status};

use tobmapapi::snap_service_server::{SnapService, SnapServiceServer};
use tobmapapi::{SnapRequest, SnapResponse};

pub mod tobmapapi {
    tonic::include_proto!("tobmapapi");
}

#[derive(Debug, Default)]
pub struct MySnapService {}

#[tonic::async_trait]
impl SnapService for MySnapService {
    async fn get_snap(
        &self,
        request: Request<SnapRequest>, // Accept request of type SnapRequest
    ) -> Result<Response<SnapResponse>, Status> { // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);


        let req = request.into_inner();

        let reply = SnapResponse {
            edge_index: 1,
            lat: req.lat,
            lng: req.lng,
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}