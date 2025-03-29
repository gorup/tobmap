use anyhow::{Context, Result};
use log::info;
use osmpbf::{Element, ElementReader};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;
use flatbuffers::FlatBufferBuilder;

use crate::model::MapData;
use crate::generated::tobmap::{TravelMode, Node, Edge, NodeArgs, EdgeArgs, KeyValue, KeyValueArgs};

// Constants for S2 cell handling
const S2_CELL_LEVEL: u64 = 15; // This level gives cells ~300m across

/// Process an OpenStreetMap PBF file and convert it to our graph model
pub fn process_osm_file<P: AsRef<Path>>(file_path: P) -> Result<MapData> {
    let path = file_path.as_ref();
    info!("Processing OSM file: {}", path.display());
    
    let reader = ElementReader::from_path(path)
        .context("Failed to create element reader")?;
    
    // First pass: collect all nodes and ways
    let mut osm_nodes = HashMap::new();
    let mut osm_ways = Vec::new();
    
    info!("First pass: collecting nodes and ways");
    reader.for_each(|element| {
        match element {
            Element::Node(node) => {
                osm_nodes.insert(node.id(), (node.lat(), node.lon()));
            },
            Element::Way(way) => {
                // Only process ways that are roads or paths
                if is_routable_way(&way) {
                    let way_id = way.id();
                    let node_ids = way.refs().collect::<Vec<_>>();
                    let tags = way.tags().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                    
                    osm_ways.push(OsmWay {
                        id: way_id,
                        node_ids,
                        tags,
                    });
                }
            },
            _ => {}
        }
    })?;
    
    info!("Collected {} nodes and {} ways", osm_nodes.len(), osm_ways.len());
    
    // Second pass: identify intersections (nodes where 3+ ways meet or endpoints)
    let mut node_way_count = HashMap::new();
    
    for way in &osm_ways {
        for &node_id in &way.node_ids {
            *node_way_count.entry(node_id).or_insert(0) += 1;
        }
    }
    
    let intersection_node_ids = node_way_count.iter()
        .filter_map(|(&node_id, &count)| {
            if count >= 3 {
                Some(node_id)
            } else {
                // Also include endpoints of ways
                for way in &osm_ways {
                    if way.node_ids.first() == Some(&node_id) || way.node_ids.last() == Some(&node_id) {
                        return Some(node_id);
                    }
                }
                None
            }
        })
        .collect::<HashSet<_>>();
    
    info!("Identified {} intersection nodes", intersection_node_ids.len());
    
    // Create graph nodes for intersections
    let mut map_data = MapData::new();
    let mut graph_nodes = HashMap::new();
    
    for &node_id in &intersection_node_ids {
        if let Some(&(lat, lon)) = osm_nodes.get(&node_id) {
            let s2_cell_id = get_s2_cell_id(lat, lon);
            
            // Create a new FlatBufferBuilder for this node
            let mut builder = FlatBufferBuilder::new();
            let node_id_str = Uuid::new_v4().to_string();
            let node_id_offset = builder.create_string(&node_id_str);
            
            // Create the Node object
            let args = NodeArgs {
                id: Some(node_id_offset),
                s2_cell_id,
                lat: lat as f32,
                lng: lon as f32,
            };
            
            let fb_node = Node::create(&mut builder, &args);
            builder.finish(fb_node, None);
            
            // Get the finished buffer
            let buf = builder.finished_data().to_vec();
            
            // Store the node ID for later references
            graph_nodes.insert(node_id, node_id_str);
            
            // Add the buffer to the cell
            let cell = map_data.get_or_create_cell(s2_cell_id);
            cell.add_node(buf);
        }
    }
    
    // Process ways to create edges between intersections
    info!("Creating edges between intersections");
    
    for way in &osm_ways {
        process_way(&mut map_data, way, &osm_nodes, &intersection_node_ids, &graph_nodes);
    }
    
    info!("Generated a graph with {} cells", map_data.cells.len());
    
    Ok(map_data)
}

/// Determine if a way is routable (road, path, etc.)
fn is_routable_way(way: &osmpbf::Way) -> bool {
    let has_highway_tag = way.tags().any(|(k, _)| k == "highway");
    let is_area = way.tags().any(|(k, v)| k == "area" && v == "yes");
    
    has_highway_tag && !is_area
}

/// Get the S2 cell ID for a lat/lng point
fn get_s2_cell_id(lat: f64, lng: f64) -> u64 {
    let latlng = LatLng::from_degrees(lat, lng);
    let cell_id = CellID::from(latlng).parent(S2_CELL_LEVEL);
    cell_id.0
}

/// Process a way to create edges between intersections
fn process_way(
    map_data: &mut MapData,
    way: &OsmWay,
    osm_nodes: &HashMap<i64, (f64, f64)>,
    intersection_node_ids: &HashSet<i64>,
    graph_nodes: &HashMap<i64, String>,
) {
    let node_ids = &way.node_ids;
    if node_ids.len() < 2 {
        return;
    }
    
    let mut current_path = Vec::new();
    let mut current_source_id = None;
    
    for (_idx, &node_id) in node_ids.iter().enumerate() {
        let is_intersection = intersection_node_ids.contains(&node_id);
        
        if let Some(&(lat, lon)) = osm_nodes.get(&node_id) {
            // Add point to the current path
            current_path.push((lat as f32, lon as f32));
            
            if is_intersection {
                if let Some(_graph_node_id) = graph_nodes.get(&node_id) {
                    if let Some(source_id) = current_source_id {
                        // We have a path from source to this intersection
                        if let (Some(source_graph_id), Some(target_graph_id)) = 
                            (graph_nodes.get(&source_id), graph_nodes.get(&node_id)) {
                            
                            // Create a new FlatBufferBuilder for this edge
                            let mut builder = FlatBufferBuilder::new();
                            
                            // Create string offsets
                            let edge_id = Uuid::new_v4().to_string();
                            let edge_id_offset = builder.create_string(&edge_id);
                            let source_id_offset = builder.create_string(source_graph_id);
                            let target_id_offset = builder.create_string(target_graph_id);
                            
                            // Get the name if available
                            let name_string = way.tags.get("name").cloned().unwrap_or_else(String::new);
                            let name_offset = builder.create_string(&name_string);
                            
                            // Extract geometry points
                            let mut geometry_lats = Vec::new();
                            let mut geometry_lngs = Vec::new();
                            
                            for &(lat, lon) in &current_path {
                                geometry_lats.push(lat);
                                geometry_lngs.push(lon);
                            }
                            
                            let lats_vec = builder.create_vector(&geometry_lats);
                            let lngs_vec = builder.create_vector(&geometry_lngs);
                            
                            // Calculate travel costs
                            let mut travel_costs = vec![-1.0; 4]; // One for each TravelMode
                            calculate_travel_costs(&mut travel_costs, way, &geometry_lats, &geometry_lngs);
                            let costs_vec = builder.create_vector(&travel_costs);
                            
                            // Create tags vector
                            let mut tag_offsets = Vec::new();
                            for (key, value) in &way.tags {
                                let key_offset = builder.create_string(key);
                                let value_offset = builder.create_string(value);
                                
                                let tag_args = KeyValueArgs {
                                    key: Some(key_offset),
                                    value: Some(value_offset),
                                };
                                
                                let tag = KeyValue::create(&mut builder, &tag_args);
                                tag_offsets.push(tag);
                            }
                            
                            let tags_vec = builder.create_vector(&tag_offsets);
                            
                            // Create the Edge object
                            let args = EdgeArgs {
                                id: Some(edge_id_offset),
                                source_node_id: Some(source_id_offset),
                                destination_node_id: Some(target_id_offset),
                                name: Some(name_offset),
                                osm_way_id: way.id as u64,
                                travel_costs: Some(costs_vec),
                                geometry_lats: Some(lats_vec),
                                geometry_lngs: Some(lngs_vec),
                                tags: Some(tags_vec),
                            };
                            
                            let fb_edge = Edge::create(&mut builder, &args);
                            builder.finish(fb_edge, None);
                            
                            // Get the finished buffer
                            let buf = builder.finished_data().to_vec();
                            
                            // Add the edge to the appropriate cell
                            // For simplicity, use the cell of the source node
                            if let Some(&(source_lat, source_lon)) = osm_nodes.get(&source_id) {
                                let cell_id = get_s2_cell_id(source_lat, source_lon);
                                let cell = map_data.get_or_create_cell(cell_id);
                                cell.add_edge(buf);
                            }
                        }
                    }
                    
                    // Start a new path from this intersection
                    current_source_id = Some(node_id);
                    current_path.clear();
                    current_path.push((lat as f32, lon as f32));
                }
            }
        }
    }
}

/// Calculate travel costs for different travel modes
fn calculate_travel_costs(travel_costs: &mut Vec<f32>, way: &OsmWay, geometry_lats: &[f32], geometry_lngs: &[f32]) {
    // Default speeds in km/h for different highway types
    let highway_type = way.tags.get("highway").map(|s| s.as_str()).unwrap_or("");
    
    // Car costs
    let car_speed = match highway_type {
        "motorway" => 110.0,
        "trunk" => 90.0,
        "primary" => 70.0,
        "secondary" => 60.0,
        "tertiary" => 50.0,
        "residential" => 30.0,
        "service" => 20.0,
        _ => -1.0, // Not allowed
    };
    
    // Bike costs
    let bike_speed = match highway_type {
        "path" | "track" | "cycleway" => 15.0,
        "residential" | "living_street" => 12.0,
        "tertiary" | "unclassified" => 10.0,
        "footway" | "pedestrian" => 8.0,
        "primary" | "secondary" => 8.0,
        _ => if car_speed > 0.0 { 10.0 } else { -1.0 },
    };
    
    // Walking costs
    let walk_speed = match highway_type {
        "footway" | "pedestrian" | "path" | "track" | "steps" => 5.0,
        "residential" | "living_street" => 4.0,
        _ => if bike_speed > 0.0 { 4.0 } else { -1.0 },
    };
    
    // Transit costs (simplified - in a real system this would be based on actual transit schedules)
    let transit_speed = match highway_type {
        "primary" | "secondary" | "tertiary" => 30.0,
        _ => -1.0, // Not accessible by transit
    };
    
    // Calculate edge length
    let mut length = 0.0;
    for i in 1..geometry_lats.len() {
        let lat1 = geometry_lats[i-1] as f64;
        let lon1 = geometry_lngs[i-1] as f64;
        let lat2 = geometry_lats[i] as f64;
        let lon2 = geometry_lngs[i] as f64;
        
        length += haversine_distance(lat1, lon1, lat2, lon2);
    }
    
    // Convert speeds to travel times in seconds
    let length_km = length / 1000.0;
    
    if car_speed > 0.0 {
        let car_time = (length_km / car_speed) * 3600.0;
        travel_costs[TravelMode::Car.0 as usize] = car_time as f32;
    }
    
    if bike_speed > 0.0 {
        let bike_time = (length_km / bike_speed) * 3600.0;
        travel_costs[TravelMode::Bike.0 as usize] = bike_time as f32;
    }
    
    if walk_speed > 0.0 {
        let walk_time = (length_km / walk_speed) * 3600.0;
        travel_costs[TravelMode::Walk.0 as usize] = walk_time as f32;
    }
    
    if transit_speed > 0.0 {
        let transit_time = (length_km / transit_speed) * 3600.0;
        travel_costs[TravelMode::Transit.0 as usize] = transit_time as f32;
    }
}

/// Calculate the distance between two points using the Haversine formula
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6371.0; // km
    
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();
    
    let a = (delta_lat / 2.0).sin().powi(2) + 
            lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    
    EARTH_RADIUS * c * 1000.0 // Convert to meters
}

/// Helper struct to store OSM way data
struct OsmWay {
    id: i64,
    node_ids: Vec<i64>,
    tags: HashMap<String, String>,
} 