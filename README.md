# tobmap
tobmap

![](png.png)

## Time Spent So far

30 hours? 25? Underestimating how much toil time on one or two issues, but over estimating cuz I don't think I've spent 1 hour 30 times.. unsure.

### Graphbuild

```
cargo run --release --bin graphbuild -- ~/Downloads/washington-latest.osm.pbf outputs/walatest_graph.fb outputs/walatest_location.fb outputs/walatest_description.fb
```

### Snap Build

```
cargo run --release --bin snapbuild -- \-g outputs/walatest_graph.fb -l outputs/walatest_location.fb
```

### Graphviz

```
cargo run --release --bin graphviz -- --graph outputs/walatest.graph.pb --location outputs/walatest.graph.location.fb --output png.png
```

```
 cargo run --release --bin graphviz -- --graph outputs/walatest.graph.pb --location outputs/walatest.graph.location.fb --output png.png --center-lat=47.814204 --center-lng=-119.045459 --zoom-meters=30000 --edge-width=5 --node-size=2
```


### Server

```
cargo run --release --bin server -- -s outputs/snapbuckets -g outputs/walatest_graph.fb
```

Crazy!

- I think we need a .ai_history file which says which lines of code were from AI, like git blame but points to what the prompt was and what the model was, among other things
- Bit shifting was weird, I wanted right most and it did MSB, idk maybe im the weird one tho
- I said to make a blob where we have more info about the edges and their points, but it still only grabbed start and end even though we had all of the intermediate point data available (curved roads are just sequences of points)
- For snapping, 2 levels, outer is for a file, inner is within a file so you can quickly get to a L8 cell, then within that L8 cell we have the cell ids and their locations - it used the L8 Cell id for all edges within that cell for some reason!
- could NOT figure out tiling haha, either stretched or centered w/ tons of whitespace


## Requests

Snap: my house, result edge: `640909`

```
{
  "lat": 47.66402050260777,
  "lng": -122.33892695814653
}
```

Snap: work, result edge: `686615`

```
{
  "lat": 47.64900906111412,
  "lng":  -122.35073491444791
}
```

Route Request

```
{
  "startEdgeIdx": 640909,
  "endEdgeIdx": 686615
}
```