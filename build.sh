#!/bin/bash
cargo +nightly build --release
mkdir -p target/wasm32-unknown-unknown/release/Payload
cp res/* target/wasm32-unknown-unknown/release/Payload
cp target/wasm32-unknown-unknown/release/*.wasm target/wasm32-unknown-unknown/release/Payload/main.wasm
cd target/wasm32-unknown-unknown/release || exit
zip -r package.aix Payload
mv package.aix ../../../
echo "Build complete: package.aix"