fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/snap.proto")?;
    tonic_build::compile_protos("proto/route.proto")?;
    Ok(())
}