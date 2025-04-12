use tonic::{transport::Server, Request, Response, Status};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use s2::{cell::Cell, cellid::CellID, latlng::LatLng};

use tobmapapi::snap_service_server::{SnapService, SnapServiceServer};
use tobmapapi::{SnapRequest, SnapResponse, SnapResponseDebugInfo};
use schema::snap_generated::tobmapsnap::{SnapBuckets, SnapBucket};

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
            
            // Binary search to find the closest edge
            let mut left = 0;
            let mut right = edge_cell_ids.len() - 1;
            
            // Handle edge cases
            if edge_cell_ids.len() == 1 {
                return Some((edge_indexes.get(0), edge_cell_ids.get(0)));
            }
            
            // Check if target is outside the range
            if target_cell_id <= edge_cell_ids.get(0) {
                return Some((edge_indexes.get(0), edge_cell_ids.get(0)));
            }
            
            if target_cell_id >= edge_cell_ids.get(right) {
                return Some((edge_indexes.get(right), edge_cell_ids.get(right)));
            }
            
            // Binary search
            while left <= right {
                let mid = left + (right - left) / 2;
                let mid_cell_id = edge_cell_ids.get(mid);
                
                if mid_cell_id == target_cell_id {
                    return Some((edge_indexes.get(mid), mid_cell_id));
                } else if mid_cell_id < target_cell_id {
                    left = mid + 1;
                } else {
                    right = mid - 1;
                }
            }
            
            // After binary search, left is the insertion point
            // Compare adjacent elements to find the closer one
            if left >= edge_cell_ids.len() {
                left = edge_cell_ids.len() - 1;
            }
            
            let cell_id_left = edge_cell_ids.get(if left > 0 { left - 1 } else { 0 });
            let cell_id_right = edge_cell_ids.get(left);
            
            if (target_cell_id - cell_id_left).abs_diff(0) < (target_cell_id - cell_id_right).abs_diff(0) {
                return Some((edge_indexes.get(if left > 0 { left - 1 } else { 0 }), cell_id_left));
            } else {
                return Some((edge_indexes.get(left), cell_id_right));
            }
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
        
        // Get the outer cell ID for the requested location
        let outer_cell_id = cell_id.parent(self.outer_cell_level as u64).0;
        
        // Get the inner cell ID for the requested location
        let inner_cell_id = cell_id.parent(self.inner_cell_level as u64).0;
        
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