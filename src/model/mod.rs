use std::collections::HashMap;
use uuid::Uuid;

pub mod flatbuffer;
pub mod processor;

/// Travel modes for edge costs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TravelMode {
    Car = 0,
    Bike = 1,
    Walk = 2,
    Transit = 3,
}

impl TravelMode {
    pub fn as_index(&self) -> usize {
        *self as usize
    }
    
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(TravelMode::Car),
            1 => Some(TravelMode::Bike),
            2 => Some(TravelMode::Walk),
            3 => Some(TravelMode::Transit),
            _ => None,
        }
    }
    
    pub fn all() -> Vec<TravelMode> {
        vec![
            TravelMode::Car,
            TravelMode::Bike,
            TravelMode::Walk,
            TravelMode::Transit,
        ]
    }
}

/// A node represents a real-world intersection
#[derive(Debug, Clone)]
pub struct Node {
    /// Unique identifier for the node
    pub id: String,
    
    /// S2 cell identifier at a specific level
    pub s2_cell_id: u64,
    
    /// Latitude and longitude
    pub lat: f32,
    pub lng: f32,
}

impl Node {
    pub fn new(lat: f32, lng: f32, s2_cell_id: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            s2_cell_id,
            lat,
            lng,
        }
    }
}

/// An edge represents a way (street, path, etc.) between two nodes
#[derive(Debug, Clone)]
pub struct Edge {
    /// Unique identifier for the edge
    pub id: String,
    
    /// Source and destination node identifiers
    pub source_node_id: String,
    pub destination_node_id: String,
    
    /// Name of the way, if available
    pub name: String,
    
    /// Way identifier from OpenStreetMap
    pub osm_way_id: u64,
    
    /// Travel costs (time in seconds) for different travel modes
    /// A negative value indicates that this mode is not allowed
    pub travel_costs: Vec<f32>,
    
    /// The geometry of the edge as a series of lat/lng points
    pub geometry_lats: Vec<f32>,
    pub geometry_lngs: Vec<f32>,
    
    /// Tags from OpenStreetMap that might be useful
    pub tags: HashMap<String, String>,
}

impl Edge {
    pub fn new(source_node_id: String, destination_node_id: String, osm_way_id: u64) -> Self {
        // Create a new edge with travel costs for all travel modes
        // By default, set all costs to -1.0 (not allowed)
        let travel_costs = vec![-1.0; TravelMode::all().len()];
        
        Self {
            id: Uuid::new_v4().to_string(),
            source_node_id,
            destination_node_id,
            name: String::new(),
            osm_way_id,
            travel_costs,
            geometry_lats: Vec::new(),
            geometry_lngs: Vec::new(),
            tags: HashMap::new(),
        }
    }
    
    /// Set the travel cost for a specific travel mode
    pub fn set_travel_cost(&mut self, mode: TravelMode, cost: f32) {
        let index = mode.as_index();
        if index < self.travel_costs.len() {
            self.travel_costs[index] = cost;
        } else {
            // Resize the vector if needed
            self.travel_costs.resize(index + 1, -1.0);
            self.travel_costs[index] = cost;
        }
    }
    
    /// Get the travel cost for a specific travel mode
    pub fn get_travel_cost(&self, mode: TravelMode) -> f32 {
        let index = mode.as_index();
        if index < self.travel_costs.len() {
            self.travel_costs[index]
        } else {
            -1.0 // Not allowed by default
        }
    }
    
    /// Add a point to the geometry
    pub fn add_point(&mut self, lat: f32, lng: f32) {
        self.geometry_lats.push(lat);
        self.geometry_lngs.push(lng);
    }
}

/// A cell contains all nodes and edges within a specific S2 cell
#[derive(Debug, Clone)]
pub struct Cell {
    pub s2_cell_id: u64,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Cell {
    pub fn new(s2_cell_id: u64) -> Self {
        Self {
            s2_cell_id,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
    
    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }
    
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
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