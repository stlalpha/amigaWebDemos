#!/bin/bash

cargo clean
# Run wasm-pack build and check its exit status
if ! wasm-pack build --target web; then
    echo "wasm-pack build failed"
    exit 1
fi

rm -rf www/pkg  # Remove old pkg directory if it exists
cp -r pkg www/  # Copy new pkg directory
cd www && basic-http-server .
cd ..
