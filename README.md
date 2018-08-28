# gfx-hal tutorials

[![Build Status](https://travis-ci.org/Mistodon/gfx-hal-tutorials.svg?branch=master)](https://travis-ci.org/Mistodon/gfx-hal-tutorials)

This repo will contain a series of self-contained gfx-hal tutorials, hopefully to make it easier to grok how each feature of the API works individually. I'm still learning myself, so they won't be perfect, but hopefully they'll be useful to somebody.

## Running tutorials

The following parts are finished:

- `cargo run --bin part00-triangle`
- `cargo run --bin part01-resizing`

The following parts should be considered WIP and are likely to change:

- `cargo run --bin part02-vertex-buffer`
- `cargo run --bin part03-uniforms`
- `cargo run --bin part04-push-constants`
- `cargo run --bin part05-depth`
    - `cargo run --bin part05-no-depth`
- `cargo run --bin part06-textures`

## Shaders

Shaders are written in GLSL and can be found under `source_assets/shaders`. The `build.rs` file in the root compiles them to SPIR-V at build time. (See [this post](https://falseidolfactory.com/2018/06/23/compiling-glsl-to-spirv-at-build-time.html) for details.)
