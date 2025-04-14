use tonic::{transport::Server, Request, Response, Status};
use flatbuffers::root;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;
use std::path::Path;

use tobmaprouteapi::route_service_server::{RouteService, RouteServiceServer};
use tobmaprouteapi::{RouteRequest, RouteResponse, Path as RoutePath};
use crate::snap::tobmapapi::Location;

pub mod tobmaprouteapi {
    tonic::include_proto!("tobmaprouteapi");
}

use crate::snap::tobmapgraph;
use crate::snap::tobmapgraph::RoadInteraction;

#[derive(Debug)]
pub struct MyRouteService {
    graph_blob: Option<tobmapgraph::GraphBlob<'static>>,
    graph_data: Option<Vec<u8>>, 
}

impl Default for MyRouteService {
    fn default() -> Self {
        Self { 
            graph_blob: None,
            graph_data: None,
        }
    }
}

impl MyRouteService {
    pub fn new<P: AsRef<Path>>(graph_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let graph_data = std::fs::read(graph_path)?;
        
        let graph_blob = unsafe {
            let data_ptr = graph_data.as_ptr();
            let data_slice = std::slice::from_raw_parts(data_ptr, graph_data.len());
            root::<tobmapgraph::GraphBlob>(data_slice).map_err(|e| e.to_string())?
        };
        
        Ok(Self { 
            graph_blob: Some(graph_blob),
            graph_data: Some(graph_data),
        })
    }

    fn calculate_edge_cost(&self, edge_id: u32) -> u32 {
        if let Some(graph_blob) = &self.graph_blob {
            if let Some(edges) = graph_blob.edges() {
                if edge_id as usize < edges.len() {
                    let edge = edges.get(edge_id as usize);
                    return (edge.costs_and_flags() as u32) << 3;
                }
            }
        }
        u32::MAX
    }
    
    fn calculate_interaction_cost(&self, node_idx: u32, incoming_edge: u32, outgoing_edge: u32) -> u32 {
        if let Some(graph_blob) = &self.graph_blob {
            if let Some(nodes) = graph_blob.nodes() {
                if node_idx as usize < nodes.len() {
                    let node = unsafe { nodes.get(node_idx as usize) };
                    
                    if let Some(node_edges) = node.edges() {
                        let mut incoming_pos = None;
                        let mut outgoing_pos = None;
                        
                        for i in 0..node_edges.len() {
                            let edge_id = node_edges.get(i);
                            if edge_id == incoming_edge {
                                incoming_pos = Some(i);
                            }
                            if edge_id == outgoing_edge {
                                outgoing_pos = Some(i);
                            }
                        }
                        
                        if let (Some(in_pos), Some(out_pos)) = (incoming_pos, outgoing_pos) {
                            if let Some(interactions) = node.interactions() {
                                if in_pos < interactions.len() {
                                    let interaction = interactions.get(in_pos);
                                    match interaction.outgoing() {
                                        RoadInteraction::None => return 2,
                                        RoadInteraction::Yield => return 4,
                                        RoadInteraction::StopSign => return 8,
                                        RoadInteraction::TrafficLight => return 32,
                                        _ => return 0,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        2
    }
    
    fn get_adjacent_edges(&self, edge_id: u32, node_idx: u32) -> Vec<u32> {
        let mut adjacent = Vec::new();
        
        if let Some(graph_blob) = &self.graph_blob {
            if let Some(nodes) = graph_blob.nodes() {
                if node_idx as usize < nodes.len() {
                    let node = unsafe { nodes.get(node_idx as usize) };
                    
                    if let Some(node_edges) = node.edges() {
                        for i in 0..node_edges.len() {
                            let adj_edge_id = node_edges.get(i);
                            if adj_edge_id != edge_id {
                                adjacent.push(adj_edge_id);
                            }
                        }
                    }
                }
            }
        }
        
        adjacent
    }
    
    fn find_paths(&self, start_edge_id: u32, end_edge_id: u32, max_paths: usize) -> Vec<Vec<u32>> {
        let mut result_paths = Vec::new();
        
        if let Some(shortest_path) = self.find_shortest_path(start_edge_id, end_edge_id, &HashSet::new()) {
            result_paths.push(shortest_path);
        } else {
            return result_paths;
        }
        
        if max_paths > 1 {
            let mut used_edges = HashSet::new();
            for edge in &result_paths[0] {
                used_edges.insert(*edge);
            }
            
            for _ in 1..max_paths {
                if let Some(path) = self.find_shortest_path(start_edge_id, end_edge_id, &used_edges) {
                    for edge in &path {
                        used_edges.insert(*edge);
                    }
                    result_paths.push(path);
                } else {
                    break;
                }
            }
        }
        
        result_paths
    }
    
    fn find_shortest_path(&self, start_edge_id: u32, end_edge_id: u32, avoid_edges: &HashSet<u32>) -> Option<Vec<u32>> {
        let graph_blob = self.graph_blob.as_ref()?;
        let edges = graph_blob.edges()?;
        
        if start_edge_id as usize >= edges.len() || end_edge_id as usize >= edges.len() {
            return None;
        }
        
        let mut distances: HashMap<u32, u32> = HashMap::new();
        let mut prev_edge: HashMap<u32, u32> = HashMap::new();
        let mut pq = BinaryHeap::new();
        
        distances.insert(start_edge_id, 0);
        pq.push((Reverse(0), start_edge_id));
        
        while let Some((Reverse(cost), current_edge)) = pq.pop() {
            if current_edge == end_edge_id {
                return Some(self.reconstruct_path(start_edge_id, end_edge_id, &prev_edge));
            }
            
            if let Some(&best_cost) = distances.get(&current_edge) {
                if cost > best_cost {
                    continue;
                }
            }
            
            let edge = edges.get(current_edge as usize);
            let node1 = edge.point_1_node_idx();
            let node2 = edge.point_2_node_idx();
            
            for &node_idx in &[node1, node2] {
                let adjacent_edges = self.get_adjacent_edges(current_edge, node_idx);
                
                for &next_edge in &adjacent_edges {
                    if avoid_edges.contains(&next_edge) && next_edge != end_edge_id {
                        continue;
                    }
                    
                    let edge_cost = self.calculate_edge_cost(next_edge);
                    let interaction_cost = self.calculate_interaction_cost(node_idx, current_edge, next_edge);
                    let next_cost = cost + edge_cost + interaction_cost;
                    
                    let is_better_path = match distances.get(&next_edge) {
                        Some(&existing_cost) => next_cost < existing_cost,
                        None => true,
                    };
                    
                    if is_better_path {
                        distances.insert(next_edge, next_cost);
                        prev_edge.insert(next_edge, current_edge);
                        pq.push((Reverse(next_cost), next_edge));
                    }
                }
            }
        }
        
        None
    }
    
    fn reconstruct_path(&self, start_edge_id: u32, end_edge_id: u32, prev_edge: &HashMap<u32, u32>) -> Vec<u32> {
        let mut path = Vec::new();
        let mut current = end_edge_id;
        
        while current != start_edge_id {
            path.push(current);
            match prev_edge.get(&current) {
                Some(&prev) => current = prev,
                None => return Vec::new(),
            }
        }
        
        path.push(start_edge_id);
        path.reverse();
        path
    }
}

#[tonic::async_trait]
impl RouteService for MyRouteService {
    async fn route(
        &self,
        request: Request<RouteRequest>,
    ) -> Result<Response<RouteResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();
        
        if self.graph_blob.is_none() {
            return Err(Status::unavailable("Graph data not loaded"));
        }
        
        let start_edge_id = req.start_edge;
        let end_edge_id = req.end_edge;
        
        let num_paths = req.num_paths.unwrap_or(3) as usize;
        let paths = self.find_paths(start_edge_id, end_edge_id, num_paths);
        
        let result_paths = paths.into_iter()
            .map(|edge_path| RoutePath { edges: edge_path })
            .collect();
        
        let reply = RouteResponse {
            paths: result_paths,
        };

        Ok(Response::new(reply))
    }
}