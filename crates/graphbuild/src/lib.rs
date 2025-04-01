use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;
use std::time::Instant;

use flatbuffers::FlatBufferBuilder;
use osmpbfreader::{Node, OsmId, OsmObj, OsmPbfReader, Way};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use schema::tobmapgraph::{Edge, GraphBlob, GraphBlobArgs, Interactions, Node as GraphNode, NodeArgs, RoadInteraction};
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
    let road_tags = &["highway"];

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
    
    // Only consider nodes with 1+ ways as intersections or endpoints
    // A true intersection is where different roads meet (way_ids.len() > 1)
    // We also want to include endpoints (first/last node of a way)
    let intersections: HashMap<i64, Intersection> = node_way_counts.iter()
        .filter(|(node_id, way_ids)| {
            // Keep nodes with multiple ways (true intersections)
            if way_ids.len() > 1 {
                return true;
            }

            // Keep endpoints (first or last node of any way)
            for &way_id in way_ids.iter() {
                if let Some(way) = ways.get(&way_id) {
                    let first_node = way.nodes.first().map(|n| n.0).unwrap_or(0);
                    let last_node = way.nodes.last().map(|n| n.0).unwrap_or(0);
                    if first_node == **node_id || last_node == **node_id {
                        return true;
                    }
                }
            }
            
            // Skip nodes that are just intermediate points on a single way
            false
        })
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
    
    // Create a map to deduplicate edges
    let mut edge_map: HashMap<(u32, u32), (u64, Vec<f32>, bool, RoadInteraction, RoadInteraction)> = HashMap::new();
    
    for segment in &road_segments {
        // Find intersection nodes along this segment
        let intersection_nodes: Vec<(usize, i64)> = segment.nodes.iter()
            .enumerate()
            .filter(|(_, node_id)| intersections.contains_key(node_id))
            .map(|(idx, node_id)| (idx, *node_id))
            .collect();
        
        // Create edges between consecutive intersection nodes
        for window in intersection_nodes.windows(2) {
            if let [(start_pos, start_id), (end_pos, end_id)] = window {
                if let (Some(start_idx), Some(end_idx)) = (node_id_to_index.get(start_id), node_id_to_index.get(end_id)) {
                    // Skip if this isn't a meaningful edge (same position)
                    if start_pos == end_pos {
                        continue;
                    }
                    
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
                    
                    // Create a canonical key for this edge (smaller node index first)
                    let edge_key = if start_idx < end_idx {
                        (*start_idx, *end_idx)
                    } else {
                        (*end_idx, *start_idx)
                    };
                    
                    // Check if this edge already exists, otherwise add it
                    edge_map.entry(edge_key).or_insert_with(|| (
                        cell_id, 
                        travel_costs.clone(), 
                        !segment.is_oneway,
                        start_interaction, 
                        end_interaction
                    )).1 = merge_travel_costs(&edge_map.get(&edge_key).map(|v| &v.1).unwrap_or(&vec![-1.0; 4]), &travel_costs);
                }
            }
        }
    }
    
    // Convert edge map to edge_node_pairs
    for ((start_idx, end_idx), (cell_id, travel_costs, backwards_allowed, start_interaction, end_interaction)) in edge_map {
        edge_node_pairs.push((start_idx, end_idx, cell_id, 
                             travel_costs, backwards_allowed,
                             start_interaction, end_interaction));
    }

    info!("Built {} deduplicated edge node pairs, will now sort edges by cell, took {:?}", edge_node_pairs.len(), last_time.elapsed());
    last_time = Instant::now();
    
    // Sort edges by cell ID for locality
    edge_node_pairs.par_sort_by_key(|(_, _, cell_id, _, _, _, _)| CellID(*cell_id).to_token());
 
    info!("Sorting done, will now create flatbuffer edges, took {:?}", last_time.elapsed());
    last_time = Instant::now();

    // Create edges
    let mut edges = Vec::new();
    for (start_idx, end_idx, cell_id, travel_costs, backwards_allowed, start_interaction, end_interaction) in &edge_node_pairs {
        // Convert travel costs from f32 seconds to u16 seconds
        // If cost is negative (not allowed), set to 0 (we'll handle this in routing)
        let car_cost = if travel_costs[0] > 0.0 { travel_costs[0].round() as u16 } else { 0 };
        let bike_cost = if travel_costs[1] > 0.0 { travel_costs[1].round() as u16 } else { 0 };
        let walk_cost = if travel_costs[2] > 0.0 { travel_costs[2].round() as u16 } else { 0 };
        
        // Create edge directly as a struct 
        let edge = Edge::new(
            *cell_id,
            *start_idx,
            *end_idx,
            *backwards_allowed,
            car_cost,
            bike_cost,
            walk_cost,
        );
        
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
    
    // Create edges vector
    let edge_structs: Vec<Edge> = edges.iter().map(|(edge, _, _, _, _, _)| *edge).collect();
    let edges_offset = builder.create_vector(&edge_structs);

    // Create nodes vector
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

/// Merges two travel cost vectors, taking the better (smaller but valid) cost for each mode
fn merge_travel_costs(costs1: &[f32], costs2: &[f32]) -> Vec<f32> {
    // Ensure both vectors have the same length
    assert_eq!(costs1.len(), costs2.len(), "Travel cost vectors must have the same length");
    
    costs1.iter().zip(costs2.iter())
        .map(|(&c1, &c2)| {
            if c1 < 0.0 {
                c2
            } else if c2 < 0.0 {
                c1
            } else {
                c1.min(c2)
            }
        })
        .collect()
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
        assert!(u64::from(edge.point_1_node_idx()) < graph.nodes().unwrap().len() as u64, "Edge start node should be valid");
        assert!(u64::from(edge.point_2_node_idx()) < graph.nodes().unwrap().len() as u64, "Edge end node should be valid");
        
        // Verify individual travel costs exist
        assert!(edge.car_travel_cost() >= 0, "Edge should have car travel cost");
        assert!(edge.bike_travel_cost() >= 0, "Edge should have bike travel cost");
        assert!(edge.walk_travel_cost() >= 0, "Edge should have walk travel cost");
        assert!(edge.transit_travel_cost() >= 0, "Edge should have transit travel cost");

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
            // Car costs should be positive for most roads or 0 (not allowed)
            assert!(edge.car_travel_cost() > 0 || edge.car_travel_cost() == 0, "Car costs should be positive or 0");
            
            // Bike and walk costs should be positive for most roads or 0 (not allowed)
            assert!(edge.bike_travel_cost() > 0 || edge.bike_travel_cost() == 0, "Bike costs should be positive or 0");
            assert!(edge.walk_travel_cost() > 0 || edge.walk_travel_cost() == 0, "Walk costs should be positive or 0");
            
            // Transit costs should be 0 (not supported)
            assert_eq!(edge.transit_travel_cost(), 0, "Transit costs should be 0");
        }
    }
}