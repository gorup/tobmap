use tonic::{transport::Server, Request, Response, Status};

use tobmaprouteapi::route_service_server::{RouteService, RouteServiceServer};
use tobmaprouteapi::{RouteRequest, RouteResponse};

pub mod tobmaprouteapi {
    tonic::include_proto!("tobmaprouteapi");
}

#[derive(Debug, Default)]
pub struct MyRouteService {}

#[tonic::async_trait]
impl RouteService for MyRouteService {
    async fn route(
        &self,
        request: Request<RouteRequest>, // Accept request of type RouteRequest
    ) -> Result<Response<RouteResponse>, Status> { // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);

        let req = request.into_inner();

        let reply = RouteResponse {
            paths: vec![],
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}