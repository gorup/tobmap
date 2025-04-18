# PowerShell script to generate Rust code from the FlatBuffer schema

# Check if flatc is in PATH
if (-not (Get-Command flatc -ErrorAction SilentlyContinue)) {
    Write-Error "Error: flatc (FlatBuffers compiler) is not installed or not in PATH"
    Write-Error "Please install it from https://google.github.io/flatbuffers/"
    exit 1
}

# Create the generated directory if it doesn't exist
if (-not (Test-Path -Path "src\generated")) {
    New-Item -Path "src\generated" -ItemType Directory -Force
}

# Generate Rust code from the schema
flatc --rust -o src/generated src/schema/graph.fbs

# Create a mod.rs file to re-export the generated code
@"
// Generated code from FlatBuffers schema
pub mod graph_generated;
pub use graph_generated::tobmap;
"@ | Out-File -FilePath "src\generated\mod.rs" -Encoding utf8

Write-Host "FlatBuffers code generation complete" 