use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flatbuffers::FlatBufferBuilder;
use schema::graph_generated::tobmapgraph::{GraphBlob, LocationBlob};
use schema::snap_generated::tobmapsnap::{SnapBucket, SnapBucketArgs, SnapBuckets, SnapBucketsArgs};

/// Configuration for SnapBucket generation
pub struct Config {
    pub cell_level: u8,
    pub graph_path: PathBuf,
    pub location_path: PathBuf,
    pub output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cell_level: 4,
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
        .expect("Failed to parse graph data from buffer");
        
    let location_blob = flatbuffers::root_with_opts::<LocationBlob>(&verifier_opts, &location_data)
        .expect("Failed to parse location data from buffer");
    
    // Create output directory if it doesn't exist
    fs::create_dir_all(&config.output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;
    
    // Group nodes and edges by cell ids at the specified level
    let buckets = build_buckets(&graph_blob, &location_blob, config.cell_level)?;
    
    // Generate and write SnapBuckets files
    write_snap_buckets(&buckets, &config.output_dir)?;
    
    Ok(())
}

// Read binary data from a file
fn read_binary_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

// Data structure to hold bucket data
struct BucketData {
    cell_id: u64,
    edge_cell_ids: Vec<u64>,
    edge_indexes: Vec<u32>,
}

// Truncate a cell_id to a specific level
fn truncate_cell_id(cell_id: u64, level: u8) -> u64 {
    // Truncate the cell_id to the specific level
    // This is a simplified implementation and may need to be adjusted
    // based on your specific cell_id encoding scheme
    let bits_to_keep = level * 3; // Assuming each level adds 3 bits of precision
    cell_id >> (64 - bits_to_keep)
}

// Build buckets of data grouped by cell id at the specified level
fn build_buckets(graph_blob: &GraphBlob, location_blob: &LocationBlob, cell_level: u8) -> Result<HashMap<u64, BucketData>, String> {
    let mut buckets: HashMap<u64, BucketData> = HashMap::new();
    
    // Process node locations
    if let Some(node_locations) = location_blob.node_location_items() {
        for i in 0..node_locations.len() {
            // Direct access instead of using match for non-optional value
            let node_loc = node_locations.get(i);
            
            let cell_id = node_loc.cell_id();
            let truncated_cell_id = truncate_cell_id(cell_id, cell_level);
            
            let bucket = buckets.entry(truncated_cell_id).or_insert_with(|| BucketData {
                cell_id: truncated_cell_id,
                edge_cell_ids: Vec::new(),
                edge_indexes: Vec::new(),
            });
            
            // Process node edges
            if let Some(graph_nodes) = graph_blob.nodes() {
                if i < graph_nodes.len() {
                    // Direct access instead of using match for non-optional value
                    let node = graph_nodes.get(i);
                    
                    if let Some(edges) = node.edges() {
                        for j in 0..edges.len() {
                            let edge_index = edges.get(j) as u32;
                            bucket.edge_indexes.push(edge_index);
                            
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
                                        // Direct access instead of using match for non-optional value
                                        let target_loc = node_locations.get(target_node_idx as usize);
                                        let target_truncated_cell_id = truncate_cell_id(target_loc.cell_id(), cell_level);
                                        bucket.edge_cell_ids.push(target_truncated_cell_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(buckets)
}

// Write SnapBuckets to files
fn write_snap_buckets(buckets: &HashMap<u64, BucketData>, output_dir: &Path) -> Result<(), String> {
    for (_, bucket_data) in buckets {
        let mut fbb = FlatBufferBuilder::new();
        
        // Create vectors for edge cell ids and edge indexes
        let edge_cell_ids = fbb.create_vector(&bucket_data.edge_cell_ids);
        let edge_indexes = fbb.create_vector(&bucket_data.edge_indexes);
        
        // Create SnapBucket
        let snap_bucket = SnapBucket::create(
            &mut fbb,
            &SnapBucketArgs {
                cell_id: bucket_data.cell_id,
                edge_cell_ids: Some(edge_cell_ids),
                edge_indexes: Some(edge_indexes),
            },
        );
        
        // Create a vector of SnapBuckets (with only one element for this cell)
        let snap_buckets_vector = fbb.create_vector(&[snap_bucket]);
        
        // Create the SnapBuckets root object
        let snap_buckets = SnapBuckets::create(
            &mut fbb,
            &SnapBucketsArgs {
                snap_buckets: Some(snap_buckets_vector),
            },
        );
        
        fbb.finish(snap_buckets, None);
        
        // Write to file
        let file_path = output_dir.join(format!("snap_bucket_{}.bin", bucket_data.cell_id));
        let mut file = File::create(&file_path)
            .map_err(|e| format!("Failed to create file {}: {}", file_path.display(), e))?;
        
        file.write_all(fbb.finished_data())
            .map_err(|e| format!("Failed to write to file {}: {}", file_path.display(), e))?;
    }
    
    Ok(())
}