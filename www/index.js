import init, { greet } from '../pkg/walk_the_dog.js';

async function run() {
  await init();
  const message = greet("World");
  document.getElementById('output').innerText = message;
}

run();
