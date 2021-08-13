# Pong
Pong clone so I can practice getting better at wgpu. Very good game.

## Testing
``
cargo run
``

## Building
### WASM
``
cargo build --target wasm32-unknown-unknown --release
``

``
~/.cargo/bin/wasm-bindgen --out-dir target/wasm32-unknown-unknown/generated  --target web target/wasm32-unknown-unknown/release/pong-wgpu.wasm
``

Output in **target/wasm32-unknown-unknown/generated** (the .js and the .wasm)

### Everything Else
Everything else can produce a simple binary, using the build command:

``
cargo build --target {target} --release
``