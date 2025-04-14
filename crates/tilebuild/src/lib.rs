use anyhow::{Result, Context};
use schema::tobmapgraph::{GraphBlob, LocationBlob};
use graphviz::{visualize_graph, VizConfig, TileConfig};
use std::path::{Path, PathBuf};
use std::fs;

/// Configuration for tile building process
#[derive(Clone, Debug)]
pub struct TileBuildConfig {
    pub output_dir: PathBuf,        // Directory where tiles will be saved
    pub max_zoom_level: u32,        // Maximum zoom level (0-based)
    pub tile_size: u32,             // Target size for tiles (longest edge)
    pub tile_overlap: u32,          // Overlap between tiles in pixels
    pub viz_config: VizConfig,      // Base visualization config
}

impl Default for TileBuildConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("tiles"),
            max_zoom_level: 3,     // 4 levels (0-3)
            tile_size: 2048,       // 2048 pixels on longest edge
            tile_overlap: 16,      // 16 pixel overlap between tiles
            viz_config: VizConfig {
                max_size: 2048,
                node_size: 0,
                edge_width: 0.0,
                show_labels: false,
                center_lat: None,
                center_lng: None,
                zoom_meters: None,
                highlight_edge_index: None,
                highlight_edge_width: None,
                tile: None,
            },
        }
    }
}

pub struct TileBuilder {
    config: TileBuildConfig,
}

impl TileBuilder {
    pub fn new(config: TileBuildConfig) -> Self {
        Self { config }
    }
    
    /// Generate tile filename for a specific level, row, column
    fn get_tile_filename(&self, level: u32, row: u32, col: u32) -> PathBuf {
        let level_dir = format!("level_{}", level);
        let filename = format!("tile_{}_{}_{}.jpg", level, row, col);
        self.config.output_dir.join(level_dir).join(filename)
    }
    
    /// Create directory structure for tiles
    fn create_directories(&self) -> Result<()> {
        for level in 0..=self.config.max_zoom_level {
            let level_dir = self.config.output_dir.join(format!("level_{}", level));
            fs::create_dir_all(&level_dir)
                .with_context(|| format!("Failed to create directory: {:?}", level_dir))?;
        }
        Ok(())
    }
    
    /// Build all tiles for all zoom levels
    pub fn build_all_tiles(&self, graph: &GraphBlob, location: &LocationBlob) -> Result<()> {
        self.create_directories()?;
        
        // For each zoom level
        for level in 0..=self.config.max_zoom_level {
            self.build_level_tiles(level, graph, location)?;
        }
        
        Ok(())
    }
    
    /// Build all tiles for a specific zoom level
    fn build_level_tiles(&self, level: u32, graph: &GraphBlob, location: &LocationBlob) -> Result<()> {
        // For each level, we have 3^level tiles per side
        let tiles_per_side = 3u32.pow(level);
        println!("Building level {} with {}x{} tiles...", level, tiles_per_side, tiles_per_side);
        
        for row in 0..tiles_per_side {
            for col in 0..tiles_per_side {
                self.build_single_tile(level, row, col, tiles_per_side, graph, location)?;
            }
        }
        
        Ok(())
    }
    
    /// Build a single tile
    fn build_single_tile(
        &self, 
        level: u32,
        row: u32, 
        col: u32, 
        tiles_per_side: u32,
        graph: &GraphBlob, 
        location: &LocationBlob
    ) -> Result<()> {
        println!("  Generating tile {}/{}: level={}, row={}, col={}...", 
            row * tiles_per_side + col + 1, 
            tiles_per_side * tiles_per_side, 
            level, row, col);
        
        // Configure tile settings
        let mut viz_config = self.config.viz_config.clone();
        viz_config.tile = Some(TileConfig {
            rows: tiles_per_side,
            columns: tiles_per_side,
            row_index: row,
            column_index: col,
            overlap_pixels: self.config.tile_overlap,
        });
        
        // Render the tile
        let image = visualize_graph(graph, location, &viz_config)
            .with_context(|| format!("Failed to render tile: level={}, row={}, col={}", level, row, col))?;
        
        // Save the image
        let output_path = self.get_tile_filename(level, row, col);
        image.save(&output_path)
            .with_context(|| format!("Failed to save tile to {:?}", output_path))?;
        
        Ok(())
    }
}
