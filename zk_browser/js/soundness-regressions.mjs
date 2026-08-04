import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gunzipSync } from "node:zlib";

const FP100 = 25_600;
const FIREBALL_SIZE = 18;
const FIREBALL_LIFE = 39;
const NG = 128;
const NB = 24;
const NI = 56;
const NJ = 160;
const NK = 32;
const NFC = 6;
const TPAD = 4000;

// Gzip/base64 copies of the four supplied legacy inputs. Original SHA-256:
// combined 6dacebf1..., pickup 305e8676..., projectile 734c85c0..., cap 8959f30e....
const cases = [
  ["combined false public statement", "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"],
  ["omitted pickup", "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"],
  ["skipped earlier projectile collision", "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"],
  ["continued after score cap", "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"],
];

function n(value) {
  return Number(value);
}

function d100(t) {
  return t <= 1433 ? 128 * t * t + 281728 * t : 648600 * t - 262880984;
}

function worldX(spawn, t) {
  return 25_600_000 - (d100(t) - d100(spawn - 1));
}

function fireX(fire, t) {
  return 4_505_600 + (t + 1 - fire) * 529_000;
}

function triQr(p) {
  return [Math.floor(p / 36), p % 36];
}

function collisionClassWitness(active, fire, count, capacity, spawnAt, edgesAt, phaseAt) {
  const selectors = Array(NFC * (capacity + 1)).fill("0");
  const first = Array(NFC).fill(String(TPAD));
  const tq = Array(2 * NFC).fill("0");
  const tr = Array(2 * NFC).fill("0");
  if (!active) return [selectors, first, tq, tr];

  const end = fire + FIREBALL_LIFE - 1;
  let idx = count;
  for (let i = 0; i < count; i++) {
    const anchor = Math.max(fire, Math.min(spawnAt(i), end));
    if (fireX(fire, anchor) < edgesAt(i, anchor)[1]) {
      idx = i;
      break;
    }
  }

  let required = true;
  for (let slot = 0; slot < NFC && required; slot++) {
    assert(idx <= capacity);
    selectors[slot * (capacity + 1) + idx] = "1";
    if (idx < count && spawnAt(idx) <= end) {
      const base = Math.max(fire, spawnAt(idx));
      let enter = base;
      while (enter <= end && !(edgesAt(idx, enter)[0] < fireX(fire, enter) + FIREBALL_SIZE * FP100)) {
        enter++;
      }
      assert(enter <= end);
      first[slot] = String(enter);
      for (let d = 0; d < 2; d++) {
        const t = enter + d;
        const [left, right] = edgesAt(idx, t);
        const fl = fireX(fire, t);
        if (fl < right && left < fl + FIREBALL_SIZE * FP100) {
          const [q, r] = triQr(phaseAt(idx, t));
          tq[2 * slot + d] = String(q);
          tr[2 * slot + d] = String(r);
        }
      }
      idx++;
    } else {
      required = false;
    }
  }
  assert(!required, "candidate capacity exceeded");
  return [selectors, first, tq, tr];
}

function upgradeLegacyInput(input) {
  const ticks = n(input.ticks);
  const itemCount = n(input.itemCount);
  input.iw1 = Array(NI).fill(String(TPAD));
  input.iw2 = Array(NI).fill(String(TPAD));
  input.ijsel = Array.from({ length: NI }, () => Array(NJ + 1).fill("0"));
  for (let i = 0; i < itemCount; i++) {
    const spawn = n(input.ispawn[i]);
    let w1 = spawn;
    while (worldX(spawn, w1) >= 4_300_800) w1++;
    let w2 = w1;
    while (worldX(spawn, w2 + 1) + 870_400 > 3_532_800) w2++;
    input.iw1[i] = String(w1);
    input.iw2[i] = String(w2);
    if (w1 <= ticks) {
      let vidx = 0;
      for (let j = 0; j < NJ && n(input.jact[j]) === 1; j++) {
        if (n(input.jtick[j]) <= w1) vidx = j + 1;
      }
      input.ijsel[i][vidx] = "1";
    }
  }

  input.kgsel = [];
  input.kbsel = [];
  input.kgfirst = [];
  input.kbfirst = [];
  input.kgq = [];
  input.kgr = [];
  input.kbq = [];
  input.kbr = [];
  const groundCount = n(input.groundCount);
  const batCount = n(input.batCount);
  for (let k = 0; k < NK; k++) {
    const active = n(input.kact[k]) === 1;
    const fire = active ? n(input.kfire[k]) : TPAD;
    const ground = collisionClassWitness(
      active,
      fire,
      groundCount,
      NG,
      (i) => n(input.gspawn[i]),
      (i, t) => {
        const x = worldX(n(input.gspawn[i]), t);
        return [x + 4 * FP100, x + (n(input.gw[i]) - 4) * FP100];
      },
      () => 0,
    );
    const bats = collisionClassWitness(
      active,
      fire,
      batCount,
      NB,
      (i) => n(input.bspawn[i]),
      (i, t) => {
        const x = worldX(n(input.bspawn[i]), t);
        return [x, x + 40 * FP100];
      },
      (i, t) => n(input.bphase[i]) + t - n(input.bspawn[i]),
    );
    input.kgsel.push(ground[0]);
    input.kgfirst.push(ground[1]);
    input.kgq.push(ground[2]);
    input.kgr.push(ground[3]);
    input.kbsel.push(bats[0]);
    input.kbfirst.push(bats[1]);
    input.kbq.push(bats[2]);
    input.kbr.push(bats[3]);
  }

  const previousDistance = d100(ticks === 0 ? 0 : ticks - 1);
  input.preScoreQ = String(Math.floor(previousDistance / 1_280_000));
  input.preScoreR = String(previousDistance % 1_280_000);
  return input;
}

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const wasm = path.resolve(process.argv[2] ?? "");
if (!process.argv[2]) {
  throw new Error("usage: node zk_browser/js/soundness-regressions.mjs <compiled dash_zk.wasm>");
}
const snarkjs = path.join(root, "zk_browser", "node_modules", ".bin", "snarkjs");
const scratch = mkdtempSync(path.join(tmpdir(), "dario-circuit-regressions-"));

function run(command, args) {
  return spawnSync(command, args, { cwd: root, encoding: "utf8" });
}

for (const seed of [4, 19, 42]) {
  const inputPath = path.join(scratch, `honest-${seed}.json`);
  const witnessPath = path.join(scratch, `honest-${seed}.wtns`);
  const exported = run("cargo", [
    "run", "-q", "--manifest-path", path.join(root, "dash_zk", "Cargo.toml"),
    "--bin", "export_input", "--", String(seed), inputPath,
  ]);
  assert.equal(exported.status, 0, exported.stderr);
  const witnessed = run(process.execPath, [
    snarkjs, "wtns", "calculate", wasm, inputPath, witnessPath,
  ]);
  assert.equal(witnessed.status, 0, witnessed.stderr || witnessed.stdout);
  console.log(`ok: honest seed ${seed}`);
}

for (const [name, compressed] of cases) {
  const input = upgradeLegacyInput(JSON.parse(gunzipSync(Buffer.from(compressed, "base64"))));
  const inputPath = path.join(scratch, `${name.replaceAll(" ", "-")}.json`);
  const witnessPath = path.join(scratch, `${name.replaceAll(" ", "-")}.wtns`);
  writeFileSync(inputPath, JSON.stringify(input));
  const witnessed = run(process.execPath, [
    snarkjs, "wtns", "calculate", wasm, inputPath, witnessPath,
  ]);
  assert.notEqual(witnessed.status, 0, `${name} unexpectedly produced a witness`);
  const output = `${witnessed.stdout}\n${witnessed.stderr}`;
  assert.match(output, /Assert Failed/, output);
  console.log(`ok: rejected ${name}`);
}
