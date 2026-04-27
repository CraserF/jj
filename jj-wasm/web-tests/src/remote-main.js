import { runRemoteWorkflow } from "./remote-workflow.js";

const output = document.querySelector("#output");

try {
  const params = new URLSearchParams(location.search);
  const result = await runRemoteWorkflow({
    remoteUrl: requiredParam(params, "remoteUrl"),
    controlUrl: requiredParam(params, "controlUrl"),
    pushRef: params.get("pushRef") ?? "refs/heads/jj-wasm-test",
  });
  output.dataset.status = "done";
  output.textContent = JSON.stringify(result, null, 2);
} catch (error) {
  output.dataset.status = "error";
  output.textContent = error?.stack || String(error);
  throw error;
}

function requiredParam(params, name) {
  const value = params.get(name);
  if (!value) {
    throw new Error(`missing required remote harness query parameter: ${name}`);
  }
  return value;
}
