// Import libraries
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use image::{RgbImage, ImageFormat};
use rayon::prelude::*;
use schema::tobmapgraph::{GraphBlob, LocationBlob, DescriptionBlob};
use graphviz::{self, VizConfig, TileConfig, process_world_data, render_tile, GraphVizError, WorldData};

/// Configuration for tile generation
#[derive(Debug, Clone)]
pub struct TileBuildConfig {
    // Output directory for tiles
    pub output_dir: PathBuf,
    
    // Maximum zoom level (0-based)
    pub max_zoom_level: u32,
    
    // Tile size in pixels (longest edge)
    pub tile_size: u32,
    
    // Overlap between tiles in pixels
    pub tile_overlap: u32,
    
    // Show vertices for each zoom level
    pub show_vertices: Vec<bool>,
    
    // Minimum priority to render for each zoom level
    pub min_priority: Vec<usize>,
    
    // Base visualization configuration
    pub viz_config: VizConfig,
}

/// Tile builder
pub struct TileBuilder {
    config: TileBuildConfig,
}

impl TileBuilder {
    /// Create a new tile builder with the given configuration
    pub fn new(config: TileBuildConfig) -> Self {
        Self { config }
    }
    
    /// Build all tiles for all zoom levels
    pub fn build_all_tiles(&self, graph: &GraphBlob, location: &LocationBlob, description: &DescriptionBlob) -> Result<()> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(&self.config.output_dir).context("Failed to create output directory")?;
        
        // Process the world data once (heavy operation)
        let world_data = Arc::new(process_world_data(graph, location, description, self.config.tile_size)
            .context("Failed to process world data")?);
            
        println!("Processed world data with {} nodes and {} edges", 
            world_data.nodes_count, world_data.edges_count);
        
        // For each zoom level...
        for zoom_level in 0..=self.config.max_zoom_level {
            self.build_zoom_level(zoom_level, graph, location, description, Arc::clone(&world_data))
                .with_context(|| format!("Failed to build zoom level {}", zoom_level))?;
        }
        
        Ok(())
    }
    
    /// Build all tiles for a specific zoom level
    fn build_zoom_level(&self, zoom_level: u32, graph: &GraphBlob, location: &LocationBlob, 
        description: &DescriptionBlob, world_data: Arc<WorldData>) -> Result<()> {
        println!("Building zoom level {}...", zoom_level);
        
        // Create directory for this zoom level
        let zoom_dir = self.config.output_dir.join(format!("{}", zoom_level));
        fs::create_dir_all(&zoom_dir).context("Failed to create zoom level directory")?;
        
        // Calculate number of tiles in each direction
        // Double the number of tiles in each direction for each zoom level
        let num_tiles = 2u32.pow(zoom_level);
        
        // Get settings for this zoom level
        let show_vertices = if zoom_level < self.config.show_vertices.len() as u32 {
            self.config.show_vertices[zoom_level as usize]
        } else {
            true // Default to showing vertices if not specified
        };
        
        let min_priority = if zoom_level < self.config.min_priority.len() as u32 {
            self.config.min_priority[zoom_level as usize]
        } else {
            0 // Default to showing all priorities if not specified
        };
        
        // Generate all tiles in parallel
        (0..num_tiles * num_tiles).into_par_iter().try_for_each(|idx| {
            let row = idx / num_tiles;
            let col = idx % num_tiles;
            
            self.build_tile(zoom_level, row, col, num_tiles, graph, location, 
                            description, Arc::clone(&world_data), show_vertices, min_priority)
                .with_context(|| format!("Failed to build tile {}/{} at zoom level {}", row, col, zoom_level))
        })?;
        
        Ok(())
    }
    
    /// Build a single tile
    fn build_tile(&self, zoom_level: u32, row: u32, col: u32, num_tiles: u32,
        graph: &GraphBlob, location: &LocationBlob, description: &DescriptionBlob, world_data: Arc<WorldData>,
        show_vertices: bool, min_priority: usize) -> Result<()> {
        
        // Configure tile for rendering
        let tile_config = TileConfig {
            rows: num_tiles,
            columns: num_tiles,
            row_index: row,
            column_index: col,
            overlap_pixels: self.config.tile_overlap,
            zoom_level,
        };
        
        // Create a visualization config specific to this tile
        let mut viz_config = self.config.viz_config.clone();
        viz_config.tile = Some(tile_config);
        viz_config.node_size = if show_vertices { Some(0) } else { None }; // Only draw nodes if enabled
        viz_config.edge_width = 1.0; // Standard edge width
        
        // Create WorldData for this zoom level with priority filtering
        // The filtering happens in the render_tile function
        
        // Render the tile
        let image = render_tile(&world_data, &viz_config, min_priority)
            .context("Failed to render tile")?;
        
        // Save the image
        let output_path = self.config.output_dir
            .join(format!("{}", zoom_level))
            .join(format!("{}_{}.png", row, col));
            
        image.save_with_format(&output_path, ImageFormat::Png)
            .with_context(|| format!("Failed to save tile image to {:?}", output_path))?;
        
        Ok(())
    }
}
