use tonic::{transport::Server, Request, Response, Status};
use flatbuffers::root;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;
use std::path::Path;
use log::info;
use std::io::Read;
use tobmaprouteapi::route_service_server::{RouteService, RouteServiceServer};
use tobmaprouteapi::{RouteRequest, RouteResponse, Path as RoutePath};
// use crate::snap::tobmapapi::Location;
use schema::tobmapgraph;
use crate::route::tobmapgraph::RoadInteraction;
use std::fs::File;
pub mod tobmaprouteapi {
    tonic::include_proto!("tobmaprouteapi");
}
use schema::tobmapgraph::{GraphBlob, LocationBlob, DescriptionBlob};
use anyhow::{Context, Result, bail, Error};

#[derive(Debug)]
pub struct MyRouteService {
    graph_data: Option<Vec<u8>>,
}

impl Default for MyRouteService {
    fn default() -> Self {
        info!("Using default MyRouteService");
        Self {
            graph_data: None,
        }
    }
}

impl MyRouteService {
    pub fn new<P: AsRef<Path>>(graph_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Loading graph from {:?}", graph_path.as_ref());

        // Read and parse the graph file
        let mut graph_file = File::open(&graph_path)
        .with_context(|| "Failed to open graph file")?;

        let gbb = Vec::new(); // Renamed to avoid shadowing
        let mut s = Self {
            graph_data: Some(gbb),
        };

        let graph_buffer: &mut Vec<u8> = s.graph_data.as_mut().unwrap();

        graph_file.read_to_end(graph_buffer)
            .with_context(|| "Failed to read graph file")?;

        // Use get_root_with_opts instead of root for better error handling and custom verifier options
        let verifier_opts = flatbuffers::VerifierOptions {
            max_tables: 3_000_000_000, // 3 billion tables
            ..Default::default()
        };

        // Verify the buffer structure but don't store the root
        flatbuffers::root_with_opts::<GraphBlob>(&verifier_opts, graph_buffer)
            .with_context(|| "Failed to parse/verify graph data from buffer")?;

        info!("Graph data loaded and verified successfully.");
        Ok(s)
    }

    // Pass GraphBlob as argument
    fn calculate_edge_cost(&self, graph_blob: &tobmapgraph::GraphBlob, edge_id: u32) -> u32 {
        if let Some(edges) = graph_blob.edges() {
            if (edge_id as usize) < edges.len() {
                let edge = edges.get(edge_id as usize);
                return (edge.costs_and_flags() >> 3).into();
            }
        }
        u32::MAX
    }

    // Pass GraphBlob as argument
    fn calculate_interaction_cost(&self, graph_blob: &tobmapgraph::GraphBlob, node_idx: u32, incoming_edge: u32, outgoing_edge: u32) -> u32 {
        if let Some(nodes) = graph_blob.nodes() {
            if (node_idx as usize) < nodes.len() {
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
                                let interaction_blob = interactions.get(in_pos);
                                let iii = interaction_blob.outgoing();
                                        match iii {
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
        2
    }

    // Pass GraphBlob as argument
    fn get_adjacent_edges(&self, graph_blob: &tobmapgraph::GraphBlob, edge_id: u32, node_idx: u32) -> Vec<u32> {
        let mut adjacent = Vec::new();

        if let Some(nodes) = graph_blob.nodes() {
            if (node_idx as usize) < nodes.len() {
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

        adjacent
    }

    fn find_paths(&self, start_edge_id: u32, end_edge_id: u32, max_paths: usize) -> Result<Vec<(Vec<u32>, Vec<u32>)>, Error> {
        let mut result_paths = Vec::new();
        let mut used_edges = HashSet::new();

        match self.find_shortest_path(start_edge_id, end_edge_id, &used_edges) {
            Ok(shortest_path_info) => {
                for &edge in &shortest_path_info.0 {
                    used_edges.insert(edge);
                }
                result_paths.push(shortest_path_info);
            }
            Err(e) => {
                // If the first path fails, return the error
                return Err(e);
            }
        }


        for _ in 1..max_paths {
            match self.find_shortest_path(start_edge_id, end_edge_id, &used_edges) {
                 Ok(path_info) => {
                    if path_info.0.is_empty() {
                        break; // No more paths found
                    }
                    for &edge in &path_info.0 {
                        used_edges.insert(edge);
                    }
                    result_paths.push(path_info);
                }
                Err(_) => {
                    // If subsequent path finding fails, we just stop finding more paths
                    // but still return the paths found so far.
                    break;
                }
            }
        }

        Ok(result_paths)
    }

    // Returns Result<(edge_path, connecting_node_path), Error>
    fn find_shortest_path(&self, start_edge_id: u32, end_edge_id: u32, avoid_edges: &HashSet<u32>) -> Result<(Vec<u32>, Vec<u32>), Error> {
        info!("Finding shortest path from {} to {}", start_edge_id, end_edge_id);
        let graph_data = self.graph_data.as_ref().context("Graph data not loaded")?;

        let verifier_opts = flatbuffers::VerifierOptions {
            max_tables: 3_000_000_000, // 3 billion tables
            ..Default::default()
        };

        // Verify the buffer structure but don't store the root
        let graph_blob = flatbuffers::root_with_opts::<GraphBlob>(&verifier_opts, graph_data)
            .with_context(|| "Failed to parse/verify graph data from buffer")?;

        // let graph_blob = flatbuffers::root::<GraphBlob>(graph_data).context("Failed to parse graph data")?;

        let edges = graph_blob.edges().context("Edges data missing in graph")?;

        let mut distances: HashMap<u32, u32> = HashMap::new();
        let mut prev_info: HashMap<u32, (u32, u32)> = HashMap::new();
        let mut pq = BinaryHeap::new();

        distances.insert(start_edge_id, 0);
        pq.push((Reverse(0), start_edge_id));

        info!("Starting Dijkstra's algorithm");

        while let Some((Reverse(cost), current_edge)) = pq.pop() {
            // info!("Visiting edge {} with cost {}", current_edge, cost);
            if current_edge == end_edge_id {
                return Ok(self.reconstruct_path(start_edge_id, end_edge_id, &prev_info));
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
                let adjacent_edges = self.get_adjacent_edges(&graph_blob, current_edge, node_idx);

                for &next_edge in &adjacent_edges {
                    if avoid_edges.contains(&next_edge) && next_edge != end_edge_id {
                        continue;
                    }

                    let edge_cost = self.calculate_edge_cost(&graph_blob, next_edge);
                    let interaction_cost = self.calculate_interaction_cost(&graph_blob, node_idx, current_edge, next_edge);

                    let cost_sum = edge_cost.saturating_add(interaction_cost);
                    let next_cost = cost.saturating_add(cost_sum);

                    let is_better_path = match distances.get(&next_edge) {
                        Some(&existing_cost) => next_cost < existing_cost,
                        None => true,
                    };

                    if is_better_path {
                        distances.insert(next_edge, next_cost);
                        prev_info.insert(next_edge, (current_edge, node_idx));
                        pq.push((Reverse(next_cost), next_edge));
                    }
                }
            }
        }

        info!("No path found from {} to {}", start_edge_id, end_edge_id);

        Err(anyhow::anyhow!("No path found from {} to {}", start_edge_id, end_edge_id))
    }

    fn reconstruct_path(&self, start_edge_id: u32, end_edge_id: u32, prev_info: &HashMap<u32, (u32, u32)>) -> (Vec<u32>, Vec<u32>) {
        let mut path_edges = Vec::new();
        let mut path_nodes = Vec::new();
        let mut current = end_edge_id;

        while current != start_edge_id {
            path_edges.push(current);
            match prev_info.get(&current) {
                Some(&(prev_edge, connecting_node)) => {
                    path_nodes.push(connecting_node);
                    current = prev_edge;
                },
                None => return (Vec::new(), Vec::new()),
            }
        }

        path_edges.push(start_edge_id);
        path_edges.reverse();
        path_nodes.reverse();

        (path_edges, path_nodes)
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

        if self.graph_data.is_none() {
            return Err(Status::unavailable("Graph data not loaded"));
        }

        let start_edge_id = req.start_edge_idx;
        let end_edge_id = req.end_edge_idx;

        let num_paths = 3;
        let paths_info = self.find_paths(start_edge_id, end_edge_id, num_paths)
            .map_err(|e| Status::internal(format!("Failed to find paths: {}", e)))?;

        let result_paths = paths_info.into_iter()
            .map(|(edge_path, node_path)| RoutePath { edges: edge_path, nodes: node_path })
            .collect();

        let reply = RouteResponse {
            paths: result_paths,
        };

        Ok(Response::new(reply))
    }
}