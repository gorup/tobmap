#!/bin/bash

# This script generates Rust code from the FlatBuffer schema

# Check if flatc is in PATH
if ! command -v flatc &> /dev/null; then
    echo "Error: flatc (FlatBuffers compiler) is not installed or not in PATH"
    echo "Please install it from https://google.github.io/flatbuffers/"
    exit 1
fi

# Create the generated directory if it doesn't exist
mkdir -p src/generated

# Generate Rust code from the schema
flatc --rust -o src/generated src/schema/graph.fbs

# Create a mod.rs file to re-export the generated code
cat > src/generated/mod.rs << EOF
// Generated code from FlatBuffers schema
pub mod graph_generated;
pub use graph_generated::tobmap;
EOF

echo "FlatBuffers code generation complete" 