use anyhow::{Context, Result};
use reqwest::blocking::Client;
use log::info;

use crate::cache::Cache;

/// Sources for OpenStreetMap data
pub enum OsmSource {
    /// Full planet dataset from OpenStreetMap
    Planet,
    /// Country-level extract
    Country(String),
    /// Region or state extract
    Region(String, String), // Country, Region
    /// A US State
    State(String),
    /// Custom URL to an OSM PBF file
    CustomUrl(String),
    /// Local file path
    LocalFile(String),
}

/// Downloader for OpenStreetMap data
pub struct Downloader {
    cache: Cache,
    client: Client,
}

impl Downloader {
    /// Create a new downloader with the given cache
    pub fn new(cache: Cache) -> Self {
        Self {
            cache,
            client: Client::new(),
        }
    }
    
    /// Download OSM data from the specified source
    /// Returns the path to the downloaded or cached file
    pub fn download(&self, source: OsmSource) -> Result<String> {
        match source {
            OsmSource::Planet => self.download_from_url(
                "https://planet.openstreetmap.org/pbf/planet-latest.osm.pbf",
            ),
            OsmSource::Country(country) => {
                let url = format!(
                    "https://download.geofabrik.de/{}.osm.pbf", 
                    country.to_lowercase()
                );
                self.download_from_url(&url)
            },
            OsmSource::Region(country, region) => {
                let url = format!(
                    "https://download.geofabrik.de/{}/{}.osm.pbf", 
                    country.to_lowercase(), 
                    region.to_lowercase()
                );
                self.download_from_url(&url)
            },
            OsmSource::State(state) => {
                let url = format!(
                    "https://download.geofabrik.de/north-america/us/{}-latest.osm.pbf", 
                    state.to_lowercase()
                );
                self.download_from_url(&url)
            },
            OsmSource::CustomUrl(url) => self.download_from_url(&url),
            OsmSource::LocalFile(path) => Ok(path),
        }
    }
    
    /// Download OSM data from a URL
    fn download_from_url(&self, url: &str) -> Result<String> {
        info!("Downloading OSM data from {}", url);
        
        // Check if the file is already in the cache
        if let Some(cached_path) = self.cache.get_cached_file(url) {
            info!("Using cached OSM data at {}", cached_path.display());
            return Ok(cached_path.to_string_lossy().into_owned());
        }
        // Download the file with a 10 minute timeout
        info!("Downloading from {}", url);
        let response = self.client.get(url)
            .timeout(std::time::Duration::from_secs(600))
            .send()
            .context("Failed to send request")?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to download: HTTP {}", response.status());
        }
        let data = response.bytes()
            .context("Failed to read response bytes")?;
        
        // Save to the cache
        let cache_path = self.cache.save_to_cache(url, &data)
            .context("Failed to save to cache")?;
        
        info!("Downloaded OSM data to {}", cache_path.display());
        
        Ok(cache_path.to_string_lossy().into_owned())
    }
} 