import { createDuskApp, DUSK_CHAIN_PRESETS } from "@dusk/connect";
import { defineDuskConnectButton } from "@dusk/connect/ui";
import "./styles.css";

import initDashWasm, { ZkDashSim } from "./dash-wasm/dash_web.js";
import { STATE } from "./fsm.js";
import { createGame } from "./game.js";

import regularSprite from "./assets/dario-regular-new.png";
import superSprite from "./assets/dario-super-new.png";
import fireSprite from "./assets/dario-fire-new.png";
import capeSprite from "./assets/dario-cape-new.png";
import gameOverSprite from "./assets/dario-gameover-new.png";

defineDuskConnectButton();

const CONTRACT_ID = import.meta.env.VITE_DARIO_CONTRACT_ID || "";
const NODE_URL =
  import.meta.env.VITE_DUSK_NODE_URL || "https://testnet.nodes.dusk.network";
// One of: local, mainnet, testnet, devnet (see DUSK_CHAIN_PRESETS).
const CHAIN_PRESET =
  DUSK_CHAIN_PRESETS[import.meta.env.VITE_DUSK_CHAIN || "testnet"] ||
  DUSK_CHAIN_PRESETS.testnet;
const DRIVER_URL = `${import.meta.env.BASE_URL}data_driver.wasm?v=${Date.now()}`;
const HAS_CONTRACT = /^0x[0-9a-fA-F]{64}$/.test(CONTRACT_ID);

const STATE_NAMES = ["Regular", "Super", "Fire", "Cape", "Game Over"];

const SPRITE_URLS = {
  [STATE.Regular]: regularSprite,
  [STATE.Super]: superSprite,
  [STATE.Fire]: fireSprite,
  [STATE.Cape]: capeSprite,
  [STATE.GameOver]: gameOverSprite,
};

const dusk = HAS_CONTRACT
  ? createDuskApp({
      nodeUrl: NODE_URL,
      chain: { chainId: CHAIN_PRESET },
      autoConnect: true,
      contracts: {
        dario: {
          contractId: CONTRACT_ID,
          driverUrl: DRIVER_URL,
          name: "Dario FSM",
          methodSigs: {
            current_state: "current_state()",
            revive_count: "revive_count()",
            current_state_for: "current_state_for(String)",
            revive_count_for: "revive_count_for(String)",
            handle_event: "handle_event(u32)",
            submit_run: "submit_run(u64, u64, u32, Vec < u8 >)",
            submit_zk_run: "submit_zk_run(u64, u64, u32, Vec < u8 >)",
            best_score_for: "best_score_for(String)",
            proven_runs_for: "proven_runs_for(String)",
            leaderboard: "leaderboard()",
          },
        },
      },
    })
  : null;

const wallet = dusk?.wallet ?? null;
const dario = dusk?.contract("dario") ?? null;

const connectBtn = document.getElementById("connectBtn");
if (connectBtn && wallet) connectBtn.wallet = wallet;

const $ = (id) => document.getElementById(id);

const elCabinet = document.querySelector(".cabinet");
const elCanvas = $("gameCanvas");
const elHudState = $("hudState");
const elHudRuns = $("hudRuns");
const elHudAccount = $("hudAccount");
const elHint = $("hint");
const elStartOverlay = $("startOverlay");
const elStartText = $("startText");
const elPlayBtn = $("playBtn");
const elConnectStartBtn = $("connectStartBtn");
const elOverOverlay = $("gameOverOverlay");
const elOverScore = $("overScore");
const elOverStats = $("overStats");
const elCommitStatus = $("commitStatus");
const elDownloadRunBtn = $("downloadRunBtn");
const elSubmitProofBtn = $("submitProofBtn");
const elAgainBtn = $("againBtn");

const model = {
  bestOnChain: null,
  runs: null,
  driverReady: false,
  submitting: false,
  submitted: false,
  submitError: null,
  proveStatus: null,
  lastRun: null,
  liveForm: STATE.Regular,
  error: null,
};

let lastAccount = "";

function selectedAccount() {
  return (
    wallet?.state.selectedProfile?.account || wallet?.state.accounts?.[0] || ""
  );
}

function shorten(value) {
  const s = String(value || "");
  if (!s) return "Not connected";
  if (s.length <= 18) return s;
  return `${s.slice(0, 10)}...${s.slice(-6)}`;
}

function connected() {
  return Boolean(wallet?.state.authorized && selectedAccount());
}

function canSubmit() {
  return HAS_CONTRACT && model.driverReady && connected();
}

function setText(node, text) {
  if (!node) return;
  const value = node.querySelector?.(".hudValue");
  if (value) value.textContent = text;
  else node.textContent = text;
}

// Sprites, preloaded for canvas drawing.
const sprites = {};
for (const [key, url] of Object.entries(SPRITE_URLS)) {
  const img = new Image();
  img.src = url;
  sprites[key] = img;
}

let game = null;

function render() {
  const account = selectedAccount();
  const isConnected = connected();
  const playing = Boolean(game?.running);
  const over = Boolean(game?.over);

  if (elCabinet) {
    elCabinet.dataset.state = String(playing ? model.liveForm : 0);
  }

  setText(elHudState, playing ? STATE_NAMES[model.liveForm] : "Idle");
  setText(
    elHudRuns,
    model.bestOnChain == null ? "-" : `${model.bestOnChain} (${model.runs ?? 0} runs)`
  );
  setText(elHudAccount, shorten(account));

  if (elStartOverlay) elStartOverlay.hidden = playing || over;
  if (elOverOverlay) elOverOverlay.hidden = playing || !over;

  if (elStartText) {
    if (!game) {
      elStartText.textContent = "Loading deterministic sim (wasm)...";
    } else if (!HAS_CONTRACT) {
      elStartText.textContent =
        "Free play mode. Set VITE_DARIO_CONTRACT_ID to prove runs on-chain.";
    } else if (!isConnected) {
      elStartText.textContent =
        "Play free, or connect your wallet to prove runs on Testnet.";
    } else {
      elStartText.textContent =
        "Wallet connected. Finished runs can be ZK-proven and submitted on-chain.";
    }
  }
  if (elConnectStartBtn) {
    elConnectStartBtn.hidden = !HAS_CONTRACT || isConnected;
  }
  if (elPlayBtn) elPlayBtn.disabled = !game;

  if (over && model.lastRun) {
    const run = model.lastRun;
    if (elOverScore) {
      elOverScore.textContent = `Score ${run.score}${
        run.score >= run.best ? " — New Best!" : ""
      }`;
    }
    if (elOverStats) {
      elOverStats.textContent = `Best ${run.best} · ${run.pickups} power-ups · ${run.kills} fried · ${Math.floor(run.distance / 50)}m · ${run.ticks} ticks`;
    }
    if (elDownloadRunBtn) {
      elDownloadRunBtn.hidden = !connected();
    }
    if (elSubmitProofBtn) {
      elSubmitProofBtn.hidden = !canSubmit();
      elSubmitProofBtn.disabled = model.submitting || model.submitted;
      elSubmitProofBtn.textContent = model.submitted
        ? "Proof Verified On-Chain ✔"
        : model.submitting
          ? "Proving..."
          : "Prove In-Browser & Submit";
    }
    if (elCommitStatus) {
      if (model.submitError) {
        elCommitStatus.textContent = model.submitError;
      } else if (model.submitting) {
        elCommitStatus.textContent = model.proveStatus || "Proving in browser...";
      } else if (model.submitted) {
        elCommitStatus.textContent = `Proven! On-chain best: ${model.bestOnChain ?? "?"}`;
      } else if (!canSubmit()) {
        elCommitStatus.textContent = HAS_CONTRACT
          ? "Connect your wallet to prove this run on-chain."
          : "";
      } else {
        elCommitStatus.textContent =
          "Generate a Groth16 proof of this run right here in your browser.";
      }
    }
    if (elAgainBtn) elAgainBtn.disabled = model.submitting;
  }

  if (!elHint) return;
  if (playing) {
    elHint.textContent =
      "Space / tap: jump · hold to glide with Cape · F: fireball with Fire";
  } else if (model.error) {
    elHint.textContent = model.error;
  } else if (over) {
    elHint.textContent = model.submitting
      ? "Submitting proof on-chain..."
      : "Run over. Prove it on-chain or dash again.";
  } else {
    elHint.textContent =
      "Collect ☕ 🌶️ 🧣 to power up. Don't get hit as Regular!";
  }
}

async function sync() {
  if (!dario || !connected()) {
    model.bestOnChain = null;
    model.runs = null;
    render();
    return;
  }

  try {
    const account = selectedAccount();
    const [best, runs] = await Promise.all([
      dario.call.best_score_for(account),
      dario.call.proven_runs_for(account),
    ]);
    model.bestOnChain = Number(best);
    model.runs = Number(runs);
    model.error = null;
  } catch {
    model.error = "Unable to sync on-chain state.";
  }
  render();
}

const elRankingPanel = $("rankingPanel");
const elRankingList = $("rankingList");
const elRankingEmpty = $("rankingEmpty");

// Fetches `leaderboard()` (Vec<(account, best_score, plays)>) and renders the
// top 10, highlighting the connected account.
async function refreshLeaderboard() {
  if (!dario || !model.driverReady || !elRankingPanel) return;
  try {
    const rows = await dario.call.leaderboard();
    elRankingPanel.hidden = false;
    const entries = (rows || [])
      .map((r) => ({ account: String(r[0]), score: Number(r[1]), plays: Number(r[2]) }))
      .sort((a, b) => b.score - a.score)
      .slice(0, 10);
    const self = connected() ? String(selectedAccount()) : null;
    elRankingList.innerHTML = "";
    for (const [i, e] of entries.entries()) {
      const li = document.createElement("li");
      if (self && String(e.account) === self) li.classList.add("rankingSelf");
      const acct = `${e.account.slice(0, 8)}…${e.account.slice(-8)}`;
      const values = [
        ["rankPos", i + 1],
        ["rankAcct", acct],
        ["rankScore", e.score],
        ["rankPlays", `${e.plays} run${e.plays === 1 ? "" : "s"}`],
      ];
      for (const [className, value] of values) {
        const span = document.createElement("span");
        span.className = className;
        span.textContent = String(value);
        if (className === "rankAcct") span.title = String(e.account);
        li.appendChild(span);
      }
      elRankingList.appendChild(li);
    }
    elRankingEmpty.hidden = entries.length > 0;
  } catch {
    // Leaderboard is best-effort; leave the panel as-is on failure.
  }
}

function traceToBase64(trace) {
  let bin = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < trace.length; i += CHUNK) {
    bin += String.fromCharCode(...trace.subarray(i, i + CHUNK));
  }
  return btoa(bin);
}

// Downloads the raw run (seed + 30 Hz input trace) for debugging/replay.
function downloadRunFile() {
  const run = model.lastRun;
  if (!run) return;
  const payload = {
    account: selectedAccount(),
    seed: run.seed.toString(),
    trace_b64: traceToBase64(run.trace),
  };
  const blob = new Blob([JSON.stringify(payload, null, 1)], {
    type: "application/json",
  });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `dario-run-${run.score}.json`;
  a.click();
  URL.revokeObjectURL(a.href);
}

// Submits a proof generated entirely in-browser to the contract.

const BASE58_ALPHABET =
  "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/** Decodes a base58 Moonlight address to its raw 96 bytes. */
function base58Decode(str) {
  let num = 0n;
  for (const ch of str) {
    const idx = BASE58_ALPHABET.indexOf(ch);
    if (idx < 0) throw new Error("Invalid account encoding");
    num = num * 58n + BigInt(idx);
  }
  const bytes = [];
  while (num > 0n) {
    bytes.unshift(Number(num & 0xffn));
    num >>= 8n;
  }
  for (const ch of str) {
    if (ch !== "1") break;
    bytes.unshift(0);
  }
  return new Uint8Array(bytes);
}

function bytesToHex(bytes) {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

function setProveStatus(message) {
  model.proveStatus = message;
  render();
}

/** Fetches a proving artifact, reporting download progress. */
async function fetchArtifact(url, label) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch ${label} (${res.status})`);
  const total = Number(res.headers.get("content-length")) || 0;
  const reader = res.body.getReader();
  const chunks = [];
  let received = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    received += value.length;
    if (total) {
      const pct = Math.floor((received / total) * 100);
      setProveStatus(`Downloading ${label}... ${pct}%`);
    } else {
      setProveStatus(`Downloading ${label}... ${(received / 1e6).toFixed(0)} MB`);
    }
  }
  const out = new Uint8Array(received);
  let off = 0;
  for (const chunk of chunks) {
    out.set(chunk, off);
    off += chunk.length;
  }
  return out.buffer;
}

/** Runs snarkjs groth16.fullProve in a Web Worker. */
function proveInWorker(input, circuitWasm, zkey) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./zk/prove.worker.js", import.meta.url), {
      type: "module",
    });
    worker.onmessage = (e) => {
      const msg = e.data;
      if (msg.type === "status") {
        setProveStatus(msg.message);
      } else if (msg.type === "done") {
        worker.terminate();
        resolve(msg);
      } else if (msg.type === "error") {
        worker.terminate();
        reject(new Error(msg.message));
      }
    };
    worker.onerror = (err) => {
      worker.terminate();
      reject(new Error(err.message || "Proving worker failed"));
    };
    worker.postMessage({ input, circuitWasm, zkey }, [circuitWasm, zkey]);
  });
}

// Proves the last run in-browser (snarkjs Groth16 over the dash_zk circuit)
// and submits it to the contract, which recomputes the obstacle schedule
// from the seed and verifies the proof via `verify_groth16_bn254`.
async function proveAndSubmit() {
  if (!canSubmit() || model.submitting || model.submitted) return;
  const run = model.lastRun;
  if (!run?.sim) return;

  model.submitting = true;
  model.submitError = null;
  render();

  try {
    // Bind the caller's account into the proof's public inputs.
    const account = selectedAccount();
    const acctBytes = base58Decode(account);
    if (acctBytes.length !== 96) {
      throw new Error("Only public (Moonlight) accounts can submit runs.");
    }
    setProveStatus("Preparing witness input...");
    const input = run.sim.input_json(bytesToHex(acctBytes));

    const base = `${import.meta.env.BASE_URL}zk/`;
    const circuitWasm = await fetchArtifact(`${base}dash_zk.wasm`, "circuit");
    const zkey = await fetchArtifact(`${base}dash_zk_final.zkey`, "proving key");

    setProveStatus("Proving in browser (this may take a minute)...");
    const { arkProof, elapsedMs } = await proveInWorker(input, circuitWasm, zkey);

    setProveStatus(
      `Proved in ${(elapsedMs / 1000).toFixed(1)}s. Verifying on-chain...`
    );
    const tx = await dario.write.submit_zk_run(
      [Number(run.seed), Number(run.score), Number(run.ticks), Array.from(arkProof)],
      { privacy: "public", amount: "0", deposit: "0" }
    );
    const receipt = await tx.wait({ timeoutMs: 90_000 });
    if (!receipt.ok) {
      throw new Error(receipt.error || "Transaction failed");
    }

    model.submitted = true;
    await sync();
    await refreshLeaderboard();
  } catch (err) {
    model.submitError = err?.message
      ? `Submit failed: ${err.message}`
      : "Proof rejected or transaction failed.";
  } finally {
    model.submitting = false;
    model.proveStatus = null;
    render();
  }
}

function startRun() {
  if (!game) return;
  model.liveForm = STATE.Regular;
  model.lastRun = null;
  model.submitted = false;
  model.submitError = null;
  game.start();
  render();
}

async function requestConnect() {
  if (!wallet) return;
  try {
    if (typeof connectBtn?.open === "function") {
      connectBtn.open();
    } else if (connectBtn?.shadowRoot?.querySelector) {
      const inner = connectBtn.shadowRoot.querySelector("button");
      if (inner?.click) inner.click();
      else await wallet.connect();
    } else {
      await wallet.connect();
    }
  } catch {
    // User rejected.
  }
}

elPlayBtn?.addEventListener("click", startRun);
elAgainBtn?.addEventListener("click", () => {
  if (!model.submitting) startRun();
});
elDownloadRunBtn?.addEventListener("click", downloadRunFile);
elSubmitProofBtn?.addEventListener("click", proveAndSubmit);
elConnectStartBtn?.addEventListener("click", requestConnect);

window.addEventListener("keydown", (e) => {
  if (game?.running || model.submitting) return;
  if (e.code === "Space" || e.code === "Enter") {
    e.preventDefault();
    startRun();
  }
});

function onWalletState() {
  const account = selectedAccount();
  render();
  if (account && account !== lastAccount) {
    lastAccount = account;
    sync();
    refreshLeaderboard();
  }
}

async function init() {
  render();

  await initDashWasm();
  game = createGame({
    canvas: elCanvas,
    sprites,
    createSim: (seed) => new ZkDashSim(seed),
    onHud: ({ form }) => {
      if (form !== model.liveForm) {
        model.liveForm = form;
        render();
      }
    },
    onGameOver: (run) => {
      model.lastRun = run;
      model.submitted = false;
      model.submitError = null;
      render();
    },
  });
  render();

  if (!dusk || !wallet) return;

  wallet.subscribe(onWalletState);
  await wallet.ready();
  onWalletState();

  try {
    await dusk.driver(DRIVER_URL);
    model.driverReady = true;
  } catch {
    model.error = "Missing or incompatible data_driver.wasm.";
  }

  await sync();
  await refreshLeaderboard();
}

init();
