# tobmap
tobmap

![](png.png)

# As of this commit, I spent 90 mins initially on the schema just thinking, but all of the other code has taken I'd say 3-4 hours. Visualization, graph generation, snap API skeleton. What's left is snap graph creation, snap search API impl, then graph search API impl, then maybe a UI but for now probably just render lines for a google maps URL or something.

- Now another 2-3hrs

```
cargo run --release --bin graphbuild -- ~/Downloads/washington-latest.osm.pbf outputs/walatest_graph.fb outputs/walatest_location.fb outputs/walatest_description.fb
```

```
cargo run --release --bin graphviz -- --graph outputs/walatest.graph.pb --location outputs/walatest.graph.location.fb --output png.png
```

```
 cargo run --release --bin graphviz -- --graph outputs/walatest.graph.pb --location outputs/walatest.graph.location.fb --output png.png --center-lat=47.814204 --center-lng=-119.045459 --zoom-meters=30000 --edge-width=5 --node-size=2
```

```
cargo run --release --bin server -- -s outputs/snapbuckets
```

```
{
  "lat": 47.7283315,
  "lng": -119.2441248
}
```

Crazy!

- I think we need a .ai_history file which says which lines of code were from AI, like git blame but points to what the prompt was and what the model was, among other things
- Bit shifting was weird, I wanted right most and it did MSB, idk maybe im the weird one tho
- I said to make a blob where we have more info about the edges and their points, but it still only grabbed start and end even though we had all of the intermediate point data available (curved roads are just sequences of points)
- For snapping, 2 levels, outer is for a file, inner is within a file so you can quickly get to a L8 cell, then within that L8 cell we have the cell ids and their locations - it used the L8 Cell id for all edges within that cell for some reason!
- could NOT figure out tiling haha, either stretched or centered w/ tons of whitespace
