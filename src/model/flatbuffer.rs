use anyhow::Result;
use flatbuffers::FlatBufferBuilder;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::collections::HashMap;
use uuid::Uuid;

use crate::model::{Cell as ModelCell, MapData as ModelMapData};
use crate::generated::tobmap::{self, Node, Edge, Cell, MapData, NodeArgs, EdgeArgs, KeyValue, KeyValueArgs, CellArgs, MapDataArgs};

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
) -> flatbuffers::WIPOffset<MapData<'a>> {
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
    let args = MapDataArgs {
        version: Some(version),
        osm_data_date: Some(osm_data_date),
        generation_date: Some(generation_date),
        cells: Some(cells_vec),
    };
    
    MapData::create(builder, &args)
}

/// Convert a Cell to a FlatBuffer
fn convert_cell_to_flatbuffer<'a>(
    builder: &mut FlatBufferBuilder<'a>,
    cell: &ModelCell,
) -> flatbuffers::WIPOffset<Cell<'a>> {
    // Get the data from the cell's buffer storage
    let s2_cell_id = cell.s2_cell_id;
    
    // For nodes
    let mut node_offsets = Vec::new();
    let nodes = cell.nodes();
    for node in nodes {
        // Extract node data
        let id = node.id().unwrap_or("");
        let id_offset = builder.create_string(id);
        
        // Create node
        let node_args = NodeArgs {
            id: Some(id_offset),
            s2_cell_id: node.s2_cell_id(),
            lat: node.lat(),
            lng: node.lng(),
        };
        
        let node_offset = Node::create(builder, &node_args);
        node_offsets.push(node_offset);
    }
    
    let nodes_vec = builder.create_vector(&node_offsets);
    
    // For edges
    let mut edge_offsets = Vec::new();
    let edges = cell.edges();
    for edge in edges {
        // Extract edge data
        let id = edge.id().unwrap_or("");
        let source_node_id = edge.source_node_id().unwrap_or("");
        let destination_node_id = edge.destination_node_id().unwrap_or("");
        let name = edge.name().unwrap_or("");
        
        let id_offset = builder.create_string(id);
        let source_id_offset = builder.create_string(source_node_id);
        let dest_id_offset = builder.create_string(destination_node_id);
        let name_offset = builder.create_string(name);
        
        // Extract travel costs
        let mut travel_costs = Vec::new();
        if let Some(costs) = edge.travel_costs() {
            for i in 0..costs.len() {
                travel_costs.push(costs.get(i));
            }
        }
        let costs_vec = builder.create_vector(&travel_costs);
        
        // Extract geometry
        let mut geometry_lats = Vec::new();
        let mut geometry_lngs = Vec::new();
        
        if let Some(lats) = edge.geometry_lats() {
            for i in 0..lats.len() {
                geometry_lats.push(lats.get(i));
            }
        }
        
        if let Some(lngs) = edge.geometry_lngs() {
            for i in 0..lngs.len() {
                geometry_lngs.push(lngs.get(i));
            }
        }
        
        let lats_vec = builder.create_vector(&geometry_lats);
        let lngs_vec = builder.create_vector(&geometry_lngs);
        
        // Extract tags
        let mut tag_offsets = Vec::new();
        if let Some(tags) = edge.tags() {
            for i in 0..tags.len() {
                let tag = tags.get(i);
                let key = tag.key().unwrap_or("");
                let value = tag.value().unwrap_or("");
                
                let key_offset = builder.create_string(key);
                let value_offset = builder.create_string(value);
                
                let tag_args = KeyValueArgs {
                    key: Some(key_offset),
                    value: Some(value_offset),
                };
                
                let tag_offset = KeyValue::create(builder, &tag_args);
                tag_offsets.push(tag_offset);
            }
        }
        
        let tags_vec = builder.create_vector(&tag_offsets);
        
        // Create edge
        let edge_args = EdgeArgs {
            id: Some(id_offset),
            source_node_id: Some(source_id_offset),
            destination_node_id: Some(dest_id_offset),
            name: Some(name_offset),
            osm_way_id: edge.osm_way_id(),
            travel_costs: Some(costs_vec),
            geometry_lats: Some(lats_vec),
            geometry_lngs: Some(lngs_vec),
            tags: Some(tags_vec),
        };
        
        let edge_offset = Edge::create(builder, &edge_args);
        edge_offsets.push(edge_offset);
    }
    
    let edges_vec = builder.create_vector(&edge_offsets);
    
    // Create the Cell object
    let args = CellArgs {
        s2_cell_id,
        nodes: Some(nodes_vec),
        edges: Some(edges_vec),
    };
    
    Cell::create(builder, &args)
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
                    
                    // Create a new FlatBufferBuilder for this node
                    let mut builder = FlatBufferBuilder::new();
                    
                    // Extract node data
                    let id = fb_node.id().unwrap_or("").to_string();
                    let id_offset = builder.create_string(&id);
                    
                    // Create a new node
                    let node_args = NodeArgs {
                        id: Some(id_offset),
                        s2_cell_id: fb_node.s2_cell_id(),
                        lat: fb_node.lat(),
                        lng: fb_node.lng(),
                    };
                    
                    let node_offset = Node::create(&mut builder, &node_args);
                    builder.finish(node_offset, None);
                    
                    // Get the buffer data and add it to the cell - make sure we clone the data
                    let buf = builder.finished_data().to_vec();
                    cell.add_node(buf);
                }
            }
            
            // Process edges
            if let Some(edges) = fb_cell.edges() {
                for j in 0..edges.len() {
                    let fb_edge = edges.get(j);
                    
                    // Create a new FlatBufferBuilder for this edge
                    let mut builder = FlatBufferBuilder::new();
                    
                    // Extract edge data
                    let id = fb_edge.id().unwrap_or("").to_string();
                    let source_node_id = fb_edge.source_node_id().unwrap_or("").to_string();
                    let destination_node_id = fb_edge.destination_node_id().unwrap_or("").to_string();
                    let name = fb_edge.name().unwrap_or("").to_string();
                    
                    let id_offset = builder.create_string(&id);
                    let source_id_offset = builder.create_string(&source_node_id);
                    let dest_id_offset = builder.create_string(&destination_node_id);
                    let name_offset = builder.create_string(&name);
                    
                    // Extract travel costs
                    let mut travel_costs = Vec::new();
                    if let Some(costs) = fb_edge.travel_costs() {
                        for k in 0..costs.len() {
                            travel_costs.push(costs.get(k));
                        }
                    }
                    let costs_vec = builder.create_vector(&travel_costs);
                    
                    // Extract geometry
                    let mut geometry_lats = Vec::new();
                    let mut geometry_lngs = Vec::new();
                    
                    if let Some(lats) = fb_edge.geometry_lats() {
                        for k in 0..lats.len() {
                            geometry_lats.push(lats.get(k));
                        }
                    }
                    
                    if let Some(lngs) = fb_edge.geometry_lngs() {
                        for k in 0..lngs.len() {
                            geometry_lngs.push(lngs.get(k));
                        }
                    }
                    
                    let lats_vec = builder.create_vector(&geometry_lats);
                    let lngs_vec = builder.create_vector(&geometry_lngs);
                    
                    // Extract tags
                    let mut tag_offsets = Vec::new();
                    if let Some(tags) = fb_edge.tags() {
                        for k in 0..tags.len() {
                            let tag = tags.get(k);
                            let key = tag.key().unwrap_or("").to_string();
                            let value = tag.value().unwrap_or("").to_string();
                            
                            let key_offset = builder.create_string(&key);
                            let value_offset = builder.create_string(&value);
                            
                            let tag_args = KeyValueArgs {
                                key: Some(key_offset),
                                value: Some(value_offset),
                            };
                            
                            let tag_offset = KeyValue::create(&mut builder, &tag_args);
                            tag_offsets.push(tag_offset);
                        }
                    }
                    
                    let tags_vec = builder.create_vector(&tag_offsets);
                    
                    // Create edge
                    let edge_args = EdgeArgs {
                        id: Some(id_offset),
                        source_node_id: Some(source_id_offset),
                        destination_node_id: Some(dest_id_offset),
                        name: Some(name_offset),
                        osm_way_id: fb_edge.osm_way_id(),
                        travel_costs: Some(costs_vec),
                        geometry_lats: Some(lats_vec),
                        geometry_lngs: Some(lngs_vec),
                        tags: Some(tags_vec),
                    };
                    
                    let edge_offset = Edge::create(&mut builder, &edge_args);
                    builder.finish(edge_offset, None);
                    
                    // Get the buffer data and add it to the cell - make sure we clone the data
                    let buf = builder.finished_data().to_vec();
                    cell.add_edge(buf);
                }
            }
        }
    }
    
    Ok(map_data)
} 