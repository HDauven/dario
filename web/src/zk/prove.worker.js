// Web Worker that proves a Dario Dash run entirely in-browser.
//
// Receives the circom input JSON (string) plus the circuit wasm and proving
// key as ArrayBuffers, runs snarkjs groth16.fullProve, and returns the
// 128-byte ark-compressed proof (as expected by Dusk's
// `verify_groth16_bn254` host function) together with the public signals.

import { groth16 } from "snarkjs";
import { proofToArkBytes } from "./ark-proof.mjs";

self.onmessage = async (e) => {
  const { input, circuitWasm, zkey } = e.data;
  try {
    self.postMessage({ type: "status", message: "Computing witness + proof..." });
    const started = performance.now();
    const { proof, publicSignals } = await groth16.fullProve(
      JSON.parse(input),
      { type: "mem", data: new Uint8Array(circuitWasm) },
      { type: "mem", data: new Uint8Array(zkey) }
    );
    const elapsedMs = Math.round(performance.now() - started);
    const arkProof = proofToArkBytes(proof);
    self.postMessage(
      { type: "done", arkProof, publicSignals, elapsedMs },
      [arkProof.buffer]
    );
  } catch (err) {
    self.postMessage({ type: "error", message: err?.message || String(err) });
  }
};
