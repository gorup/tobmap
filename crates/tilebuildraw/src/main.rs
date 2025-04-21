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
use tilebuildraw::proto::tobmapdata::{S2CellData, Vertex, Edge};
use schema::graph_generated::tobmapgraph;

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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Define our three tile levels
    let levels = vec![
        TileLevel {
            name: "high".to_string(),
            s2_cell_level: 1,
            min_priority: 8,
            max_priority: 10,
        },
        TileLevel {
            name: "medium".to_string(),
            s2_cell_level: 4,
            min_priority: 4,
            max_priority: 7,
        },
        TileLevel {
            name: "low".to_string(),
            s2_cell_level: 7,
            min_priority: 0,
            max_priority: 3,
        },
    ];

    // Read blob files
    info!("Reading blob files...");
    let graph_data = fs::read(&args.graph_blob)?;
    let location_data = fs::read(&args.location_blob)?;
    let description_data = fs::read(&args.description_blob)?;

    // Parse flatbuffers data
    info!("Parsing flatbuffers data...");
    let graph_blob = root::<tobmapgraph::GraphBlob>(&graph_data)?;
    let location_blob = root::<tobmapgraph::LocationBlob>(&location_data)?;
    let description_blob = root::<tobmapgraph::DescriptionBlob>(&description_data)?;

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
    
    // Create output directory for this level
    let level_dir = output_dir.join(&level.name);
    fs::create_dir_all(&level_dir)?;

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

        // Write tile to file
        let tile_path = level_dir.join(format!("tile_{}.pb", cell_id));
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