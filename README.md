# Snooze

For Ludum Dare 50: https://ldjam.com/events/ludum-dare/50

https://ldjam.com/events/ludum-dare/50/snooze-2

## Building for web
### Prerequisites
* `rustup target install wasm32-unknown-unknown`
* `cargo install wasm-bindgen-cli`
### Build
1. `cargo build --release --target wasm32-unknown-unknown`
1. `wasm-bindgen --out-dir out --target web target/wasm32-unknown-unknown/release/ludum-dare-50.wasm`
1. `cp index.html out`
1. `cp -r assets out`
