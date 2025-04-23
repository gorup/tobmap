use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_files as fs;
use std::path::Path;
use std::fs::File;
use std::io::Read;

#[get("/api/tiles/{level}/{s2cell}.pb")]
async fn serve_tile(path: web::Path<(u8, String)>) -> impl Responder {
    let (level, s2cell) = path.into_inner();
    
    if level > 10 {
        return HttpResponse::BadRequest().body("Invalid level. Must be between 1-10");
    }

    let tile_path = format!("outputs/tilesvector/level_{}/tile_{}.pb", level, s2cell);

    // Check if file exists
    if Path::new(&tile_path).exists() {
        // Read file contents
        match File::open(&tile_path) {
            Ok(mut file) => {
                let mut contents = Vec::new();
                if file.read_to_end(&mut contents).is_ok() {
                    return HttpResponse::Ok()
                        .content_type("application/protobuf")
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
    println!("Starting server at http://127.0.0.1:8080");
    
    HttpServer::new(|| {
        App::new()
            .service(serve_tile)
            // Serve static files from the static directory
            .service(fs::Files::new("/", "static").index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}