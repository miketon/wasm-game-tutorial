// ELI5: why match the Cargo.toml package name, not directory name
// 'rust_webpack_template' not 'walk-the-dog'
import {
  main_js,
  set_depth,
  set_length,
  get_depth,
  get_length,
} from "../pkg/rust_webpack_template.js";

const ELEMENT_IDS = {
  OUTPUT: "output",
  DEPTH: "depth",
  LENGTH: "length",
  DRAW: "draw",
  CANVAS: "canvas",
};

async function run() {
  try {
    document.getElementById(ELEMENT_IDS.OUTPUT).innerText =
      "0 Error Wasm World!";
    logMessage("Starting run function");
    // get all elements and log if any are missing
    const elements = Object.entries(ELEMENT_IDS).map(([key, id]) => [
      key,
      document.getElementById(id),
    ]);
    logMessage(
      `Found [${elements.length}/${Object.keys(ELEMENT_IDS).length}] HTML elements`,
    );
    const missingElements = elements
      .filter(([, element]) => !element)
      .map(([key]) => key);

    if (missingElements.length > 0) {
      throw new Error(`Missing HTML elements: ${missingElements.join(",")}`);
    }

    // Destructuring elements, skipping the OUTPUT element
    // FIXED: Correct way to destructure the elements

    /** @type {HTMLInputElement} */
    const depthInput = elements.find(([key]) => key === "DEPTH")[1];
    /** @type {HTMLInputElement} */
    const lengthInput = elements.find(([key]) => key === "LENGTH")[1];
    const drawButton = elements.find(([key]) => key === "DRAW")[1];
    const canvasElement = elements.find(([key]) => key === "CANVAS")[1];

    // Log the type of each element
    logMessage(`depthInput type: ${depthInput.constructor.name}`);
    logMessage(`lengthInput type: ${lengthInput.constructor.name}`);
    logMessage(`drawButton type: ${drawButton.constructor.name}`);
    logMessage(`canvasElement type: ${canvasElement.constructor.name}`);

    depthInput.value = get_depth();
    lengthInput.value = get_length();

    // interesting even drawButton listens to "change" as opposed to "click"
    // TODO: Explain the tradeoff
    [depthInput, lengthInput, drawButton].forEach((element) => {
      logMessage("Adding : " + element.id);
      element.addEventListener("input", updateTriangle, { passive: true });
    });

    updateTriangle();
  } catch (error) {
    logMessage(`Error in run function: ${error.message}`, true);
  }
}

run().catch((error) => logMessage(`Unhandled error : ${error.message}`, true));

function updateTriangle() {
  const depthInput = document.getElementById(ELEMENT_IDS.DEPTH);
  const lengthInput = document.getElementById(ELEMENT_IDS.LENGTH);

  const depth = parseInt(depthInput.value);
  const length = parseFloat(lengthInput.value);

  // ensure these elements exist in HTML
  if (isNaN(depth) || isNaN(length)) {
    logMessage("Invalid input values", true);
    return;
  }

  set_depth(depth);
  set_length(length);

  // Clear the canvas
  clearCanvas();
  // Draw the serpinski triangle
  main_js();
}

function clearCanvas() {
  const canvas = document.getElementById(ELEMENT_IDS.CANVAS);
  if (!canvas) {
    logMessage("Canvas element not found");
    return;
  }
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, canvas.width, canvas.height);
}

function logMessage(message, isError = false) {
  const outElement = document.getElementById(ELEMENT_IDS.OUTPUT);
  // append messages if Error
  if (outElement && isError) {
    outElement.innerText += message + "\n";
  }
  if (isError) {
    console.error(message);
  } else {
    console.log(message);
  }
}
