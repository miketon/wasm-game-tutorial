// NOTE: should match the Cargo.toml package name, not directory name
// 'rust_webpack_template' not 'walk-the-dog'
import { main_js } from '../pkg/rust_webpack_template.js';

async function run() {
  // Initialize the WebAssembly module
  const wasm = main_js();

  // Check what functions are available
  console.log("Available exports : ", Object.keys(wasm));

  // if main_js is available use it
  if (wasm.main_js) {
    console.log("main_js IS available");
    main_js();
  }
  else {
    console.log("main_js NOT available");
  }
  const message = ("Yo Wasm, World");
  document.getElementById('output').innerText = message;
}

run().catch(console.error);
