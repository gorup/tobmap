# tobmap
tobmap

![](png.png)

# As of this commit, I spent 90 mins initially on the schema just thinking, but all of the other code has taken I'd say 3-4 hours. Visualization, graph generation, snap API skeleton. What's left is snap graph creation, snap search API impl, then graph search API impl, then maybe a UI but for now probably just render lines for a google maps URL or something.

- Now another 2-3hrs

```
cargo run --release --bin graphbuild ../Downloads/washington-latest.osm.pbf outputs/walatest.graph.pb
```

```
cargo run --release --bin graphviz -- --graph outputs/walatest.graph.pb --location outputs/walatest.graph.location.fb --output png.png
```

Crazy!