// Dario Dash — renderer + input recorder over the deterministic wasm sim
// (dash_zk compiled via dash_web). The browser runs the exact 30 Hz
// simulation the Groth16 circuit verifies, so a finished run can be proven
// entirely in-browser and the score submitted on-chain.

import { STATE } from "./fsm.js";

const W = 960;
const H = 540;
const GROUND_Y = 464;
const TICK = 1 / 30;

// snapshot record entity types (see dash_core::Sim::snapshot)
const ENT_OBSTACLE = 0;
const ENT_ITEM = 1;
const ENT_FIREBALL = 2;
// obstacle kinds
const KIND_BARREL = 0;
const KIND_PIPE = 1;
const KIND_BAT = 2;
// item kinds
const ITEMS = [
  { emoji: "\u2615", tint: "#f6d365" }, // espresso
  { emoji: "\ud83c\udf36\ufe0f", tint: "#ff6b3d" }, // chili
  { emoji: "\ud83e\udde3", tint: "#7de3ef" }, // cape
];

const THEMES = {
  [STATE.Regular]: { top: "#52d5e4", mid: "#9adf77", ground: "#7c441d", hill: "#5cbd45" },
  [STATE.Super]: { top: "#173b7b", mid: "#225fb2", ground: "#10192e", hill: "#1c2c52" },
  [STATE.Fire]: { top: "#612111", mid: "#c9461a", ground: "#13090a", hill: "#8a2e13" },
  [STATE.Cape]: { top: "#78ddeb", mid: "#aeeaf0", ground: "#244769", hill: "#6bc3d8" },
};

// Seeds are limited to 32 bits so they survive the JSON round-trip to the
// contract data-driver (JS numbers are only exact up to 2^53).
function randomSeed() {
  const bytes = new Uint8Array(4);
  crypto.getRandomValues(bytes);
  let seed = 0n;
  for (const b of bytes) seed = (seed << 8n) | BigInt(b);
  return seed;
}

export function createGame({ canvas, sprites, createSim, onGameOver, onHud }) {
  const ctx = canvas.getContext("2d");
  canvas.width = W;
  canvas.height = H;

  const game = {
    running: false,
    over: false,
    sim: null,
    seed: 0n,
    form: STATE.Regular,
    score: 0,
    best: Number(localStorage.getItem("dario-best") || 0),
    distance: 0,
    time: 0,
    holdJump: false,
    holdFire: false,
    // deltas for effects
    lastPickups: 0,
    lastKills: 0,
    lastInvuln: false,
    particles: [],
    clouds: [],
    shake: 0,
  };

  for (let i = 0; i < 5; i++) {
    game.clouds.push({
      x: Math.random() * W,
      y: 40 + Math.random() * 150,
      s: 0.5 + Math.random() * 0.8,
    });
  }

  let raf = 0;
  let last = 0;
  let acc = 0;

  function burst(x, y, color, n = 10) {
    for (let i = 0; i < n; i++) {
      const a = Math.random() * Math.PI * 2;
      const v = 90 + Math.random() * 220;
      game.particles.push({
        x,
        y,
        vx: Math.cos(a) * v,
        vy: Math.sin(a) * v - 80,
        life: 0.5 + Math.random() * 0.4,
        color,
      });
    }
  }

  function playerRect() {
    const [x, y, w, h] = game.sim.player();
    return { x, y, w, h };
  }

  function stepSim() {
    const input = (game.holdJump ? 1 : 0) | (game.holdFire ? 2 : 0);
    game.sim.tick(input);

    const p = playerRect();
    const form = Number(game.sim.form());
    if (form !== game.form && form !== STATE.GameOver) {
      burst(p.x + p.w / 2, p.y - p.h / 2, "#ffe082", 16);
    }
    const pickups = Number(game.sim.pickups());
    if (pickups > game.lastPickups) {
      burst(p.x + p.w + 20, p.y - p.h / 2, "#f6d365", 12);
    }
    const kills = Number(game.sim.kills());
    if (kills > game.lastKills) {
      burst(p.x + p.w + 120, p.y - p.h / 2, "#ffb347", 14);
    }
    const invuln = game.sim.invulnerable();
    if (invuln && !game.lastInvuln) {
      game.shake = 0.35;
      burst(p.x + 20, p.y - 40, "#ff5a5a", 14);
    }
    game.form = form === STATE.GameOver ? game.form : form;
    game.lastPickups = pickups;
    game.lastKills = kills;
    game.lastInvuln = invuln;
    game.score = Number(game.sim.score());
    game.distance = Number(game.sim.distance_px());

    if (game.sim.over()) endRun();
  }

  function endRun() {
    game.over = true;
    game.running = false;
    game.best = Math.max(game.best, game.score);
    localStorage.setItem("dario-best", String(game.best));
    cancelAnimationFrame(raf);
    game.shake = 0.4;
    draw();
    onGameOver?.({
      score: game.score,
      best: game.best,
      ticks: Number(game.sim.ticks()),
      pickups: game.lastPickups,
      kills: game.lastKills,
      distance: game.distance,
      seed: game.seed,
      trace: game.sim.trace(),
      sim: game.sim,
    });
  }

  function drawBackground(theme) {
    const sky = ctx.createLinearGradient(0, 0, 0, H);
    sky.addColorStop(0, theme.top);
    sky.addColorStop(0.72, theme.mid);
    sky.addColorStop(0.73, theme.ground);
    sky.addColorStop(1, "#1c0e06");
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, W, H);

    // Hills
    ctx.fillStyle = theme.hill;
    const off = -(game.distance * 0.4) % 480;
    for (let x = off - 480; x < W + 480; x += 480) {
      ctx.beginPath();
      ctx.ellipse(x + 240, GROUND_Y + 10, 260, 90, 0, Math.PI, 0);
      ctx.fill();
    }

    // Clouds
    ctx.fillStyle = "rgba(255,255,255,0.85)";
    for (const c of game.clouds) {
      ctx.beginPath();
      ctx.ellipse(c.x, c.y, 46 * c.s, 18 * c.s, 0, 0, Math.PI * 2);
      ctx.ellipse(c.x + 30 * c.s, c.y + 6 * c.s, 34 * c.s, 14 * c.s, 0, 0, Math.PI * 2);
      ctx.fill();
    }

    // Ground line + dashes
    ctx.fillStyle = "rgba(0,0,0,0.35)";
    ctx.fillRect(0, GROUND_Y, W, 4);
    ctx.fillStyle = "rgba(255,255,255,0.28)";
    const dashOff = -(game.distance % 64);
    for (let x = dashOff; x < W; x += 64) {
      ctx.fillRect(x, GROUND_Y + 26, 30, 5);
    }
  }

  function drawPlayer() {
    const p = playerRect();
    if (game.sim.invulnerable() && Math.floor(game.time * 12) % 2 === 0 && !game.over) return;

    const img = sprites[game.over ? STATE.GameOver : game.form];
    const drawH = p.h;
    const drawW = img?.naturalWidth
      ? drawH * (img.naturalWidth / img.naturalHeight)
      : p.w;

    const squash = game.sim.grounded() ? 1 + Math.sin(game.time * 18) * 0.02 : 1;
    ctx.save();
    ctx.translate(p.x + p.w / 2, p.y);
    ctx.scale(1, squash);
    if (img?.complete && img.naturalWidth) {
      ctx.drawImage(img, -drawW / 2, -drawH, drawW, drawH);
    } else {
      ctx.fillStyle = "#e14434";
      ctx.fillRect(-p.w / 2, -p.h, p.w, p.h);
    }
    ctx.restore();
  }

  function drawObstacle(kind, ox, oy, ow, oh) {
    if (kind === KIND_BAT) {
      ctx.save();
      ctx.translate(ox + ow / 2, oy + oh / 2);
      ctx.fillStyle = "#3a2a4d";
      ctx.beginPath();
      ctx.ellipse(0, 0, ow / 2, oh / 2.4, 0, 0, Math.PI * 2);
      ctx.fill();
      const flap = Math.sin(game.time * 14) * 10;
      ctx.fillStyle = "#54407a";
      ctx.beginPath();
      ctx.moveTo(-ow / 2, 0);
      ctx.lineTo(-ow, -8 - flap);
      ctx.lineTo(-ow / 2, 6);
      ctx.moveTo(ow / 2, 0);
      ctx.lineTo(ow, -8 - flap);
      ctx.lineTo(ow / 2, 6);
      ctx.fill();
      ctx.fillStyle = "#ffdf5d";
      ctx.fillRect(-8, -6, 5, 5);
      ctx.fillRect(3, -6, 5, 5);
      ctx.restore();
      return;
    }

    const x = ox;
    const y = oy - oh; // barrel/pipe are anchored to the ground line
    if (kind === KIND_PIPE) {
      ctx.fillStyle = "#1e8f3e";
      ctx.fillRect(x + 4, y + 14, ow - 8, oh - 14);
      ctx.fillStyle = "#27b04e";
      ctx.fillRect(x, y, ow, 18);
      ctx.fillStyle = "rgba(255,255,255,0.25)";
      ctx.fillRect(x + 8, y + 18, 7, oh - 20);
    } else {
      ctx.fillStyle = "#8a5a2b";
      ctx.fillRect(x, y, ow, oh);
      ctx.fillStyle = "#a06c35";
      ctx.fillRect(x + 3, y + 3, ow - 6, oh - 6);
      ctx.strokeStyle = "#5f3d1c";
      ctx.lineWidth = 2;
      ctx.strokeRect(x + 3, y + 3, ow - 6, oh - 6);
      ctx.beginPath();
      ctx.moveTo(x + 3, y + 3);
      ctx.lineTo(x + ow - 3, y + oh - 3);
      ctx.moveTo(x + ow - 3, y + 3);
      ctx.lineTo(x + 3, y + oh - 3);
      ctx.stroke();
    }
  }

  function draw() {
    const theme = THEMES[game.over ? STATE.Regular : game.form] || THEMES[STATE.Regular];
    ctx.save();
    if (game.shake > 0) {
      ctx.translate((Math.random() - 0.5) * 10, (Math.random() - 0.5) * 8);
    }

    drawBackground(theme);

    if (game.sim) {
      const snap = game.sim.snapshot();

      // Items first (behind obstacles), then obstacles, then fireballs.
      for (let i = 0; i < snap.length; i += 6) {
        if (snap[i] !== ENT_ITEM) continue;
        const kind = snap[i + 1];
        const [x, y, w, h] = [snap[i + 2], snap[i + 3], snap[i + 4], snap[i + 5]];
        const meta = ITEMS[kind] ?? ITEMS[0];
        const bob = Math.sin(game.time * 4 + x * 0.02) * 6;
        ctx.save();
        ctx.translate(x + w / 2, y + h / 2 + bob);
        ctx.fillStyle = "rgba(255,255,255,0.18)";
        ctx.beginPath();
        ctx.arc(0, 0, 24, 0, Math.PI * 2);
        ctx.fill();
        ctx.font = "28px serif";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(meta.emoji, 0, 2);
        ctx.restore();
      }

      for (let i = 0; i < snap.length; i += 6) {
        if (snap[i] !== ENT_OBSTACLE) continue;
        drawObstacle(snap[i + 1], snap[i + 2], snap[i + 3], snap[i + 4], snap[i + 5]);
      }

      for (let i = 0; i < snap.length; i += 6) {
        if (snap[i] !== ENT_FIREBALL) continue;
        const [x, y, w, h] = [snap[i + 2], snap[i + 3], snap[i + 4], snap[i + 5]];
        ctx.fillStyle = "#ff8c1a";
        ctx.beginPath();
        ctx.arc(x + w / 2, y + h / 2, 9, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = "#ffd23d";
        ctx.beginPath();
        ctx.arc(x + w / 2, y + h / 2, 5, 0, Math.PI * 2);
        ctx.fill();
      }

      drawPlayer();
    }

    for (const pt of game.particles) {
      ctx.globalAlpha = Math.max(0, pt.life * 2);
      ctx.fillStyle = pt.color;
      ctx.fillRect(pt.x - 3, pt.y - 3, 6, 6);
      ctx.globalAlpha = 1;
    }

    // Score
    ctx.font = "700 26px 'Courier New', monospace";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    ctx.fillStyle = "rgba(0,0,0,0.45)";
    ctx.fillText(`SCORE ${String(game.score).padStart(6, "0")}`, 22, 20);
    ctx.fillStyle = "#fff";
    ctx.fillText(`SCORE ${String(game.score).padStart(6, "0")}`, 20, 18);
    ctx.textAlign = "right";
    ctx.fillStyle = "rgba(0,0,0,0.45)";
    ctx.fillText(`BEST ${String(game.best).padStart(6, "0")}`, W - 18, 20);
    ctx.fillStyle = "#ffe082";
    ctx.fillText(`BEST ${String(game.best).padStart(6, "0")}`, W - 20, 18);

    if (game.form === STATE.Fire && !game.over) {
      ctx.textAlign = "left";
      ctx.font = "700 16px 'Courier New', monospace";
      ctx.fillStyle = "#ffd23d";
      ctx.fillText("F / tap right side: FIREBALL", 20, 52);
    }

    ctx.restore();
  }

  function updateEffects(dt) {
    game.time += dt;
    game.shake = Math.max(0, game.shake - dt);
    const dx = (game.sim ? 5.5 : 1) * 60 * dt;
    for (const c of game.clouds) {
      c.x -= dx * 0.25 * c.s;
      if (c.x < -120) c.x = W + 60;
    }
    for (const pt of game.particles) {
      pt.x += pt.vx * dt;
      pt.y += pt.vy * dt;
      pt.vy += 900 * dt;
      pt.life -= dt;
    }
    game.particles = game.particles.filter((pt) => pt.life > 0);
  }

  function loop(ts) {
    if (!game.running) return;
    const dt = Math.min(0.1, (ts - last) / 1000 || TICK);
    last = ts;

    // Fixed-timestep sim; rendering runs per frame.
    acc += dt;
    while (acc >= TICK && !game.over) {
      stepSim();
      acc -= TICK;
    }
    updateEffects(dt);

    onHud?.({ score: game.score, form: game.form });

    if (!game.over) {
      draw();
      raf = requestAnimationFrame(loop);
    }
  }

  function start() {
    game.seed = randomSeed();
    game.sim = createSim(game.seed);
    game.running = true;
    game.over = false;
    game.form = STATE.Regular;
    game.score = 0;
    game.distance = 0;
    game.time = 0;
    game.lastPickups = 0;
    game.lastKills = 0;
    game.lastInvuln = false;
    game.particles = [];
    game.holdJump = false;
    game.holdFire = false;
    last = performance.now();
    acc = 0;
    raf = requestAnimationFrame(loop);
  }

  function stop() {
    game.running = false;
    cancelAnimationFrame(raf);
  }

  // Input
  window.addEventListener("keydown", (e) => {
    if (!game.running) return;
    if (e.code === "Space" || e.code === "ArrowUp" || e.code === "KeyW") {
      e.preventDefault();
      game.holdJump = true;
    }
    if (e.code === "KeyF" || e.code === "KeyX") {
      e.preventDefault();
      game.holdFire = true;
    }
  });
  window.addEventListener("keyup", (e) => {
    if (e.code === "Space" || e.code === "ArrowUp" || e.code === "KeyW") {
      game.holdJump = false;
    }
    if (e.code === "KeyF" || e.code === "KeyX") {
      game.holdFire = false;
    }
  });
  canvas.addEventListener("pointerdown", (e) => {
    if (!game.running) return;
    const rect = canvas.getBoundingClientRect();
    const rightSide = (e.clientX - rect.left) / rect.width > 0.6;
    if (rightSide && game.form === STATE.Fire) {
      game.holdFire = true;
    } else {
      game.holdJump = true;
    }
  });
  window.addEventListener("pointerup", () => {
    game.holdJump = false;
    game.holdFire = false;
  });

  draw();

  return {
    start,
    stop,
    get running() {
      return game.running;
    },
    get over() {
      return game.over;
    },
    get best() {
      return game.best;
    },
  };
}
