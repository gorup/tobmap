use std::collections::HashMap;
use flatbuffers;

pub mod flatbuffer;
pub mod processor;

// Import the types from the generated module
use crate::generated::tobmap::{TravelMode, Node, Edge};

/// A cell contains all nodes and edges within a specific S2 cell
#[derive(Debug, Clone)]
pub struct Cell {
    pub s2_cell_id: u64,
    // Store buffer data
    pub node_buffers: Vec<Vec<u8>>,
    pub edge_buffers: Vec<Vec<u8>>,
}

impl Cell {
    pub fn new(s2_cell_id: u64) -> Self {
        Self {
            s2_cell_id,
            node_buffers: Vec::new(),
            edge_buffers: Vec::new(),
        }
    }
    
    pub fn add_node(&mut self, buffer: Vec<u8>) {
        self.node_buffers.push(buffer);
    }
    
    pub fn add_edge(&mut self, buffer: Vec<u8>) {
        self.edge_buffers.push(buffer);
    }
    
    pub fn nodes(&self) -> Vec<Node<'_>> {
        self.node_buffers.iter()
            .map(|buf| unsafe { flatbuffers::root_unchecked::<Node>(buf) })
            .collect()
    }
    
    pub fn edges(&self) -> Vec<Edge<'_>> {
        self.edge_buffers.iter()
            .map(|buf| unsafe { flatbuffers::root_unchecked::<Edge>(buf) })
            .collect()
    }
}

/// The root object containing all the map data organized by S2 cells
#[derive(Debug, Clone)]
pub struct MapData {
    /// Metadata
    pub version: String,
    pub osm_data_date: String,
    pub generation_date: String,
    
    /// All cells in the map
    pub cells: HashMap<u64, Cell>,
}

impl MapData {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            osm_data_date: String::new(),
            generation_date: chrono::Utc::now().to_rfc3339(),
            cells: HashMap::new(),
        }
    }
    
    pub fn get_or_create_cell(&mut self, s2_cell_id: u64) -> &mut Cell {
        if !self.cells.contains_key(&s2_cell_id) {
            self.cells.insert(s2_cell_id, Cell::new(s2_cell_id));
        }
        
        self.cells.get_mut(&s2_cell_id).unwrap()
    }
} 
