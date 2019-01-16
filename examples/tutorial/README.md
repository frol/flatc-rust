# FlatBuffers Tutorial using flatc-rust crate

This is a clone of
[the official FlatBuffers Tutorial](https://google.github.io/flatbuffers/flatbuffers_guide_tutorial.html)
with only one change, that is you don't need to use `flatc` utility yourself as `build.rs` manages
it for you.

## How to run?

In the end, once you modify the `flatbuffers/monster.fbs`, you just follow the regular Rust project
workflow (that is, you don't need to invoke `flatc` command manually in contrast to the official
FlatBuffers tutorial):

```
$ cargo run
```
