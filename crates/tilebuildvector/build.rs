use std::env;
use std::path::PathBuf;

fn main() {
    let proto_file = "proto/tile.proto";
    
    // Tell cargo to re-run this build script if the proto file changes
    println!("cargo:rerun-if-changed={}", proto_file);
    
    // Compile protobuf files
    prost_build::compile_protos(&[proto_file], &["proto"]).unwrap();
}