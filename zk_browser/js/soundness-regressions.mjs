import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gunzipSync } from "node:zlib";

const NG = 128;
const BAT_PERIOD = 36;
const SCORE_DENOMINATOR = 1_280_000;

// The supplied seed-42 omitted-pickup PoC (source SHA-256
// 305e8676a25b4ff63e000abd23abbe304a9610e18d52ae83555ecc7eeef76735)
// upgraded once to the current input schema. Its SHA-256 as uncompressed
// canonical JSON is pinned so the focused control below cannot silently drift
// with helper code.
const OMITTED_PICKUP_SHA256 =
  "169a44d9cc99dbae918392250396ab0429f473f43b32fcbbe639cdbad47df812";
const OMITTED_PICKUP_FIXTURE = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "fixtures",
  "omitted-pickup-seed42.json.gz.base64",
);

function omittedPickupFixture() {
  const encoded = readFileSync(OMITTED_PICKUP_FIXTURE, "utf8").trim();
  const json = gunzipSync(Buffer.from(encoded, "base64"));
  assert.equal(
    createHash("sha256").update(json).digest("hex"),
    OMITTED_PICKUP_SHA256,
    "omitted-pickup fixture digest mismatch",
  );
  return JSON.parse(json);
}

function n(value) {
  return Number(value);
}

function d100(t) {
  return t <= 1433 ? 128 * t * t + 281728 * t : 648600 * t - 262880984;
}

function triQr(p) {
  return [Math.floor(p / BAT_PERIOD), p % BAT_PERIOD];
}

function clone(input) {
  return structuredClone(input);
}

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const circuit = path.join(root, "zk_browser", "circuits", "dash_zk.circom");
const nodeModules = path.join(root, "zk_browser", "node_modules");
const snarkjs = path.join(nodeModules, ".bin", "snarkjs");
const patchedWasm = path.resolve(process.argv[2] ?? "");
if (!process.argv[2]) {
  throw new Error(
    "usage: node zk_browser/js/soundness-regressions.mjs <compiled dash_zk.wasm>",
  );
}

const scratch = mkdtempSync(path.join(tmpdir(), "dario-circuit-regressions-"));
let calculation = 0;

function run(command, args) {
  return spawnSync(command, args, { cwd: root, encoding: "utf8" });
}

function outputOf(result) {
  return `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
}

function calculate(wasm, input, label) {
  const slug = `${String(calculation++).padStart(3, "0")}-${label.replaceAll(/[^a-z0-9]+/gi, "-")}`;
  const inputPath = path.join(scratch, `${slug}.json`);
  const witnessPath = path.join(scratch, `${slug}.wtns`);
  writeFileSync(inputPath, JSON.stringify(input));
  return run(process.execPath, [
    snarkjs,
    "wtns",
    "calculate",
    wasm,
    inputPath,
    witnessPath,
  ]);
}

function expectWitness(wasm, input, label) {
  const result = calculate(wasm, input, label);
  assert.equal(result.status, 0, `${label}\n${outputOf(result)}`);
}

function expectRejected(wasm, input, label) {
  const result = calculate(wasm, input, label);
  assert.notEqual(result.status, 0, `${label} unexpectedly produced a witness`);
  assert.match(outputOf(result), /Assert Failed/, outputOf(result));
}

function exportInput(seed) {
  const inputPath = path.join(scratch, `honest-${seed}.json`);
  const exported = run("cargo", [
    "run",
    "-q",
    "--manifest-path",
    path.join(root, "dash_zk", "Cargo.toml"),
    "--bin",
    "export_input",
    "--",
    String(seed),
    inputPath,
  ]);
  assert.equal(exported.status, 0, outputOf(exported));
  return JSON.parse(readFileSync(inputPath, "utf8"));
}

// Compile a circuit differing by exactly one disabled enforcement constraint.
// A malformed input must witness successfully here before its failure against
// the patched circuit can be attributed to that specific relation.
function compileControl(name, needle, replacement) {
  const source = readFileSync(circuit, "utf8");
  assert.equal(source.split(needle).length, 2, `${name}: control needle is not unique`);
  const controlSource = source.replace(needle, replacement);
  const sourcePath = path.join(scratch, `${name}.circom`);
  const outputDir = path.join(scratch, `${name}-build`);
  mkdirSync(outputDir);
  writeFileSync(sourcePath, controlSource);
  const compiled = run(process.env.CIRCOM ?? "circom", [
    sourcePath,
    "--wasm",
    "-o",
    outputDir,
    "-l",
    nodeModules,
  ]);
  assert.equal(compiled.status, 0, `${name} control compile\n${outputOf(compiled)}`);
  return path.join(outputDir, `${name}_js`, `${name}.wasm`);
}

function findControlledMutation(name, candidates, controlWasm) {
  const failures = [];
  for (const candidate of candidates) {
    const result = calculate(controlWasm, candidate.input, `${name}-control`);
    if (result.status === 0) {
      return candidate;
    }
    failures.push(`${candidate.description}: ${outputOf(result).trim().split("\n").at(-1)}`);
  }
  assert.fail(`${name}: no isolated mutation satisfied its control circuit\n${failures.join("\n")}`);
}

function* delayedPickupCandidates(base) {
  for (let e = 0; n(base.eact[e]) === 1; e++) {
    if (n(base.ekind[e]) > 2) continue;
    const item = base.eisel[e].findIndex((selected) => n(selected) === 1);
    assert.notEqual(item, -1, `pickup event ${e} has no item selector`);
    const oldTick = n(base.etick[e]);
    for (let delta = 1; delta <= 7; delta++) {
      const newTick = oldTick + delta;
      if (newTick > n(base.ticks)) break;

      // Keep every form-at and last-jump selector unchanged; the only semantic
      // mutation is moving this automatic pickup later within its overlap.
      const crossesJump = base.jtick.some(
        (tick, j) => n(base.jact[j]) === 1 && n(tick) > oldTick && n(tick) <= newTick,
      );
      const crossesFire = base.kfire.some(
        (tick, k) => n(base.kact[k]) === 1 && n(tick) > oldTick && n(tick) <= newTick,
      );
      if (crossesJump || crossesFire) continue;

      if (e + 1 < base.eact.length && n(base.eact[e + 1]) === 1) {
        const nextTick = n(base.etick[e + 1]);
        const nextKind = n(base.ekind[e + 1]);
        if (newTick > nextTick || (newTick === nextTick && nextKind < 3)) continue;
      }

      const input = clone(base);
      input.etick[e] = String(newTick);
      yield {
        input,
        description: `item ${item}, event ${e}, tick ${oldTick} -> ${newTick}`,
      };
    }
  }
}

function* delayedCollisionCandidates(base) {
  for (let k = 0; n(base.kact[k]) === 1; k++) {
    const target = base.kosel[k].findIndex((selected) => n(selected) === 1);
    assert.notEqual(target, -1, `kill ${k} has no obstacle selector`);
    const oldTick = n(base.khit[k]);
    const newTick = oldTick + 1;
    const input = clone(base);
    input.khit[k] = String(newTick);
    if (target < NG) {
      input.gevt[target] = String(newTick);
    } else {
      const bat = target - NG;
      input.bevt[bat] = String(newTick);
      const [q, r] = triQr(n(input.bphase[bat]) + newTick - n(input.bspawn[bat]));
      input.ktq[k] = String(q);
      input.ktr[k] = String(r);
    }
    yield {
      input,
      description: `kill ${k}, target ${target}, tick ${oldTick} -> ${newTick}`,
    };
  }
}

function* postCapCandidates(base) {
  const oldTick = n(base.ticks);
  assert.equal(n(base.score), 1500, "post-cap positive control must end at score 1500");
  for (let delta = 1; delta <= 8; delta++) {
    const newTick = oldTick + delta;
    const input = clone(base);
    input.ticks = String(newTick);
    const distance = d100(newTick);
    input.scoreQ = String(Math.floor(distance / SCORE_DENOMINATOR));
    input.scoreR = String(distance % SCORE_DENOMINATOR);
    const previousDistance = d100(newTick - 1);
    input.preScoreQ = String(Math.floor(previousDistance / SCORE_DENOMINATOR));
    input.preScoreR = String(previousDistance % SCORE_DENOMINATOR);
    yield {
      input,
      description: `terminal tick ${oldTick} -> ${newTick}`,
    };
  }
}

const honest = new Map();
for (const seed of [4, 19, 42]) {
  const input = exportInput(seed);
  expectWitness(patchedWasm, input, `honest-seed-${seed}`);
  honest.set(seed, input);
  console.log(`ok: honest seed ${seed}`);
}

const pickupTimingControl = compileControl(
  "pickup_timing_control",
  "iPickCount[i] * (iPickTick[i] - ifdot[i].out) === 0;",
  "iPickCount[i] * (iPickTick[i] - ifdot[i].out) * 0 === 0;",
);
const delayedPickup = findControlledMutation(
  "delayed pickup",
  delayedPickupCandidates(honest.get(42)),
  pickupTimingControl,
);
expectRejected(patchedWasm, delayedPickup.input, "delayed-pickup-patched");
console.log(`ok: rejected isolated delayed pickup (${delayedPickup.description})`);

const pickupCompletenessControl = compileControl(
  "pickup_completeness_control",
  "iPickCount[i] === iseen[i][8];",
  "iPickCount[i] * 0 === 0;",
);
const omittedPickup = omittedPickupFixture();
expectWitness(
  pickupCompletenessControl,
  omittedPickup,
  "omitted-pickup-completeness-control",
);
expectRejected(patchedWasm, omittedPickup, "omitted-pickup-patched");
console.log("ok: rejected isolated omitted pickup from supplied seed-42 PoC");

const collisionControl = compileControl(
  "collision_control",
  "no2[s][d] === 0;",
  "no2[s][d] * 0 === 0;",
);
const delayedCollision = findControlledMutation(
  "skipped first projectile collision",
  delayedCollisionCandidates(honest.get(4)),
  collisionControl,
);
expectRejected(patchedWasm, delayedCollision.input, "delayed-collision-patched");
console.log(`ok: rejected isolated later collision (${delayedCollision.description})`);

const capControl = compileControl(
  "cap_control",
  "scoreCapped.out * (1 - preRawLtCap.out) === 0;",
  "scoreCapped.out * (1 - preRawLtCap.out) * 0 === 0;",
);
const postCap = findControlledMutation(
  "continued after score cap",
  postCapCandidates(honest.get(19)),
  capControl,
);
expectRejected(patchedWasm, postCap.input, "post-cap-patched");
console.log(`ok: rejected isolated post-cap continuation (${postCap.description})`);
