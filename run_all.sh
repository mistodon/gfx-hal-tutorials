#!/bin/bash

set -e

cargo build --bins

./target/debug/part00-triangle
./target/debug/part01-resizing
./target/debug/part02-vertex-buffer
./target/debug/part03-uniforms
./target/debug/part04-push-constants
./target/debug/part05-no-depth
./target/debug/part05-depth
./target/debug/part06-textures
./target/debug/part07-render-to-texture

echo "Done"
