use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use prost::Message;
use clap::Parser;
use flatbuffers::root;
use s2::{cell::Cell, cellid::CellID};
use rayon::prelude::*;
use log::{info, warn, debug};
use tilebuildvector::proto::tobmapdata::{S2CellData, Vertex, Edge};
use schema::graph_generated::tobmapgraph;
use anyhow::Context;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to the GraphBlob file
    #[clap(long)]
    graph_blob: PathBuf,

    /// Path to the LocationBlob file
    #[clap(long)]
    location_blob: PathBuf,

    /// Path to the DescriptionBlob file
    #[clap(long)]
    description_blob: PathBuf,

    /// Output directory for the tiles
    #[clap(long)]
    output_dir: PathBuf,
}

// Define the tile levels
struct TileLevel {
    name: String,
    s2_cell_level: u8,
    min_priority: u8,
    max_priority: u8,
}

// Convert priority to zoom level (0-10)
fn priority_to_zoom(priority: u8) -> u8 {
    // Inverting priority (10 is highest priority, 0 is lowest)
    // So zoom 0 is highest priority, zoom 10 is lowest
    10 - priority.min(10)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Define our ten tile levels (one for each priority)
    let levels = vec![
        TileLevel {
            name: "level0".to_string(),
            s2_cell_level: 1,
            min_priority: 10,
            max_priority: 10,
        },
        TileLevel {
            name: "level1".to_string(),
            s2_cell_level: 2,
            min_priority: 9,
            max_priority: 9,
        },
        TileLevel {
            name: "level2".to_string(),
            s2_cell_level: 3,
            min_priority: 8,
            max_priority: 8,
        },
        TileLevel {
            name: "level3".to_string(),
            s2_cell_level: 4,
            min_priority: 7,
            max_priority: 7,
        },
        TileLevel {
            name: "level4".to_string(),
            s2_cell_level: 5,
            min_priority: 6,
            max_priority: 6,
        },
        TileLevel {
            name: "level5".to_string(),
            s2_cell_level: 6,
            min_priority: 5,
            max_priority: 5,
        },
        TileLevel {
            name: "level6".to_string(),
            s2_cell_level: 7,
            min_priority: 4,
            max_priority: 4,
        },
        TileLevel {
            name: "level7".to_string(),
            s2_cell_level: 8,
            min_priority: 3,
            max_priority: 3,
        },
        TileLevel {
            name: "level8".to_string(),
            s2_cell_level: 9,
            min_priority: 2,
            max_priority: 2,
        },
        TileLevel {
            name: "level9".to_string(),
            s2_cell_level: 10,
            min_priority: 1,
            max_priority: 1,
        },
        TileLevel {
            name: "level10".to_string(),
            s2_cell_level: 11,
            min_priority: 0,
            max_priority: 0,
        },
    ];

    // Read blob files
    info!("Reading blob files...");
    let graph_data = fs::read(&args.graph_blob)?;
    let location_data = fs::read(&args.location_blob)?;
    let description_data = fs::read(&args.description_blob)?;

    // Parse flatbuffers data
    info!("Parsing flatbuffers data...");
    // Use get_root_with_opts instead of root for better error handling and custom verifier options
    let verifier_opts = flatbuffers::VerifierOptions {
        max_tables: 3_000_000_000, // 3 billion tables
        ..Default::default()
    };

    let graph_blob = flatbuffers::root_with_opts::<tobmapgraph::GraphBlob>(&verifier_opts, &graph_data)
        .with_context(|| "Failed to parse graph data from buffer")?;

    let location_blob = flatbuffers::root_with_opts::<tobmapgraph::LocationBlob>(&verifier_opts, &location_data)
        .with_context(|| "Failed to parse location data from buffer")?;

    let description_blob = flatbuffers::root_with_opts::<tobmapgraph::DescriptionBlob>(&verifier_opts, &description_data)
        .with_context(|| "Failed to parse description data from buffer")?;


    // Process data and generate tiles for each level
    for level in &levels {
        generate_tiles_for_level(
            level,
            &graph_blob,
            &location_blob,
            &description_blob,
            &args.output_dir,
        )?;
    }

    info!("Tile generation completed successfully!");
    Ok(())
}

fn generate_tiles_for_level(
    level: &TileLevel,
    graph_blob: &tobmapgraph::GraphBlob,
    location_blob: &tobmapgraph::LocationBlob,
    description_blob: &tobmapgraph::DescriptionBlob,
    output_dir: &Path,
) -> anyhow::Result<()> {
    info!("Generating tiles for level: {}", level.name);
    
    // Build a map of edge index to edge description
    let mut edge_descriptions = HashMap::new();
    if let Some(desc_vec) = description_blob.edge_descriptions() {
        for (i, desc) in desc_vec.iter().enumerate() {
            let priority = desc.priority();
            if priority >= level.min_priority && priority <= level.max_priority {
                let mut street_names = Vec::new();
                if let Some(names) = desc.street_names() {
                    for name in names {
                        street_names.push(name.to_string());
                    }
                }
                
                // Get whether this edge is one-way from the graph blob if available
                let is_oneway = if let Some(graph_edges) = graph_blob.edges() {
                    if i < graph_edges.len() {
                        // In a real implementation, you would extract this from the costs_and_flags
                        // This is a placeholder - replace with actual logic
                        let flags = graph_edges.get(i).costs_and_flags();
                        (flags & 0x1) != 0 // Example: first bit indicates one-way
                    } else {
                        false
                    }
                } else {
                    false
                };
                
                edge_descriptions.insert(i as u32, (priority, street_names, is_oneway));
            }
        }
    }

    // Group edges by S2 cell
    let mut cell_to_edges: HashMap<u64, Vec<(usize, Vec<u64>)>> = HashMap::new();
    
    if let Some(edges_loc) = location_blob.edge_location_items() {
        for (edge_idx, edge_loc) in edges_loc.iter().enumerate() {
            if let Some(points) = edge_loc.points() {
                // Skip edges that don't match our priority level
                if !edge_descriptions.contains_key(&(edge_idx as u32)) {
                    continue;
                }
                
                // Get all relevant S2 cells for this edge at our level
                let mut cells = HashSet::new();
                for point in points {
                    // Convert to the appropriate S2 cell level using the S2 library
                    let cell_id = CellID(point);
                    let cell_at_level = cell_id.parent(level.s2_cell_level as u64);
                    cells.insert(cell_at_level.0);
                }
                
                // Add edge to all relevant cells
                let point_vec: Vec<u64> = points.iter().collect();
                for cell in cells {
                    cell_to_edges.entry(cell).or_default().push((edge_idx, point_vec.clone()));
                }
            }
        }
    }

    // Generate tiles in parallel
    let results: Vec<anyhow::Result<()>> = cell_to_edges.par_iter().map(|(cell_id, edges)| {
        let mut tile = S2CellData {
            cell_id: *cell_id,
            vertices: Vec::new(),
            edges: Vec::new(),
        };

        // Add vertices (unique cells)
        let mut vertex_cells = HashSet::new();
        for (_, points) in edges {
            for point in points {
                vertex_cells.insert(*point);
            }
        }

        for cell in vertex_cells {
            tile.vertices.push(Vertex {
                cell_id: cell,
            });
        }

        // Add edges
        for (edge_idx, points) in edges {
            if let Some((priority, street_names, is_oneway)) = edge_descriptions.get(&(*edge_idx as u32)) {
                let proto_edge = Edge {
                    points: points.clone(),
                    priority: *priority as u32,
                    street_names: street_names.clone(),
                    is_oneway: *is_oneway,
                };
                tile.edges.push(proto_edge);
            }
        }

        // Convert priority to zoom level
        let zoom = priority_to_zoom(level.min_priority);

        // Convert cell ID to token for filename
        let cell = Cell::from(CellID(*cell_id));
        let token = cell.id.to_token();

        // Write tile to file using token instead of raw cell ID
        let tile_path = output_dir.join(format!("level_{}/tile_{}.pb", zoom, token));
        fs::create_dir_all(tile_path.parent().unwrap())?;
        let mut file = File::create(tile_path)?;
        let encoded = tile.encode_to_vec();
        file.write_all(&encoded)?;

        Ok(())
    }).collect();

    // Check for errors
    for result in results {
        result?;
    }

    info!("Generated {} tiles for level {}", cell_to_edges.len(), level.name);
    Ok(())
}