import Sha1 from "sha.js/sha1.js";

export function createHash(algorithm) {
  if (algorithm !== "sha1") {
    throw new Error(`jj-wasm browser crypto shim only supports sha1, got ${algorithm}`);
  }
  const hash = new Sha1();
  return {
    update(chunk) {
      hash.update(chunk);
      return this;
    },
    digest(encoding) {
      const value = hash.digest();
      if (!encoding) {
        return value;
      }
      if (encoding === "hex") {
        return value.toString("hex");
      }
      throw new Error(`jj-wasm browser crypto shim only supports hex digest output, got ${encoding}`);
    },
  };
}

export default {
  createHash,
};
