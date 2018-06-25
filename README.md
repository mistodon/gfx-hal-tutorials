# gfx-hal demos

This repo will contain a series of self-contained gfx-hal examples, hopefully to make it easier to grok how each feature of the API works individually. I'm still learning myself, so they won't be perfect, but hopefully they'll be useful to somebody.

## Running examples

`cargo run --bin part00-triangle`

## Shaders

Shaders are written in GLSL and can be found under `source_assets/shaders`. The `build.rs` file in the root compiles them to SPIR-V at build time. (See [this post](https://falseidolfactory.com/2018/06/23/compiling-glsl-to-spirv-at-build-time.html) for details.)
