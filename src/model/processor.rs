use anyhow::{Context, Result};
use log::info;
use osmpbf::{Element, ElementReader};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::model::{Edge, MapData, Node, TravelMode};

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
            let node = Node::new(lat as f32, lon as f32, s2_cell_id);
            graph_nodes.insert(node_id, node.id.clone());
            
            let cell = map_data.get_or_create_cell(s2_cell_id);
            cell.add_node(node);
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
                            
                            // Create an edge
                            let mut edge = Edge::new(
                                source_graph_id.clone(),
                                target_graph_id.clone(),
                                way.id as u64,
                            );
                            
                            // Set way name if available
                            if let Some(name) = way.tags.get("name") {
                                edge.name = name.clone();
                            }
                            
                            // Add all points to the edge geometry
                            for &(lat, lon) in &current_path {
                                edge.add_point(lat, lon);
                            }
                            
                            // Calculate travel costs based on way type
                            calculate_travel_costs(&mut edge, way);
                            
                            // Add OSM tags to the edge
                            for (key, value) in &way.tags {
                                edge.tags.insert(key.clone(), value.clone());
                            }
                            
                            // Add the edge to the appropriate cell
                            // For simplicity, use the cell of the source node
                            if let Some(&(source_lat, source_lon)) = osm_nodes.get(&source_id) {
                                let cell_id = get_s2_cell_id(source_lat, source_lon);
                                let cell = map_data.get_or_create_cell(cell_id);
                                cell.add_edge(edge);
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
fn calculate_travel_costs(edge: &mut Edge, way: &OsmWay) {
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
    for i in 1..edge.geometry_lats.len() {
        let lat1 = edge.geometry_lats[i-1] as f64;
        let lon1 = edge.geometry_lngs[i-1] as f64;
        let lat2 = edge.geometry_lats[i] as f64;
        let lon2 = edge.geometry_lngs[i] as f64;
        
        length += haversine_distance(lat1, lon1, lat2, lon2);
    }
    
    // Convert speeds to travel times in seconds
    let length_km = length / 1000.0;
    
    if car_speed > 0.0 {
        let car_time = (length_km / car_speed) * 3600.0;
        edge.set_travel_cost(TravelMode::Car, car_time as f32);
    }
    
    if bike_speed > 0.0 {
        let bike_time = (length_km / bike_speed) * 3600.0;
        edge.set_travel_cost(TravelMode::Bike, bike_time as f32);
    }
    
    if walk_speed > 0.0 {
        let walk_time = (length_km / walk_speed) * 3600.0;
        edge.set_travel_cost(TravelMode::Walk, walk_time as f32);
    }
    
    if transit_speed > 0.0 {
        let transit_time = (length_km / transit_speed) * 3600.0;
        edge.set_travel_cost(TravelMode::Transit, transit_time as f32);
    }
}

/// Calculate the distance between two points using the Haversine formula
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6371.0; // km
    
    let lat1 = lat1.to_radians();
    let lon1 = lon1.to_radians();
    let lat2 = lat2.to_radians();
    let lon2 = lon2.to_radians();
    
    let dlat = lat2 - lat1;
    let dlon = lon2 - lon1;
    
    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    
    EARTH_RADIUS * c * 1000.0 // Convert to meters
}

/// Helper struct to store OSM way data
struct OsmWay {
    id: i64,
    node_ids: Vec<i64>,
    tags: HashMap<String, String>,
} 