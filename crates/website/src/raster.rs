use actix_files as fs;
use actix_web::{web, App, HttpServer, Responder, Result, HttpResponse};
use std::path::Path;
use std::fs::File;
use std::io::Read;

async fn index() -> Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("crates/website/raster/index.html")?)
}

async fn get_tile(path: web::Path<(u32, u32, u32)>) -> impl Responder {
    let (level, x, y) = path.into_inner();
    
    // Check if the requested level is within our supported range (1-10)
    if level < 1 || level > 10 {
        return HttpResponse::NotFound().body("Zoom level out of range");
    }
    
    let tile_path = format!("outputs/tilesrastergraph/{}/{}_{}.png", level, x, y);

    // Check if file exists
    if Path::new(&tile_path).exists() {
        // Read file contents
        match File::open(&tile_path) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                if file.read_to_end(&mut contents).is_ok() {
                    return HttpResponse::Ok()
                        .content_type("image/png")
                        .body(contents);
                }
            }
            Err(_) => {}
        }
    }

    HttpResponse::NotFound().body("Tile not found")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting raster tile server at http://127.0.0.1:8080");
    
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/tile/{level}/{x}/{y}", web::get().to(get_tile))
            .service(fs::Files::new("/static", "crates/website/raster")
                .show_files_listing()
                .use_last_modified(true))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}