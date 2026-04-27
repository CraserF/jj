export interface Sha1Hash {
  update(chunk: Uint8Array | string): Sha1Hash;
  digest(): Uint8Array;
  digest(encoding: "hex"): string;
}

export declare function createHash(algorithm: "sha1"): Sha1Hash;

declare const cryptoShim: {
  createHash: typeof createHash;
};

export default cryptoShim;
