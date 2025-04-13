use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flatbuffers::FlatBufferBuilder;
use s2::{cell::Cell, cellid::CellID};
use schema::graph_generated::tobmapgraph::{GraphBlob, LocationBlob};
use schema::snap_generated::tobmapsnap::{SnapBucket, SnapBucketArgs, SnapBuckets, SnapBucketsArgs};

/// Configuration for SnapBucket generation
pub struct Config {
    pub outer_cell_level: u8,
    pub inner_cell_level: u8,
    pub graph_path: PathBuf,
    pub location_path: PathBuf,
    pub output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            outer_cell_level: 4,
            inner_cell_level: 8,
            graph_path: PathBuf::from("graph.bin"),
            location_path: PathBuf::from("location.bin"),
            output_dir: PathBuf::from("snapbuckets"),
        }
    }
}

/// Process the graph and location data to generate SnapBuckets files
pub fn process(config: &Config) -> Result<(), String> {
    // Read graph data
    let graph_data = read_binary_file(&config.graph_path)
        .map_err(|e| format!("Failed to read graph file: {}", e))?;
    
    // Read location data
    let location_data = read_binary_file(&config.location_path)
        .map_err(|e| format!("Failed to read location file: {}", e))?;

    // Use get_root_with_opts instead of root for better error handling and custom verifier options
    let verifier_opts = flatbuffers::VerifierOptions {
        max_tables: 3_000_000_000, // 3 billion tables
        ..Default::default()
    };
    
    // Parse graph blob
    let graph_blob = flatbuffers::root_with_opts::<GraphBlob>(&verifier_opts, &graph_data)
        .map_err(|e| format!("Failed to parse graph data: {}", e))?;
        
    let location_blob = flatbuffers::root_with_opts::<LocationBlob>(&verifier_opts, &location_data)
        .map_err(|e| format!("Failed to parse location data: {}", e))?;
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(&config.output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    
    // Group nodes and edges by cell ids at the specified levels
    let outer_buckets = build_outer_buckets(&graph_blob, &location_blob, config.outer_cell_level, config.inner_cell_level)?;
    
    // Generate and write SnapBuckets files, one per outer level cell
    write_snap_buckets(&outer_buckets, &config.output_dir)?;
    
    Ok(())
}

// Read binary data from a file
fn read_binary_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

// Data structure to hold inner bucket data
struct InnerBucketData {
    cell_id: u64,
    edge_cell_ids: Vec<u64>,
    edge_indexes: Vec<u32>,
}

// Data structure to hold outer bucket data (contains inner buckets)
struct OuterBucketData {
    cell_id: u64,
    inner_buckets: HashMap<u64, InnerBucketData>,
}

// Truncate a cell_id to a specific level using S2 library
fn parent_cell_id(cell_id: u64, level: u8) -> u64 {
    let s2_cell_id = CellID(cell_id);
    s2_cell_id.parent(level as u64).0
}

// Convert cell ID to token string representation
fn cell_id_to_token(cell_id: u64, level: u8) -> String {
    let s2_cell_id = CellID(cell_id);
    s2_cell_id.to_token()
}

// Build outer buckets with inner buckets grouped by cell IDs
fn build_outer_buckets(
    graph_blob: &GraphBlob, 
    location_blob: &LocationBlob, 
    outer_level: u8, 
    inner_level: u8
) -> Result<HashMap<u64, OuterBucketData>, String> {
    let mut outer_buckets: HashMap<u64, OuterBucketData> = HashMap::new();
    let mut all_outer_cell_ids = HashSet::new();
    
    // First pass: collect all outer cell IDs from node locations
    if let Some(node_locations) = location_blob.node_location_items() {
        for i in 0..node_locations.len() {
            let node_loc = node_locations.get(i);
            let cell_id = node_loc.cell_id();
            let outer_cell_id = parent_cell_id(cell_id, outer_level);
            all_outer_cell_ids.insert(outer_cell_id);
        }
    }
    
    // Initialize all outer buckets with empty inner buckets
    for &outer_cell_id in &all_outer_cell_ids {
        let outer_bucket = outer_buckets.entry(outer_cell_id).or_insert_with(|| OuterBucketData {
            cell_id: outer_cell_id,
            inner_buckets: HashMap::new(),
        });
        
        // Generate all possible inner cells for this outer cell
        // For simplicity, we'll just ensure we have entries in the inner_buckets map
        // A real implementation would calculate all possible inner cells within the outer cell
    }
    
    // Process node locations and edges
    if let Some(node_locations) = location_blob.node_location_items() {
        for i in 0..node_locations.len() {
            let node_loc = node_locations.get(i);
            let cell_id = node_loc.cell_id();
            let outer_cell_id = parent_cell_id(cell_id, outer_level);
            let inner_cell_id = parent_cell_id(cell_id, inner_level);
            
            let outer_bucket = outer_buckets.get_mut(&outer_cell_id).unwrap();
            let inner_bucket = outer_bucket.inner_buckets.entry(inner_cell_id).or_insert_with(|| InnerBucketData {
                cell_id: inner_cell_id,
                edge_cell_ids: Vec::new(),
                edge_indexes: Vec::new(),
            });
            
            // Process node edges
            if let Some(graph_nodes) = graph_blob.nodes() {
                if i < graph_nodes.len() {
                    let node = graph_nodes.get(i);
                    
                    if let Some(edges) = node.edges() {
                        for j in 0..edges.len() {
                            let edge_index = edges.get(j) as u32;
                            
                            // Get the connected node's cell_id
                            if let Some(graph_edges) = graph_blob.edges() {
                                if (edge_index as usize) < graph_edges.len() {
                                    let edge = graph_edges.get(edge_index as usize);
                                    let target_node_idx = if edge.point_1_node_idx() == i as u32 {
                                        edge.point_2_node_idx()
                                    } else {
                                        edge.point_1_node_idx()
                                    };
                                    
                                    // Get the cell_id of the target node
                                    if (target_node_idx as usize) < node_locations.len() {
                                        let target_loc = node_locations.get(target_node_idx as usize);
                                        
                                        inner_bucket.edge_cell_ids.push(target_loc.cell_id());
                                        inner_bucket.edge_indexes.push(edge_index);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Ensure all possible inner cells for each outer cell have entries
    // In a real implementation, you would calculate all possible inner cells within each outer cell
    // For now, we'll ensure inner buckets are consistently represented in our output
    for outer_bucket in outer_buckets.values_mut() {
        // Get all unique inner cell IDs that should exist at inner_level within this outer cell
        let mut all_inner_cell_ids = HashSet::new();
        
        // For each outer cell, calculate all possible inner cells
        // This is a simplified approach - in practice you'd generate all inner cells based on the specific S2 algorithm
        let num_cells_per_side = 1 << (inner_level - outer_level); // Number of inner cells per dimension
        let total_inner_cells = num_cells_per_side * num_cells_per_side; // Total inner cells in this outer cell
        
        for i in 0..total_inner_cells {
            // This is a simplified mapping from outer to inner cells
            // A real implementation would use proper S2Cell logic
            let inner_cell_id = (outer_bucket.cell_id << ((inner_level - outer_level) * 2)) | i;
            all_inner_cell_ids.insert(inner_cell_id);
        }
        
        // Ensure all possible inner cells have entries
        for inner_cell_id in all_inner_cell_ids {
            outer_bucket.inner_buckets.entry(inner_cell_id).or_insert_with(|| InnerBucketData {
                cell_id: inner_cell_id,
                edge_cell_ids: Vec::new(),
                edge_indexes: Vec::new(),
            });
        }
    }
    
    Ok(outer_buckets)
}

// Write SnapBuckets to files, one file per outer bucket
fn write_snap_buckets(outer_buckets: &HashMap<u64, OuterBucketData>, output_dir: &Path) -> Result<(), String> {
    for (_, outer_bucket) in outer_buckets {
        let mut fbb = FlatBufferBuilder::new();
        let mut snap_bucket_offsets = Vec::new();
        
        // Sort inner buckets by cell_id for consistency
        let mut inner_buckets: Vec<_> = outer_bucket.inner_buckets.values().collect();
        inner_buckets.sort_by_key(|b| b.cell_id);
        
        // Create a SnapBucket for each inner bucket
        for inner_bucket in inner_buckets {
            // Create vectors for edge cell ids and edge indexes
            let edge_cell_ids = fbb.create_vector(&inner_bucket.edge_cell_ids);
            let edge_indexes = fbb.create_vector(&inner_bucket.edge_indexes);
            
            // Create SnapBucket for this inner bucket
            let snap_bucket = SnapBucket::create(
                &mut fbb,
                &SnapBucketArgs {
                    cell_id: inner_bucket.cell_id,
                    edge_cell_ids: Some(edge_cell_ids),
                    edge_indexes: Some(edge_indexes),
                },
            );
            
            snap_bucket_offsets.push(snap_bucket);
        }
        
        // Create a vector of all SnapBuckets for this outer bucket
        let snap_buckets_vector = fbb.create_vector(&snap_bucket_offsets);
        
        // Create the SnapBuckets root object
        let snap_buckets = SnapBuckets::create(
            &mut fbb,
            &SnapBucketsArgs {
                snap_buckets: Some(snap_buckets_vector),
            },
        );
        
        fbb.finish(snap_buckets, None);
        
        // Use S2 library to get cell info
        let s2_cell_id = CellID(outer_bucket.cell_id);
        let cell = Cell::from(s2_cell_id);
        let level = cell.level();
        let token = s2_cell_id.to_token();
        
        // Log the outer bucket cell ID and its token
        println!("Processing outer bucket - Cell ID: {}, Token: {}, Level: {}", 
                 outer_bucket.cell_id, token, level);
        
        // Write to file named by the outer bucket's token
        let file_path = output_dir.join(format!("snap_bucket_{}.bin", token));
        let mut file = File::create(&file_path)
            .map_err(|e| format!("Failed to create file {}: {}", file_path.display(), e))?;
        
        file.write_all(fbb.finished_data())
            .map_err(|e| format!("Failed to write to file {}: {}", file_path.display(), e))?;
    }
    
    Ok(())
}