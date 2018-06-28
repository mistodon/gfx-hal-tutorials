# gfx-hal tutorials

This repo will contain a series of self-contained gfx-hal tutorials, hopefully to make it easier to grok how each feature of the API works individually. I'm still learning myself, so they won't be perfect, but hopefully they'll be useful to somebody.

These tutorials all currently use the Metal backend, and so will only run on macOS devices. However, you should be able to quite easily swap out the backend on other platforms. In future, I'll make some modifications to allow them to be cross-platform.

## Running tutorials

- `cargo run --bin part00-triangle`
- `cargo run --bin part01-resizing`

## Shaders

Shaders are written in GLSL and can be found under `source_assets/shaders`. The `build.rs` file in the root compiles them to SPIR-V at build time. (See [this post](https://falseidolfactory.com/2018/06/23/compiling-glsl-to-spirv-at-build-time.html) for details.)
