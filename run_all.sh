#!/bin/bash

set -e

cargo run --bin part00-triangle && \
cargo run --bin part01-resizing && \
cargo run --bin part02-vertex-buffer && \
cargo run --bin part03-uniforms && \
cargo run --bin part04-push-constants && \
cargo run --bin part05-no-depth && \
cargo run --bin part05-depth && \
cargo run --bin part06-textures && \
cargo run --bin part07-render-to-texture && \
echo "Done"
