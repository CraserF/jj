import { Buffer } from "buffer";
import LightningFS from "@isomorphic-git/lightning-fs";
import git from "isomorphic-git";
import init, { JjSession } from "../pkg/jj_wasm.js";

globalThis.Buffer ??= Buffer;

const AUTHOR = {
  name: "jj-wasm",
  email: "jj-wasm@example.invalid",
};

export async function runWorkflow() {
  await init();

  const suffix = globalThis.crypto?.randomUUID?.() ?? Date.now();
  const fs = new LightningFS(`jj-wasm-${suffix}`);
  const pfs = fs.promises;
  const dir = "/repo";
  await pfs.mkdir(dir);

  const session = await JjSession.init({
    git,
    fs,
    dir,
    defaultBranch: "main",
  });

  await pfs.writeFile(`${dir}/README.md`, "hello from jj-wasm\n", "utf8");

  const initialStatus = await session.status();
  const snapshot = await session.snapshot({
    message: "initial browser snapshot",
    author: AUTHOR,
  });
  const described = await session.describe({
    message: "describe browser snapshot",
    author: AUTHOR,
  });

  await pfs.writeFile(`${dir}/README.md`, "hello again from jj-wasm\n", "utf8");
  const modifiedStatus = await session.status();

  const nextChange = await session.new({
    description: "browser follow-up change",
  });
  const restored = await session.restore({
    paths: ["README.md"],
    source: "HEAD",
    message: "restore README",
    author: AUTHOR,
  });
  const undo = await session.undo();
  const opLog = await session.opLog();
  const log = await session.log({ limit: 10 });

  return {
    initialStatus,
    snapshot,
    described,
    modifiedStatus,
    nextChange,
    restored,
    undo,
    opLog,
    log,
  };
}
