# gfx-hal tutorials

[![Build Status](https://travis-ci.org/mistodon/gfx-hal-tutorials.svg?branch=master)](https://travis-ci.org/mistodon/gfx-hal-tutorials)

This is a series of tutorials on the gfx-hal graphics API for Rust. You can see the write-ups for them here: https://falseidolfactory.com/projects/learning-gfx-hal/

The tutorials are broken into sequential parts, so Part 2 is built on top of Part 1. This means if you want to follow along with a part, you can use the previous part as a base.

If you just want to see the examples in action, you can simply clone the repo and run the binary for a part:

```bash
$ cargo run --bin part-1-triangle
$ cargo run --bin part-2-push-constants
```

## License

The _code_ for these tutorials (e.g. everything under the `src/` directory) is under the [CC0](https://creativecommons.org/share-your-work/public-domain/cc0/) waiver. It's in the public domain, as much as it can be. Do what you like with it!

The _text_ for the tutorials (e.g. everything under the `doc/` directory) is under the [Creative Commons Attribution 4.0](https://creativecommons.org/licenses/by/4.0/) license. Free to use and modify as long as attribution is given.

## Contributing

All contributions are welcome! If it's a very significant change, it's probably best to open an issue first so we can discuss it. Other than that, feel free to open a PR.

Thanks to:

- icefoxen
- human9

for their contributions!

## The old version...

These tutorials are currently undergoing a significant rewrite. You can see the old examples in the [pre-0.1.0 branch](https://github.com/mistodon/gfx-hal-tutorials/tree/pre-0.1.0). You can also see the writeups here:

- [Part 0: Drawing a triangle](https://falseidolfactory.com/2018/08/16/gfx-hal-part-0-drawing-a-triangle.html)
- [Part 1: Resizing windows](https://falseidolfactory.com/2018/08/23/gfx-hal-part-1-resizing-windows.html)
- [Part 2: Vertex buffers](https://falseidolfactory.com/2018/10/09/gfx-hal-part-2-vertex-buffers.html)

**But** please bear in mind, they are not likely to be very useful, given how much the API has changed since they were written. Please look forward to the rewritten versions coming soon!
