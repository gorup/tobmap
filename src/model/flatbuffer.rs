use anyhow::Result;
use flatbuffers::FlatBufferBuilder;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::collections::HashMap;

use crate::model::{Cell as ModelCell, Edge as ModelEdge, MapData as ModelMapData, Node as ModelNode};

// Import the generated FlatBuffer code
pub use crate::generated::tobmap;

/// Convert the map data to a FlatBuffer and write it to a file
pub fn write_to_file<P: AsRef<Path>>(map_data: &ModelMapData, path: P) -> Result<()> {
    let buf = convert_to_flatbuffer(map_data)?;
    let mut file = File::create(path)?;
    file.write_all(&buf)?;
    Ok(())
}

/// Convert the map data to a FlatBuffer
pub fn convert_to_flatbuffer(map_data: &ModelMapData) -> Result<Vec<u8>> {
    // Create a new FlatBufferBuilder with a reasonable initial size
    let mut builder = FlatBufferBuilder::with_capacity(1024 * 1024); // 1MB initial capacity
    
    // Convert the map data to FlatBuffer format
    let fb_map_data = convert_map_data_to_flatbuffer(&mut builder, map_data);
    
    // Finish the buffer
    builder.finish(fb_map_data, None);
    
    // Return the finished buffer
    Ok(builder.finished_data().to_vec())
}

/// Convert a MapData object to a FlatBuffer
fn convert_map_data_to_flatbuffer<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    map_data: &ModelMapData,
) -> flatbuffers::WIPOffset<tobmap::MapData<'a>> {
    // Create string offsets for metadata
    let version = builder.create_string(&map_data.version);
    let osm_data_date = builder.create_string(&map_data.osm_data_date);
    let generation_date = builder.create_string(&map_data.generation_date);
    
    // Convert all cells to FlatBuffer format
    let mut fb_cells = Vec::new();
    for (_, cell) in &map_data.cells {
        let fb_cell = convert_cell_to_flatbuffer(builder, cell);
        fb_cells.push(fb_cell);
    }
    
    // Create a vector of cell offsets
    let cells_vec = builder.create_vector(&fb_cells);
    
    // Create the MapData object
    let args = tobmap::MapDataArgs {
        version: Some(version),
        osm_data_date: Some(osm_data_date),
        generation_date: Some(generation_date),
        cells: Some(cells_vec),
    };
    
    tobmap::MapData::create(builder, &args)
}

/// Convert a Cell to a FlatBuffer
fn convert_cell_to_flatbuffer<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    cell: &ModelCell,
) -> flatbuffers::WIPOffset<tobmap::Cell<'a>> {
    // Convert all nodes to FlatBuffer format
    let mut fb_nodes = Vec::new();
    for node in &cell.nodes {
        let fb_node = convert_node_to_flatbuffer(builder, node);
        fb_nodes.push(fb_node);
    }
    
    // Create a vector of node offsets
    let nodes_vec = builder.create_vector(&fb_nodes);
    
    // Convert all edges to FlatBuffer format
    let mut fb_edges = Vec::new();
    for edge in &cell.edges {
        let fb_edge = convert_edge_to_flatbuffer(builder, edge);
        fb_edges.push(fb_edge);
    }
    
    // Create a vector of edge offsets
    let edges_vec = builder.create_vector(&fb_edges);
    
    // Create the Cell object
    let args = tobmap::CellArgs {
        s2_cell_id: cell.s2_cell_id,
        nodes: Some(nodes_vec),
        edges: Some(edges_vec),
    };
    
    tobmap::Cell::create(builder, &args)
}

/// Convert a Node to a FlatBuffer
fn convert_node_to_flatbuffer<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    node: &ModelNode,
) -> flatbuffers::WIPOffset<tobmap::Node<'a>> {
    // Create a string offset for the node ID
    let id = builder.create_string(&node.id);
    
    // Create the Node object
    let args = tobmap::NodeArgs {
        id: Some(id),
        s2_cell_id: node.s2_cell_id,
        lat: node.lat,
        lng: node.lng,
    };
    
    tobmap::Node::create(builder, &args)
}

/// Convert an Edge to a FlatBuffer
fn convert_edge_to_flatbuffer<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    edge: &ModelEdge,
) -> flatbuffers::WIPOffset<tobmap::Edge<'a>> {
    // Create string offsets
    let id = builder.create_string(&edge.id);
    let source_node_id = builder.create_string(&edge.source_node_id);
    let destination_node_id = builder.create_string(&edge.destination_node_id);
    let name = builder.create_string(&edge.name);
    
    // Create vectors for travel costs and geometry
    let travel_costs_vec = builder.create_vector(&edge.travel_costs);
    let geometry_lats_vec = builder.create_vector(&edge.geometry_lats);
    let geometry_lngs_vec = builder.create_vector(&edge.geometry_lngs);
    
    // Convert tags to FlatBuffer KeyValue objects
    let mut fb_tags = Vec::new();
    for (key, value) in &edge.tags {
        let key_str = builder.create_string(key);
        let value_str = builder.create_string(value);
        
        let args = tobmap::KeyValueArgs {
            key: Some(key_str),
            value: Some(value_str),
        };
        
        let kv = tobmap::KeyValue::create(builder, &args);
        fb_tags.push(kv);
    }
    
    // Create a vector of KeyValue offsets
    let tags_vec = builder.create_vector(&fb_tags);
    
    // Create the Edge object
    let args = tobmap::EdgeArgs {
        id: Some(id),
        source_node_id: Some(source_node_id),
        destination_node_id: Some(destination_node_id),
        name: Some(name),
        osm_way_id: edge.osm_way_id,
        travel_costs: Some(travel_costs_vec),
        geometry_lats: Some(geometry_lats_vec),
        geometry_lngs: Some(geometry_lngs_vec),
        tags: Some(tags_vec),
    };
    
    tobmap::Edge::create(builder, &args)
}

/// Read map data from a file
pub fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
    let file_path = path.as_ref();
    let mut file = File::open(file_path)?;
    
    // Read the file contents into a buffer
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer)?;
    
    Ok(buffer)
}

/// Parse a flatbuffer back into a MapData struct
pub fn parse_flatbuffer(buffer: &[u8]) -> Result<ModelMapData> {
    // Get the root MapData object from the buffer
    let fb_map_data = tobmap::root_as_map_data(buffer)?;
    
    // Create a new ModelMapData
    let mut map_data = ModelMapData::new();
    
    // Copy metadata
    if let Some(version) = fb_map_data.version() {
        map_data.version = version.to_string();
    }
    
    if let Some(osm_data_date) = fb_map_data.osm_data_date() {
        map_data.osm_data_date = osm_data_date.to_string();
    }
    
    if let Some(generation_date) = fb_map_data.generation_date() {
        map_data.generation_date = generation_date.to_string();
    }
    
    // Process each cell
    if let Some(cells) = fb_map_data.cells() {
        for i in 0..cells.len() {
            let fb_cell = cells.get(i);
            
            // Get the cell's S2 cell ID
            let s2_cell_id = fb_cell.s2_cell_id();
            let cell = map_data.get_or_create_cell(s2_cell_id);
            
            // Process nodes
            if let Some(nodes) = fb_cell.nodes() {
                for j in 0..nodes.len() {
                    let fb_node = nodes.get(j);
                    let node = ModelNode {
                        id: fb_node.id().unwrap_or("").to_string(),
                        s2_cell_id: fb_node.s2_cell_id(),
                        lat: fb_node.lat(),
                        lng: fb_node.lng(),
                    };
                    cell.add_node(node);
                }
            }
            
            // Process edges
            if let Some(edges) = fb_cell.edges() {
                for j in 0..edges.len() {
                    let fb_edge = edges.get(j);
                    
                    let mut edge = ModelEdge {
                        id: fb_edge.id().unwrap_or("").to_string(),
                        source_node_id: fb_edge.source_node_id().unwrap_or("").to_string(),
                        destination_node_id: fb_edge.destination_node_id().unwrap_or("").to_string(),
                        name: fb_edge.name().unwrap_or("").to_string(),
                        osm_way_id: fb_edge.osm_way_id(),
                        travel_costs: Vec::new(),
                        geometry_lats: Vec::new(),
                        geometry_lngs: Vec::new(),
                        tags: HashMap::new(),
                    };
                    
                    // Process travel costs
                    if let Some(travel_costs) = fb_edge.travel_costs() {
                        for k in 0..travel_costs.len() {
                            edge.travel_costs.push(travel_costs.get(k));
                        }
                    }
                    
                    // Process geometry
                    if let Some(geometry_lats) = fb_edge.geometry_lats() {
                        for k in 0..geometry_lats.len() {
                            edge.geometry_lats.push(geometry_lats.get(k));
                        }
                    }
                    
                    if let Some(geometry_lngs) = fb_edge.geometry_lngs() {
                        for k in 0..geometry_lngs.len() {
                            edge.geometry_lngs.push(geometry_lngs.get(k));
                        }
                    }
                    
                    // Process tags
                    if let Some(tags) = fb_edge.tags() {
                        for k in 0..tags.len() {
                            let fb_tag = tags.get(k);
                            if let (Some(key), Some(value)) = (fb_tag.key(), fb_tag.value()) {
                                edge.tags.insert(key.to_string(), value.to_string());
                            }
                        }
                    }
                    
                    cell.add_edge(edge);
                }
            }
        }
    }
    
    Ok(map_data)
} 