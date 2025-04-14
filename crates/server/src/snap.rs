use tonic::{transport::Server, Request, Response, Status};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use s2::{cell::Cell, cellid::CellID, latlng::LatLng, point::Point};
use log::{info, warn};

use tobmapapi::snap_service_server::{SnapService, SnapServiceServer};
use tobmapapi::{SnapRequest, SnapResponse, SnapResponseDebugInfo};
use schema::snap_generated::tobmapsnap::{SnapBuckets, SnapBucket};

// Export the tobmapgraph module so it can be used by route.rs
pub use crate::schema::graph_generated::tobmapgraph;

pub mod tobmapapi {
    tonic::include_proto!("tobmapapi");
}

#[derive(Debug)]
pub struct MySnapService {
    // Map from outer cell ID to loaded SnapBuckets
    snap_buckets: HashMap<u64, Vec<u8>>,
    outer_cell_level: u8,
    inner_cell_level: u8,
}

impl Default for MySnapService {
    fn default() -> Self {
        Self::new("/workspaces/tobmap/snapbuckets", 4, 8).unwrap_or_else(|e| {
            eprintln!("Failed to initialize MySnapService with default parameters: {}", e);
            Self {
                snap_buckets: HashMap::new(),
                outer_cell_level: 4,
                inner_cell_level: 8,
            }
        })
    }
}

impl MySnapService {
    pub fn new(snapbuckets_dir: impl AsRef<Path>, outer_cell_level: u8, inner_cell_level: u8) -> Result<Self, String> {
        let mut snap_buckets = HashMap::new();
        
        // Read all snapbucket files from the directory
        let entries = fs::read_dir(snapbuckets_dir)
            .map_err(|e| format!("Failed to read snapbuckets directory: {}", e))?;
            
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && path.to_string_lossy().contains("snap_bucket_") {
                // Extract S2 token from filename
                let filename = path.file_name()
                    .ok_or_else(|| format!("Invalid filename: {:?}", path))?
                    .to_string_lossy();
                    
                if let Some(token_start) = filename.find("snap_bucket_") {
                    if let Some(token_end) = filename.find(".bin") {
                        let token = &filename[token_start + 12..token_end];
                        
                        // Convert token to cell ID
                        let cell_id = CellID::from_token(token);
                        
                        // Read the file content
                        let mut file = File::open(&path)
                            .map_err(|e| format!("Failed to open file {:?}: {}", path, e))?;
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer)
                            .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
                        
                        // Store the binary data with the cell ID as the key
                        snap_buckets.insert(cell_id.0, buffer);
                        println!("Loaded snapbucket for cell ID: {}, token: {}", cell_id.0, token);
                    }
                }
            }
        }
        
        println!("Loaded {} snapbucket files", snap_buckets.len());
        
        Ok(Self {
            snap_buckets,
            outer_cell_level,
            inner_cell_level,
        })
    }
    
    // Find the closest edge in a snap bucket to the given cell ID
    fn find_closest_edge(&self, snap_bucket: &SnapBucket, target_cell_id: u64) -> Option<(u32, u64)> {
        if let (Some(edge_cell_ids), Some(edge_indexes)) = (snap_bucket.edge_cell_ids(), snap_bucket.edge_indexes()) {
            if edge_cell_ids.len() == 0 {
                return None;
            }

            info!("num edges and indexes we'll look thru {} {}", edge_cell_ids.len(), edge_indexes.len());
            
            // Create S2 Cell for target position to calculate geographic distance
            let target_s2_cell = CellID(target_cell_id);
            let target_center = Cell::from(target_s2_cell).center();
            
            let mut closest_index = 0;
            let mut closest_cell_id = edge_cell_ids.get(0);
            let mut min_distance = s2::s1::Angle::inf();
            
            // Iterate through all edges and find the closest one geographically
            for i in 0..edge_cell_ids.len() {
                let cell_id = edge_cell_ids.get(i);
                let s2_cell = CellID(cell_id);
                let cell_center = Cell::from(s2_cell).center();
                
                // Calculate distance between points using the distance method
                let dist = target_center.distance(&cell_center);
                
                // info!("Cell id {}, distance {:?}", s2_cell.to_token(), dist);

                if dist < min_distance {
                    min_distance = dist;
                    info!("Found closer edge: {} (distance: {:?})", s2_cell.to_token(), dist);
                    closest_index = i;
                    closest_cell_id = cell_id;
                }
            }
            
            return Some((edge_indexes.get(closest_index), closest_cell_id));
        }
        
        None
    }
}

#[tonic::async_trait]
impl SnapService for MySnapService {
    async fn get_snap(
        &self,
        request: Request<SnapRequest>,
    ) -> Result<Response<SnapResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();
        
        // Convert lat/lng to S2 cell
        let lat_lng = LatLng::from_degrees(req.lat, req.lng);
        let cell_id = CellID::from(lat_lng);

        info!("Received request for lat: {}, lng: {}, converted to cell ID: {}", req.lat, req.lng, cell_id.0);
        
        // Get the outer cell ID for the requested location
        let outer_cell_id = cell_id.parent(self.outer_cell_level as u64).0;
        
        // Get the inner cell ID for the requested location
        let inner_cell_id = cell_id.parent(self.inner_cell_level as u64).0;
        
        info!("Outer cell ID: {}, Inner cell ID: {}", outer_cell_id, inner_cell_id);

        // Debug info
        // let mut debug_info = SnapResponseDebugInfo {
        //     outer_cell_id,
        //     inner_cell_id,
        //     target_cell_id: cell_id.0,
        //     found_outer_cell: false,
        //     found_inner_cell: false,
        //     edges_in_bucket: 0,
        // };
        
        // Try to find the correct outer bucket
        if let Some(bucket_data) = self.snap_buckets.get(&outer_cell_id) {
            // debug_info.found_outer_cell = true;
            
            // Parse the flatbuffer
            match flatbuffers::root::<SnapBuckets>(&bucket_data) {
                Ok(snap_buckets) => {
                    if let Some(buckets) = snap_buckets.snap_buckets() {
                        // Find the bucket for the inner cell
                        for i in 0..buckets.len() {
                            let snap_bucket = buckets.get(i);
                            if snap_bucket.cell_id() == inner_cell_id {
                                info!("Found snap bucket, {}", snap_bucket.cell_id());
                                // debug_info.found_inner_cell = true;
                                
                                // Set the number of edges in this bucket
                                // if let Some(edge_cell_ids) = snap_bucket.edge_cell_ids() {
                                //     debug_info.edges_in_bucket = edge_cell_ids.len() as u32;
                                // }
                                
                                // Find the closest edge in the bucket
                                if let Some((edge_index, edge_cell_id)) = self.find_closest_edge(&snap_bucket, cell_id.0) {
                                    // Convert the edge cell ID back to lat/lng
                                    let edge_s2_cell = CellID(edge_cell_id);
                                    let edge_center = Cell::from(edge_s2_cell).center();
                                    let edge_latlng = LatLng::from(edge_center);
                                    
                                    let reply = SnapResponse {
                                        edge_index: edge_index.into(),
                                        lat: edge_latlng.lat.deg(),
                                        lng: edge_latlng.lng.deg(),
                                        debug_info: None,
                                    };
                                    
                                    return Ok(Response::new(reply));
                                }
                                
                                break;
                            }
                        }
                    }
                },
                Err(e) => {
                    eprintln!("Failed to parse SnapBuckets flatbuffer: {}", e);
                }
            }
        }
        
        // If we couldn't find a match, return the original coordinates
        let reply = SnapResponse {
            edge_index: 0,
            lat: req.lat,
            lng: req.lng,
            debug_info: None,
        };
        
        Ok(Response::new(reply))
    }
}