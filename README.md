# rust-todomvc

 [The TodoMVC app](https://github.com/tastejs/todomvc/blob/master/app-spec.md)
 implemented entirely in Rust using emscripten.

 Built on top of the [http://github.com/tcr/rust-webplatform] library.

## Compilation

Compiling rust-todomvc for the browser requires a nightly Rust.

```
rustup install nightly
rustup override set nightly
rustup target add asmjs-unknown-emscripten
rustup target add wasm32-unknown-emscripten
```

You should also set up emscripten:

```
curl -O https://s3.amazonaws.com/mozilla-games/emscripten/releases/emsdk-portable.tar.gz
tar -xzf emsdk-portable.tar.gz
source emsdk_portable/emsdk_env.sh
emsdk update
emsdk install sdk-incoming-64bit
emsdk activate sdk-incoming-64bit
```

Then you're ready to build:

```
cargo build --target=asmjs-unknown-emscripten
cp target/asmjs-unknown-emscripten/debug/todomvc.js static
cd static; python -m SimpleHTTPServer
```

Open `http://localhost:8000/`. There you go!

See [brson's post on Rust and emscripten](https://users.rust-lang.org/t/compiling-to-the-web-with-rust-and-emscripten/7627) for more installation details.

## License

MIT or Apache-2.0, at your option.
