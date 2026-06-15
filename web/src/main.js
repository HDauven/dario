import { createDuskApp, DUSK_CHAIN_PRESETS } from "@dusk/connect";
import { defineDuskConnectButton } from "@dusk/connect/ui";
import "./styles.css";

import regularSprite from "./assets/dario-regular-new.png";
import superSprite from "./assets/dario-super-new.png";
import fireSprite from "./assets/dario-fire-new.png";
import capeSprite from "./assets/dario-cape-new.png";
import gameOverSprite from "./assets/dario-gameover-new.png";

defineDuskConnectButton();

const CONTRACT_ID = import.meta.env.VITE_DARIO_CONTRACT_ID || "";
const NODE_URL =
  import.meta.env.VITE_DUSK_NODE_URL || "https://testnet.nodes.dusk.network";
const DRIVER_URL = `${import.meta.env.BASE_URL}data_driver.wasm?v=${Date.now()}`;
const HAS_CONTRACT = /^0x[0-9a-fA-F]{64}$/.test(CONTRACT_ID);

const STATE_META = [
  { name: "Regular", sprite: regularSprite },
  { name: "Super", sprite: superSprite },
  { name: "Fire", sprite: fireSprite },
  { name: "Cape", sprite: capeSprite },
  { name: "Game Over", sprite: gameOverSprite },
];

const ACTION_META = {
  0: { label: "Espresso", emoji: "☕" },
  1: { label: "Chili", emoji: "🌶️" },
  2: { label: "Cape", emoji: "🧣" },
  3: { label: "Damage", emoji: "💥" },
  4: { label: "Revive", emoji: "💙" },
};

const dusk = HAS_CONTRACT
  ? createDuskApp({
      nodeUrl: NODE_URL,
      chain: { chainId: DUSK_CHAIN_PRESETS.testnet },
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

const elSprite = $("darioSprite");
const elStage = document.querySelector(".stage");
const elCabinet = document.querySelector(".cabinet");
const elHudState = $("hudState");
const elHudRevives = $("hudRevives");
const elHudAccount = $("hudAccount");
const elHint = $("hint");
const elDeadOverlay = $("deadOverlay");
const elStartOverlay = $("startOverlay");
const elStartText = $("startText");
const elStartBtn = $("startBtn");
const elPendingOverlay = $("pendingOverlay");
const elPendingText = $("pendingText");
const elActions = $("actions");
const elReviveBig = $("reviveBig");

const model = {
  state: null,
  revives: null,
  pending: false,
  pendingPhase: "",
  lastAction: null,
  ready: false,
  error: null,
};

let lastAccount = "";
let syncPromise = null;
let syncFailStreak = 0;

function selectedAccount() {
  return wallet?.state.selectedProfile?.account || wallet?.state.accounts?.[0] || "";
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

function isDead(state = model.state) {
  return Number(state) === 4;
}

function metaForState(state) {
  const i = Number(state);
  return Number.isFinite(i) && STATE_META[i] ? STATE_META[i] : STATE_META[0];
}

function setText(node, text) {
  if (!node) return;
  const value = node.querySelector?.(".hudValue");
  if (value) value.textContent = text;
  else node.textContent = text;
}

function render() {
  const account = selectedAccount();
  const isConnected = connected();
  const dead = isDead();

  if (elStage) elStage.dataset.state = String(model.state ?? 0);
  if (elCabinet) elCabinet.dataset.state = String(model.state ?? 0);

  const meta = metaForState(model.state);
  if (elSprite && elSprite.getAttribute("src") !== meta.sprite) {
    elSprite.style.opacity = "0";
    window.setTimeout(() => {
      elSprite.setAttribute("src", meta.sprite);
      elSprite.style.opacity = "1";
    }, 120);
  }

  setText(elHudState, model.state == null ? "-" : meta.name);
  setText(elHudRevives, model.revives == null ? "Revives -" : `Revives ${model.revives}`);
  setText(elHudAccount, shorten(account));

  if (elPendingOverlay) elPendingOverlay.hidden = !model.pending;
  if (elStartOverlay) {
    elStartOverlay.hidden = (HAS_CONTRACT && isConnected) || model.pending;
  }
  if (elDeadOverlay) elDeadOverlay.hidden = !isConnected || model.pending || !dead;

  if (elStartText) {
    elStartText.textContent = HAS_CONTRACT
      ? "Connect Wallet to play on Testnet."
      : "Set VITE_DARIO_CONTRACT_ID and rebuild.";
  }
  if (elStartBtn) {
    elStartBtn.disabled = !HAS_CONTRACT || !wallet;
    elStartBtn.textContent = HAS_CONTRACT ? "Connect Wallet" : "Contract ID Missing";
  }

  if (elPendingText) {
    if (!model.pending) {
      elPendingText.textContent = "Waiting for finalization...";
    } else {
      const action = ACTION_META[model.lastAction] || { label: "Move", emoji: "" };
      if (model.pendingPhase === "sign") {
        elPendingText.textContent = `Confirm ${action.emoji} ${action.label} in your wallet...`;
      } else if (model.pendingPhase === "submitted") {
        elPendingText.textContent = `Submitted ${action.emoji} ${action.label}. Waiting for execution...`;
      } else {
        elPendingText.textContent = `Finalizing ${action.emoji} ${action.label} on-chain...`;
      }
    }
  }

  const buttons = elActions ? elActions.querySelectorAll("[data-event]") : [];
  for (const button of buttons) {
    const event = Number(button.getAttribute("data-event"));
    const show = dead ? event === 4 : event !== 4;
    button.hidden = !show;
    button.disabled = !HAS_CONTRACT || !model.ready || !isConnected || model.pending;
  }

  if (elReviveBig) {
    elReviveBig.disabled = !HAS_CONTRACT || !model.ready || !isConnected || model.pending;
  }

  if (!elHint) return;
  if (!HAS_CONTRACT) {
    elHint.textContent = "Set VITE_DARIO_CONTRACT_ID and rebuild.";
  } else if (model.error) {
    elHint.textContent = model.error;
  } else if (!isConnected) {
    elHint.textContent = "Connect your wallet to play.";
  } else if (!model.ready) {
    elHint.textContent = "Loading data-driver.";
  } else if (model.pending) {
    elHint.textContent =
      model.pendingPhase === "sign"
        ? "Confirm the transaction in your wallet..."
        : "Waiting for on-chain execution...";
  } else if (dead) {
    elHint.textContent = "Revive Dario to continue.";
  } else {
    elHint.textContent = "Choose an action.";
  }
}

async function sync() {
  if (!dario || !connected()) {
    model.state = null;
    model.revives = null;
    render();
    return;
  }

  if (syncPromise) return syncPromise;

  syncPromise = (async () => {
    try {
      const account = selectedAccount();
      const before = model.state;
      const [state, revives] = await Promise.all([
        dario.call.current_state_for(account),
        dario.call.revive_count_for(account),
      ]);

      model.state = Number(state);
      model.revives = Number(revives);
      syncFailStreak = 0;
      model.error = null;

      if (before != null && model.state !== before) {
        pulseFx(before, model.state);
      }
    } catch {
      syncFailStreak++;
      if (model.state == null || syncFailStreak >= 3) {
        model.error = "Unable to sync on-chain state.";
      } else if (syncFailStreak >= 2) {
        model.error = "Network hiccup... retrying.";
      } else {
        model.error = null;
      }
    } finally {
      render();
    }
  })();

  try {
    await syncPromise;
  } finally {
    syncPromise = null;
  }
}

function pulseFx(prev, next) {
  if (!elStage) return;
  let kind = "";

  if (Number(next) === 4) kind = "hit";
  else if (Number(prev) === 4 && Number(next) === 0) kind = "revive";
  else if (Number(next) === 1) kind = "spark";
  else if (Number(next) === 2) kind = "ember";
  else if (Number(next) === 3) kind = "wind";

  if (!kind) return;

  elStage.dataset.fx = kind;
  window.setTimeout(() => {
    if (elStage.dataset.fx === kind) delete elStage.dataset.fx;
  }, 750);
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

async function connectWallet() {
  if (!wallet) return;
  try {
    await wallet.connect();
  } catch {
    // User rejected.
  }
}

async function sendEvent(event) {
  if (!dario || model.pending || !HAS_CONTRACT) return;

  model.lastAction = event;

  if (!connected()) {
    await connectWallet();
    if (!connected()) return;
  }

  model.pending = true;
  model.pendingPhase = "sign";
  model.error = null;
  render();

  try {
    const tx = await dario.write.handle_event(event, {
      privacy: "public",
      amount: "0",
      deposit: "0",
    });

    const unsubscribe = tx.onStatus((update) => {
      if (update.status === "submitted") model.pendingPhase = "submitted";
      if (update.status === "executing") model.pendingPhase = "executing";
      if ((update.status === "failed" || update.status === "timeout") && update.receipt?.error) {
        model.error = update.receipt.error;
      }
      render();
    });

    let receipt;
    try {
      receipt = await tx.wait({ timeoutMs: 60_000 });
    } finally {
      unsubscribe();
    }

    if (!receipt.ok && receipt.error) model.error = receipt.error;
    await sync();

    if (receipt.status === "timeout") {
      model.error = "Still processing... it may take a bit longer.";
      render();
    }
  } catch {
    model.error = "Transaction rejected or failed.";
  } finally {
    model.pending = false;
    model.pendingPhase = "";
    render();
  }
}

elActions?.addEventListener("click", (event) => {
  const button = event.target?.closest?.("[data-event]");
  if (!button) return;
  const move = Number(button.getAttribute("data-event"));
  if (Number.isFinite(move)) sendEvent(move);
});

elReviveBig?.addEventListener("click", () => sendEvent(4));
elStartBtn?.addEventListener("click", requestConnect);

function onWalletState() {
  const account = selectedAccount();
  render();
  if (account && account !== lastAccount) {
    lastAccount = account;
    sync();
  }
}

async function init() {
  render();

  if (!dusk || !wallet) return;

  wallet.subscribe(onWalletState);
  await wallet.ready();
  onWalletState();

  try {
    await dusk.driver(DRIVER_URL);
    model.ready = true;
  } catch {
    model.error = "Missing or incompatible data_driver.wasm.";
  }

  await sync();

  window.setInterval(() => {
    if (!model.pending) sync();
  }, 8000);
}

init();
