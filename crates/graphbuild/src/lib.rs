use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

use flatbuffers::FlatBufferBuilder;
use osmpbfreader::{Node, OsmId, OsmObj, OsmPbfReader, Way};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use schema::tobmapgraph::{Edge, EdgeArgs, GraphBlob, GraphBlobArgs, Interactions, Node as GraphNode, NodeArgs, RoadInteraction};
use thiserror::Error;
use log::info;
use rayon::prelude::*;


#[derive(Error, Debug)]
pub enum GraphBuildError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("OSM error: {0}")]
    OsmError(String),
    
    #[error("Processing error: {0}")]
    ProcessingError(String),
}

pub type StatusOr<T> = Result<T, GraphBuildError>;

/// A basic speed model for different road types (in km/h)
struct SpeedModel {
    car: f64,
    bike: f64,
    walk: f64,
}

impl Default for SpeedModel {
    fn default() -> Self {
        Self {
            car: 0.0,
            bike: 0.0,
            walk: 0.0,
        }
    }
}

/// Represents an intersection between roads
#[allow(dead_code)]
struct Intersection {
    location: LatLng,
    ways: HashSet<i64>,
    cell_id: u64,
}

/// Represents a way (road, path, etc.) in the map
#[allow(dead_code)]
struct RoadSegment {
    id: i64,
    nodes: Vec<i64>,
    speed_model: SpeedModel,
    is_oneway: bool,
    interactions: HashMap<i64, RoadInteraction>,
}

/// Parses OSM PBF data and returns a GraphBlob
/// 
/// The function processes the OpenStreetMap data to create a graph representation
/// with nodes (intersections) and edges (road segments).
///
/// # Arguments
/// * `osm_data` - Slice of bytes containing OSM PBF data
///
/// # Returns
/// * `StatusOr<Vec<u8>>` - Result containing the serialized graph data or an error
pub fn osm_to_graph_blob(osm_data: &[u8]) -> StatusOr<Vec<u8>> {
    let mut reader = OsmPbfReader::new(std::io::Cursor::new(osm_data));

    let mut last_time = Instant::now();
    
    info!("Reading OSM data...");
    
    // Use get_objs_and_deps to get all highways and their nodes in a single pass
    let road_tags = &["highway", "road", "street", "primary", "secondary", "tertiary", "residential", "service", "trunk"];

    info!("Loading highways and nodes...");
    let objects = reader.get_objs_and_deps(|obj| {
        match obj {
            OsmObj::Way(way) => way.tags.keys().any(|tag| road_tags.contains(&tag.as_str())),
            _ => false
        }
    }).map_err(|e| GraphBuildError::OsmError(e.to_string()))?;
    
    // Extract ways and nodes from the objects
    let mut ways: HashMap<i64, Way> = HashMap::new();
    let mut nodes: HashMap<i64, Node> = HashMap::new();
    
    for (id, obj) in objects {
        match obj {
            OsmObj::Way(way) => {
                let way_id = match id {
                    OsmId::Way(id) => id.0,
                    _ => continue, // Skip if not matching the correct type
                };
                ways.insert(way_id, way);
            },
            OsmObj::Node(node) => {
                let node_id = match id {
                    OsmId::Node(id) => id.0,
                    _ => continue, // Skip if not matching the correct type
                };
                nodes.insert(node_id, node);
            },
            _ => {} // Ignore relations
        }
    }
    
    info!("Found {} ways and {} nodes", ways.len(), nodes.len());
    
    // Find intersections (nodes where multiple ways meet)
    let mut node_way_counts: HashMap<i64, HashSet<i64>> = HashMap::new();
    for (way_id, way) in &ways {
        for node_id in &way.nodes {
            node_way_counts
                .entry(node_id.0)
                .or_insert_with(HashSet::new)
                .insert(*way_id);
        }
    }
    
    // Nodes with 1+ ways are intersections or endpoints
    let intersections: HashMap<i64, Intersection> = node_way_counts.iter()
        .filter(|(_, way_ids)| way_ids.len() >= 1)
        .filter_map(|(node_id, way_ids)| {
            if let Some(node) = nodes.get(node_id) {
                let lat_lng = LatLng::from_degrees(node.lat(), node.lon());
                let cell_id = CellID::from(lat_lng).0;
                
                Some((*node_id, Intersection {
                    location: lat_lng,
                    ways: way_ids.clone(),
                    cell_id,
                }))
            } else {
                None
            }
        })
        .collect();
    
    info!("Found {} intersections", intersections.len());
    
    // Build road segments with speed models
    let mut road_segments: Vec<RoadSegment> = Vec::new();
    for (way_id, way) in &ways {
        // Parse speed model from tags
        let mut speed_model = SpeedModel::default();
        
        // Check if way is oneway
        let is_oneway = way.tags.get("oneway")
            .map(|v| v == "yes")
            .unwrap_or(false);
        
        // Default speeds based on road type
        if let Some(highway) = way.tags.get("highway") {
            match highway.as_str() {
                "motorway" | "motorway_link" => {
                    speed_model.car = 100.0;
                    speed_model.bike = -1.0; // Not allowed
                    speed_model.walk = -1.0; // Not allowed
                },
                "trunk" | "trunk_link" => {
                    speed_model.car = 80.0;
                    speed_model.bike = -1.0;
                    speed_model.walk = -1.0;
                },
                "primary" | "primary_link" => {
                    speed_model.car = 60.0;
                    speed_model.bike = 15.0;
                    speed_model.walk = 5.0;
                },
                "secondary" | "secondary_link" => {
                    speed_model.car = 50.0;
                    speed_model.bike = 15.0;
                    speed_model.walk = 5.0;
                },
                "tertiary" | "tertiary_link" => {
                    speed_model.car = 40.0;
                    speed_model.bike = 15.0;
                    speed_model.walk = 5.0;
                },
                "residential" | "unclassified" => {
                    speed_model.car = 30.0;
                    speed_model.bike = 15.0;
                    speed_model.walk = 5.0;
                },
                "service" => {
                    speed_model.car = 20.0;
                    speed_model.bike = 15.0;
                    speed_model.walk = 5.0;
                },
                "living_street" => {
                    speed_model.car = 10.0;
                    speed_model.bike = 10.0;
                    speed_model.walk = 5.0;
                },
                "pedestrian" => {
                    speed_model.car = -1.0;
                    speed_model.bike = 5.0;
                    speed_model.walk = 5.0;
                },
                "cycleway" => {
                    speed_model.car = -1.0;
                    speed_model.bike = 20.0;
                    speed_model.walk = 5.0;
                },
                "footway" | "path" | "steps" => {
                    speed_model.car = -1.0;
                    speed_model.bike = 5.0;
                    speed_model.walk = 5.0;
                },
                _ => {
                    speed_model.car = 30.0;
                    speed_model.bike = 15.0;
                    speed_model.walk = 5.0;
                },
            }
        }
        
        // Override with maxspeed tag if present
        if let Some(maxspeed) = way.tags.get("maxspeed") {
            if let Ok(speed) = maxspeed.parse::<f64>() {
                speed_model.car = speed;
            }
        }
        
        // Determine traffic control (traffic lights, stop signs, etc.)
        let mut interactions = HashMap::new();
        for node_id in &way.nodes {
            let interaction = if let Some(node) = nodes.get(&node_id.0) {
                // Check node tags for traffic signals and stop signs
                if let Some(highway) = node.tags.get("highway") {
                    match highway.as_str() {
                        "traffic_signals" => RoadInteraction::TrafficLight,
                        "stop" => RoadInteraction::StopSign,
                        "give_way" => RoadInteraction::Yield,
                        _ => RoadInteraction::None,
                    }
                } else {
                    RoadInteraction::None
                }
            } else {
                RoadInteraction::None
            };
            
            interactions.insert(node_id.0, interaction);
        }
        
        road_segments.push(RoadSegment {
            id: *way_id,
            nodes: way.nodes.iter().map(|n| n.0).collect(),
            speed_model,
            is_oneway,
            interactions,
        });
    }
    
    info!("Built {} road segments, will sort intersections by cell (took {:?})", road_segments.len(), last_time.elapsed());
    last_time = Instant::now();

    // Convert to GraphBlob format
    // First build a map of node IDs to their index in the final array
    let mut intersections_vec: Vec<(&i64, &Intersection)> = intersections.iter().collect();
    
    // Sort nodes by cell ID for locality
    intersections_vec.par_sort_by_key(|(_, intersection)| CellID(intersection.cell_id).to_token());
    
    info!("Sorting done intersections by cell, will now build edges, took {:?}", last_time.elapsed());
    last_time = Instant::now();

    let node_id_to_index: HashMap<i64, u32> = intersections_vec.iter()
        .enumerate()
        .map(|(idx, (node_id, _))| (**node_id, idx as u32))
        .collect();
    
    // Create FlatBufferBuilder
    let mut builder = FlatBufferBuilder::new();
    
    // Build edges
    let mut edge_node_pairs = Vec::new();
    
    for segment in &road_segments {
        // For each segment, create edges between consecutive intersection nodes
        let mut current_path = Vec::new();
        
        for node_id in &segment.nodes {
            if intersections.contains_key(node_id) {
                current_path.push(*node_id);
            }
        }
        
        // Create edges between consecutive intersection nodes
        for window in current_path.windows(2) {
            if let [start_id, end_id] = window {
                if let (Some(start_idx), Some(end_idx)) = (node_id_to_index.get(start_id), node_id_to_index.get(end_id)) {
                    // Calculate edge cost based on distance and speed
                    let start_node = &intersections[start_id];
                    let end_node = &intersections[end_id];
                    
                    // Get S2 distance in meters (radius earth meters)
                    let distance_meters = start_node.location.distance(&end_node.location).rad() * 6371000.0;
                    
                    // Calculate midpoint lat/lng and convert to cell ID
                    let midpoint = LatLng::from_degrees(
                        (start_node.location.lat.deg() + end_node.location.lat.deg()) / 2.0,
                        (start_node.location.lng.deg() + end_node.location.lng.deg()) / 2.0
                    );
                    let cell_id = CellID::from(midpoint).0;
                    
                    // Calculate travel costs for each mode
                    let mut travel_costs = vec![-1.0, -1.0, -1.0, -1.0]; // Default: not allowed
                    
                    // Car cost (in seconds)
                    if segment.speed_model.car > 0.0 {
                        travel_costs[0] = // Car index 
                            (distance_meters / (segment.speed_model.car * 1000.0 / 3600.0)) as f32;
                    }
                    
                    // Bike cost
                    if segment.speed_model.bike > 0.0 {
                        travel_costs[1] = // Bike index
                            (distance_meters / (segment.speed_model.bike * 1000.0 / 3600.0)) as f32;
                    }
                    
                    // Walk cost
                    if segment.speed_model.walk > 0.0 {
                        travel_costs[2] = // Walk index
                            (distance_meters / (segment.speed_model.walk * 1000.0 / 3600.0)) as f32;
                    }
                    
                    // Transit not supported in this implementation
                    
                    // Get road interactions
                    let start_interaction = segment.interactions.get(start_id).cloned().unwrap_or(RoadInteraction::None);
                    let end_interaction = segment.interactions.get(end_id).cloned().unwrap_or(RoadInteraction::None);
                    
                    edge_node_pairs.push((*start_idx as u32, *end_idx as u32, cell_id, 
                                         travel_costs.clone(), !segment.is_oneway,
                                         start_interaction, end_interaction));
                }
            }
        }
    }

    info!("Built {} edge node pairs, will now sort edges by cell, took {:?}", edge_node_pairs.len(), last_time.elapsed());
    last_time = Instant::now();
    
    // Sort edges by cell ID for locality
    edge_node_pairs.par_sort_by_key(|(_, _, cell_id, _, _, _, _)| CellID(*cell_id).to_token());
 
    info!("Sorting done, will now create flatbuffer edges, took {:?}", last_time.elapsed());
    last_time = Instant::now();

    // Create flatbuffer edges
    let mut edges = Vec::new();
    for (start_idx, end_idx, cell_id, travel_costs, backwards_allowed, start_interaction, end_interaction) in &edge_node_pairs {
        let travel_costs_offset = builder.create_vector(travel_costs);
        
        // Create edge arguments
        let mut edge_args = EdgeArgs::default();
        edge_args.cell_id = *cell_id;
        edge_args.point_1_node_idx = *start_idx;
        edge_args.point_2_node_idx = *end_idx;
        edge_args.backwards_allowed = *backwards_allowed;
        edge_args.travel_costs = Some(travel_costs_offset);
        
        let edge = Edge::create(&mut builder, &edge_args);
        
        edges.push((edge, *start_idx, *end_idx, *start_interaction, *end_interaction, *backwards_allowed));
    }
    
    info!("Built {} edges, will now build nodes with edges, took {:?}", edges.len(), last_time.elapsed());
    last_time = Instant::now();

    // Build nodes with edge references
    let mut nodes_with_edges: Vec<(i64, u64, Vec<usize>, Vec<(RoadInteraction, RoadInteraction)>)> = 
        intersections_vec.iter()
        .map(|(node_id, intersection)| (**node_id, intersection.cell_id, Vec::new(), Vec::new()))
        .collect();

    info!("Built nodes with edges, will now add edge references, edges num {} nodes_with_edges num {} took {:?}", edges.len(), nodes_with_edges.len(), last_time.elapsed());
    last_time = Instant::now();
    
    // Create a map from node index to position in nodes_with_edges
    let node_to_pos: HashMap<u32, u32> = nodes_with_edges.iter()
        .enumerate()
        .map(|(pos, (node_id, _, _, _))| (node_id_to_index[node_id], pos as u32))
        .collect();

    // Add edge references to nodes using direct map access
    for (edge_idx, (_, start_idx, end_idx, start_interaction, end_interaction, backwards_allowed)) in edges.iter().enumerate() {
        if let Some(&start_pos) = node_to_pos.get(&(*start_idx as u32)) {
            nodes_with_edges[start_pos as usize].2.push(edge_idx);
            nodes_with_edges[start_pos as usize].3.push((*start_interaction, *end_interaction));
        }
        
        if *backwards_allowed {
            if let Some(&end_pos) = node_to_pos.get(&(*end_idx as u32)) {
                nodes_with_edges[end_pos as usize].2.push(edge_idx);
                nodes_with_edges[end_pos as usize].3.push((*end_interaction, *start_interaction));
            }
        }
    }
    
    info!("Added edge references to nodes, will now sort nodes_with_edges by cell, took {:?}", last_time.elapsed());
    last_time = Instant::now();
    
    // Sort nodes by cell ID (again, for safety)
    nodes_with_edges.par_sort_by_key(|(_, cell_id, _, _)| CellID(*cell_id).to_token());
    
    info!("Sorting done, now create flatbuffer things, took {:?}", last_time.elapsed());
    last_time = Instant::now();

    // Create FlatBuffer nodes
    let mut graph_nodes = Vec::with_capacity(nodes_with_edges.len());
    
    for (_, cell_id, edge_indices, interactions) in nodes_with_edges {
        let edge_indices_u32: Vec<u32> = edge_indices.iter().map(|&i| i as u32).collect();
        let edge_indices_offset = builder.create_vector(&edge_indices_u32);
        
        let interaction_objects: Vec<Interactions> = interactions.iter()
            .map(|(incoming, outgoing)| {
                let mut interaction = Interactions::default();
                interaction.set_incoming(*incoming);
                interaction.set_outgoing(*outgoing);
                interaction
            })
            .collect();
        let interactions_offset = builder.create_vector(&interaction_objects);
        
        // Create node arguments
        let mut node_args = NodeArgs::default();
        node_args.cell_id = cell_id;
        node_args.edges = Some(edge_indices_offset);
        node_args.interactions = Some(interactions_offset);
        
        let node = GraphNode::create(&mut builder, &node_args);
        
        graph_nodes.push(node);
    }
    
    // We need different vector creation for edges and nodes since they contain ForwardsUOffset
    let _vector_start = builder.start_vector::<flatbuffers::ForwardsUOffset<Edge>>(edges.len());
    for i in (0..edges.len()).rev() {
        builder.push(edges[i].0);
    }
    let edges_offset = builder.end_vector(edges.len());

    let _vector_start = builder.start_vector::<flatbuffers::ForwardsUOffset<GraphNode>>(graph_nodes.len());
    for i in (0..graph_nodes.len()).rev() {
        builder.push(graph_nodes[i]);
    }
    let nodes_offset = builder.end_vector(graph_nodes.len());
    
    info!("Done, now wrapping up, edges num {} nodes num {} took {:?}", edges.len(), graph_nodes.len(), last_time.elapsed());
    last_time = Instant::now();

    // Create graph blob name
    let name_offset = builder.create_string("OSM Generated Graph");
    
    // Create graph blob arguments
    let mut graph_blob_args = GraphBlobArgs::default();
    graph_blob_args.name = Some(name_offset);
    graph_blob_args.edges = Some(edges_offset);
    graph_blob_args.nodes = Some(nodes_offset);
    
    // Build final graph blob
    let graph_blob = GraphBlob::create(&mut builder, &graph_blob_args);
    
    builder.finish(graph_blob, None);
    
    info!("Graph building complete!");
    
    // Return serialized data
    let finished_data = builder.finished_data().to_vec();
    
    Ok(finished_data)
}

/// Converts the serialized buffer to a GraphBlob reference
/// 
/// # Arguments
/// * `buffer` - Serialized flatbuffer data
///
/// # Returns
/// * `GraphBlob` - Reference to the graph data in the buffer
pub fn get_graph_blob(buffer: &[u8]) -> schema::tobmapgraph::GraphBlob {
    flatbuffers::root::<schema::tobmapgraph::GraphBlob>(buffer).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_tinytiny_graph_building() {
        // Read the test data file
        let test_data = fs::read("testdata/test_network.osm.pbf").expect("Failed to read test data file");
        
        // Build the graph
        let graph_data = osm_to_graph_blob(&test_data).expect("Failed to build graph");
        let graph = get_graph_blob(&graph_data);

        // Verify basic graph properties
        assert_eq!(graph.nodes().unwrap().len(), 15, "Should have 15 nodes");
        assert_eq!(graph.edges().unwrap().len(), 13, "Should have 13 edges");

        // Verify graph name
        assert_eq!(graph.name().unwrap(), "OSM Generated Graph");

        // Verify edge properties
        let edge = graph.edges().expect("Should have at least one edge").get(0);
        assert!(edge.cell_id() > 0, "Edge should have a valid cell ID");
        assert!(edge.point_1_node_idx() < graph.nodes().unwrap().len() as u64, "Edge start node should be valid");
        assert!(edge.point_2_node_idx() < graph.nodes().unwrap().len() as u64, "Edge end node should be valid");
        assert_eq!(edge.travel_costs().unwrap().len(), 4, "Edge should have costs for all travel modes");

        // Verify node properties
        let node = graph.nodes().expect("Should have at least one node").get(0);
        assert!(node.cell_id() > 0, "Node should have a valid cell ID");
        assert_eq!(node.edges().unwrap().len(), node.interactions().unwrap().len(), 
                  "Node should have matching number of edges and interactions");

        // Verify connectivity
        let node_edges = node.edges().unwrap();
        assert!(!node_edges.is_empty(), "Node should have at least one connected edge");
        
        // Verify travel costs are reasonable
        for edge in graph.edges().unwrap().iter() {
            let costs = edge.travel_costs().unwrap();
            assert_eq!(costs.len(), 4, "Each edge should have costs for all travel modes");
            
            // Car costs should be positive for most roads
            assert!(costs.get(0) > 0.0 || costs.get(0) == -1.0, "Car costs should be positive or -1");
            
            // Bike and walk costs should be positive for most roads
            assert!(costs.get(1) > 0.0 || costs.get(1) == -1.0, "Bike costs should be positive or -1");
            assert!(costs.get(2) > 0.0 || costs.get(2) == -1.0, "Walk costs should be positive or -1");
            
            // Transit costs should be -1 (not supported)
            assert_eq!(costs.get(3), -1.0, "Transit costs should be -1");
        }
    }
}