# gfx-hal tutorials

[![Build Status](https://travis-ci.org/Mistodon/gfx-hal-tutorials.svg?branch=master)](https://travis-ci.org/Mistodon/gfx-hal-tutorials)

This repo will contain a series of self-contained gfx-hal tutorials, hopefully to make it easier to grok how each feature of the API works individually. I'm still learning myself, so they won't be perfect, but hopefully they'll be useful to somebody.

Each tutorial is covered by a blog post:

- [Part 0: Drawing a triangle](https://falseidolfactory.com/2018/08/16/gfx-hal-part-0-drawing-a-triangle.html)
- [Part 1: Resizing windows](https://falseidolfactory.com/2018/08/23/gfx-hal-part-1-resizing-windows.html)
- [Part 2: Vertex buffers](https://falseidolfactory.com/2018/10/09/gfx-hal-part-2-vertex-buffers.html)

## Running tutorials

The following parts are finished:

- `cargo run --bin part00-triangle`
- `cargo run --bin part01-resizing`
- `cargo run --bin part02-vertex-buffer [teapot]`

The following parts should be considered WIP and are likely to change:

- `cargo run --bin part03-uniforms`
- `cargo run --bin part04-push-constants`
- `cargo run --bin part05-depth`
    - `cargo run --bin part05-no-depth`
- `cargo run --bin part06-textures`

## Shaders

Shaders are written in GLSL and can be found under `source_assets/shaders`. The `build.rs` file in the root compiles them to SPIR-V at build time. (See [this post](https://falseidolfactory.com/2018/06/23/compiling-glsl-to-spirv-at-build-time.html) for details.)

## Contributing

All contributions are welcome. If your change is applicable to multiple non-WIP parts (see above), please make it in all of them. If it's a very significant change, it's probably best to open an issue first so we can discuss it. Other than that, feel free to open a PR.

Thanks to:

- icefoxen
- human9

for their contributions!
