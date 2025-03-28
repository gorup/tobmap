# TobMap

TobMap is a tool for downloading OpenStreetMap data and converting it into a graph-based representation using FlatBuffers.

## Features

- Download OpenStreetMap data from various sources (planet, country/region extracts, custom URLs)
- Cache downloaded data to avoid multiple copies
- Process OSM data into a graph representation where:
  - Nodes represent real-world intersections
  - Edges represent the ways (streets, paths) between intersections
- Store data efficiently using FlatBuffers
- Organize data by S2 cells for geospatial queries
- Support for multiple travel modes (car, bike, walk, transit) with respective costs

## Installation

### Prerequisites

- Rust (1.65+)
- FlatBuffers compiler (flatc) - install from https://google.github.io/flatbuffers/

### Building

**IMPORTANT**: This project requires the FlatBuffers compiler to generate code from the schema.

#### Step 1: Install the FlatBuffers Compiler

- **Windows**: 
  - Download the latest release from [GitHub](https://github.com/google/flatbuffers/releases)
  - Extract and add the folder containing `flatc.exe` to your PATH

- **macOS**:
  ```sh
  brew install flatbuffers
  ```

- **Linux**:
  ```sh
  # Ubuntu/Debian
  sudo apt-get install flatbuffers-compiler
  
  # Or build from source
  git clone https://github.com/google/flatbuffers.git
  cd flatbuffers
  cmake -G "Unix Makefiles" -DCMAKE_BUILD_TYPE=Release
  make
  sudo make install
  ```

#### Step 2: Generate the FlatBuffers Code

After installing flatc and cloning the repository:

```sh
# On Linux/macOS
./generate_flatbuffers.sh

# On Windows
.\generate_flatbuffers.ps1
```

This step is mandatory - the project won't compile without generating the FlatBuffers code!

#### Step 3: Build the Project

```sh
cargo build --release
```

## Usage

```sh
# Download and process a country extract
cargo run -- -c .cache -o output download -s country -c germany -o germany.fb

# Process an existing OSM PBF file
cargo run -- -c .cache -o output process -i path/to/map.osm.pbf -o mymap.fb

# Clear the cache
cargo run -- -c .cache -o output clear-cache
```

## Command Line Options

- `-c, --cache-dir <CACHE_DIR>`: Path to the cache directory (default: ".cache")
- `-o, --output-dir <OUTPUT_DIR>`: Path to the output directory (default: "output")

### Download Subcommand

- `-s, --source <SOURCE>`: Source of the OpenStreetMap data ("planet", "country", "region", "url", "file")
- `-c, --country <COUNTRY>`: Country for country-level extracts
- `-r, --region <REGION>`: Region for region-level extracts
- `-u, --url <URL>`: URL for custom data source
- `-f, --file <FILE>`: Path to local OSM PBF file
- `-o, --output <OUTPUT>`: Output filename for the processed data (default: "map.fb")

### Process Subcommand

- `-i, --input <INPUT>`: Path to the input OSM PBF file
- `-o, --output <OUTPUT>`: Output filename for the processed data (default: "map.fb")

## Data Format

The resulting FlatBuffer contains:

- Nodes with unique IDs and S2 cell IDs
- Edges with unique IDs and travel costs for different modes
- Data organized by S2 cells for efficient spatial queries

## FlatBuffer Schema

The schema for the FlatBuffer data is located in `src/schema/graph.fbs`. The schema defines:

- MapData: The root object containing all cells
- Cell: Contains nodes and edges within a specific S2 cell
- Node: Represents intersections with ID and position
- Edge: Represents ways between nodes with travel costs for different modes

## License

This project is licensed under the MIT License - see the LICENSE file for details.
