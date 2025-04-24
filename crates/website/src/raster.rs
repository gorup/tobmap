use actix_files as fs;
use actix_web::{web, App, HttpServer, Responder, Result, HttpResponse};
use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::time::SystemTime;
use actix_web::http::header;

async fn index() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("crates/website/raster/index.html")?)
}

async fn get_tile_with_cache(
    path: web::Path<(u32, u32, u32)>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let (level, x, y) = path.into_inner();
    
    // Check if the requested level is within our supported range (1-10)
    if level < 1 || level > 10 {
        return HttpResponse::NotFound().body("Zoom level out of range");
    }
    
    let tile_path = format!("outputs/tilesrastergraph/{}/{}_{}.png", level, x, y);
    
    // Check if file exists
    if !Path::new(&tile_path).exists() {
        return HttpResponse::NotFound().body("Tile not found");
    }
    
    // Get file metadata for caching
    match std::fs::metadata(&tile_path) {
        Ok(metadata) => {
            let last_modified = metadata.modified().unwrap_or(SystemTime::now());
            let last_modified_secs = last_modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            // Create a simple ETag based on last modified time and file size
            let file_size = metadata.len();
            let etag = format!("\"{:x}-{:x}\"", last_modified_secs, file_size);
            
            // Check if the client has a valid cached version
            if let Some(if_none_match) = req.headers().get(header::IF_NONE_MATCH) {
                if let Ok(if_none_match_str) = if_none_match.to_str() {
                    if if_none_match_str == etag {
                        // Client has a valid cached version
                        return HttpResponse::NotModified()
                            .insert_header((header::CACHE_CONTROL, "public, max-age=86400"))
                            .insert_header((header::ETAG, etag))
                            .finish();
                    }
                }
            }
            
            // Read file contents
            match File::open(&tile_path) {
                Ok(mut file) => {
                    let mut contents = Vec::new();
                    if file.read_to_end(&mut contents).is_ok() {
                        return HttpResponse::Ok()
                            .content_type("image/png")
                            .insert_header((header::CACHE_CONTROL, "public, max-age=86400"))
                            .insert_header((header::ETAG, etag))
                            .body(contents);
                    }
                }
                Err(_) => {}
            }
        }
        Err(_) => {}
    }
    
    HttpResponse::InternalServerError().body("Failed to process tile")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting raster tile server at http://127.0.0.1:8080");
    
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/tile/{level}/{x}/{y}", web::get().to(get_tile_with_cache))
            .service(fs::Files::new("/static", "crates/website/raster")
                .show_files_listing()
                .use_last_modified(true))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}