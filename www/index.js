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

// object to store the elements
const elements = {};

const DELAY_MS = 250;

// #region Main
async function run() {
  try {
    document.getElementById(ELEMENT_IDS.OUTPUT).innerText =
      "0 Error Wasm World!";
    logMessage("Starting run function");

    // get all elements
    Object.entries(ELEMENT_IDS).forEach(([key, id]) => {
      elements[key] = document.getElementById(id);
    });

    // check and thow error if any element are missing
    const missingElements = Object.entries(elements)
      .filter(([, element]) => !element)
      .map(([key]) => key);

    if (missingElements.length > 0) {
      throw new Error(`Missing HTML elements: ${missingElements.join(",")}`);
    }

    // get values from public wasm-bindgen functions
    elements.DEPTH.value = get_depth();
    elements.LENGTH.value = get_length();

    [elements.DEPTH, elements.DRAW].forEach((element) => {
      // update to "click" becuase with "change" button click would NOT redraw
      element.addEventListener("click", () => updateTriangle(elements), {
        passive: true,
      });
    });

    // tune compute cost of rapid value change vs responsiveness
    // - create a debounced version of updateTriangle
    const debounceUpdateTriangle = debounce(
      () => updateTriangle(elements),
      DELAY_MS,
    );

    // - use debounced triangle update for input events
    elements.LENGTH.addEventListener("input", debounceUpdateTriangle, {
      passive: true,
    });

    updateTriangle(elements);
  } catch (error) {
    logMessage(`Error in run function: ${error.message}`, true);
  }
}

run().catch((error) => logMessage(`Unhandled error : ${error.message}`, true));
// #endregion

// #region Draw
function updateTriangle(elements) {
  const depth = parseInt(elements.DEPTH.value);
  const length = parseFloat(elements.LENGTH.value);

  // ensure these elements exist in HTML
  if (isNaN(depth) || isNaN(length)) {
    logMessage("Invalid input values", true);
    return;
  }

  set_depth(depth);
  set_length(length);

  // Clear the canvas
  clearCanvas(elements.CANVAS);
  // Draw the serpinski triangle
  main_js();
}

function clearCanvas(canvas) {
  if (!canvas) {
    logMessage("Canvas element not found");
    return;
  }
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, canvas.width, canvas.height);
}
// #endregion

// #region Utils
function debounce(func, waitFor) {
  let timeout = null;

  return function (...args) {
    if (timeout !== null) {
      clearTimeout(timeout);
    }
    timeout = setTimeout(() => func(...args), waitFor);
  };
}

function logMessage(message, isError = false) {
  // append messages if Error
  if (elements.OUTPUT && isError) {
    elements.OUTPUT.innerText += `${message}\n`;
  }

  message = `[index.js] ${message}`;
  if (isError) {
    console.error(message);
  } else {
    console.log(message);
  }
}
//#endregion
