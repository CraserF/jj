import { runWorkerWorkflow } from "./worker-workflow.js";

const output = document.querySelector("#output");

try {
  const result = await runWorkerWorkflow();
  output.dataset.status = "done";
  output.textContent = JSON.stringify(result, null, 2);
} catch (error) {
  output.dataset.status = "error";
  output.textContent = error?.stack || String(error);
  throw error;
}
