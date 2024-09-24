// NOTE: should match the Cargo.toml package name, not directory name
// 'rust_webpack_template' not 'walk-the-dog'
import { main_js } from '../pkg/rust_webpack_template.js';

async function run() {
  // Initialize the WebAssembly module
  main_js();

  const message = ("Yoyo Wasm, World");
  document.getElementById('output').innerText = message;

}

run().catch(console.error);
