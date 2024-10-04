// ELI5: why match the Cargo.toml package name, not directory name
// 'rust_webpack_template' not 'walk-the-dog'
import {
  main_js,
  set_depth,
  set_length,
  get_depth,
  get_length,
} from "../pkg/rust_webpack_template.js";

async function run() {
  // FIXME: this results in a blank HMTL page, which call is failing???
  // initialize the Wasm module
  // await init();

  const depthInput = document.getElementById("depth");
  const lengthInput = document.getElementById("length");
  const drawButton = document.getElementById("draw");

  // ensure these elements exist in HTML
  if (!depthInput || !lengthInput || !drawButton) {
    console.error("Missing HTML elements");
    const message = "Missing HTML elements";
    document.getElementById("output").innerText = message;
    return;
  } else {
    const message = "Yoyo Wasm, World";
    document.getElementById("output").innerText = message;
  }

  depthInput.value = get_depth();
  lengthInput.value = get_length();

  function updateTriangle() {
    const depth = parseInt(depthInput.value);
    const length = parseFloat(lengthInput.value);

    set_depth(depth);
    set_length(length);

    // Clear the canvas
    const canvas = document.getElementById("canvas");
    const ctx = canvas.getContext("2d");
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    // Draw the serpinski triangle
    main_js();
  }

  depthInput.addEventListener("change", updateTriangle);
  lengthInput.addEventListener("change", updateTriangle);
  drawButton.addEventListener("click", updateTriangle);

  // Initial draw
  main_js();
}

run().catch(console.error);
