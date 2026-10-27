// Generates the built-in panda pet pack (SVG frames + manifest.json).
//
//   node tools/gen-panda.mjs
//
// The art is drawn from simple shapes so it can be tweaked here and
// regenerated. Swap in hand-drawn frames any time: the app only reads
// pets/panda/manifest.json and the files it lists.

import { mkdirSync, writeFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "pets", "panda");

const INK = "#2b2b2b";
const FUR = "#ffffff";
const BLUSH = "#ffb3c1";

/** Draw one 128x128 panda frame. Faces right when `fx` > 0. */
function panda({
  dy = 0, // whole-body vertical offset (bob / hop)
  fx = 0, // face shift, gives a slight head turn
  legs = "stand", // stand | walkA | walkB | sit | dangle
  arms = "down", // down | up | waveL | waveR | rest | dangleA | dangleB
  eyes = "open", // open | closed | happy | sad | wide | half
  mouth = "smile", // smile | open | frown | o | flat
  extras = [], // zzz1 | zzz2 | heart | tear1 | tear2 | bang | dots | sweat
  shadow = true,
  sitting = false,
} = {}) {
  const p = [];
  const bodyY = (sitting ? 96 : 90) + dy;
  const headY = (sitting ? 56 : 50) + dy;

  if (shadow) p.push(`<ellipse cx="64" cy="123" rx="${sitting ? 34 : 28}" ry="4" fill="#000" opacity=".15"/>`);

  // Legs
  const leg = (x, y, rx = 10, ry = 9, rot = 0) =>
    `<ellipse cx="${x}" cy="${y}" rx="${rx}" ry="${ry}" fill="${INK}" transform="rotate(${rot} ${x} ${y})"/>`;
  switch (legs) {
    case "stand": p.push(leg(48, 114 + dy, 10, 8), leg(80, 114 + dy, 10, 8)); break;
    case "walkA": p.push(leg(44, 113 + dy, 10, 8, -15), leg(82, 116 + dy, 10, 7, 10)); break;
    case "walkB": p.push(leg(46, 116 + dy, 10, 7, -10), leg(84, 113 + dy, 10, 8, 15)); break;
    case "sit": p.push(leg(40, 116 + dy, 13, 8, -10), leg(88, 116 + dy, 13, 8, 10)); break;
    case "dangle": p.push(leg(50, 118 + dy, 8, 11, 8), leg(78, 118 + dy, 8, 11, -8)); break;
  }

  // Body
  p.push(`<ellipse cx="64" cy="${bodyY}" rx="${sitting ? 32 : 29}" ry="${sitting ? 24 : 25}" fill="${FUR}" stroke="${INK}" stroke-width="2.5"/>`);
  p.push(`<ellipse cx="64" cy="${bodyY + 4}" rx="15" ry="13" fill="#f3f3f3"/>`);

  // Arms
  const arm = (x, y, rot) =>
    `<ellipse cx="${x}" cy="${y}" rx="8" ry="13" fill="${INK}" transform="rotate(${rot} ${x} ${y})"/>`;
  const by = bodyY;
  const armSets = {
    down: [arm(38, by, 20), arm(90, by, -20)],
    rest: [arm(42, by + 8, 50), arm(86, by + 8, -50)],
    up: [arm(32, by - 18, -35), arm(96, by - 18, 35)],
    waveL: [arm(32, by - 18, -35), arm(90, by, -20)],
    waveR: [arm(38, by, 20), arm(96, by - 18, 35)],
    dangleA: [arm(34, by - 20, -20), arm(94, by - 20, 20)],
    dangleB: [arm(36, by - 22, -10), arm(92, by - 22, 10)],
  };
  p.push(...armSets[arms]);

  // Ears, then head on top
  p.push(`<circle cx="${36 + fx * 0.5}" cy="${headY - 26}" r="12" fill="${INK}"/>`);
  p.push(`<circle cx="${92 + fx * 0.5}" cy="${headY - 26}" r="12" fill="${INK}"/>`);
  p.push(`<circle cx="64" cy="${headY}" r="34" fill="${FUR}" stroke="${INK}" stroke-width="2.5"/>`);

  // Face
  const lx = 50 + fx, rx = 78 + fx, ey = headY + 2;
  p.push(`<ellipse cx="${lx}" cy="${ey}" rx="9" ry="11" fill="${INK}" transform="rotate(25 ${lx} ${ey})"/>`);
  p.push(`<ellipse cx="${rx}" cy="${ey}" rx="9" ry="11" fill="${INK}" transform="rotate(-25 ${rx} ${ey})"/>`);
  const line = (d) => `<path d="${d}" fill="none" stroke="#fff" stroke-width="2.4" stroke-linecap="round"/>`;
  for (const x of [lx + 1, rx - 1]) {
    switch (eyes) {
      case "open":
        p.push(`<circle cx="${x}" cy="${ey - 1}" r="4" fill="#fff"/><circle cx="${x + 1 + fx * 0.25}" cy="${ey}" r="2.4" fill="${INK}"/>`);
        break;
      case "wide":
        p.push(`<circle cx="${x}" cy="${ey - 1}" r="5.5" fill="#fff"/><circle cx="${x}" cy="${ey - 1}" r="2" fill="${INK}"/>`);
        break;
      case "closed": p.push(line(`M${x - 4} ${ey} q4 3 8 0`)); break;
      case "happy": p.push(line(`M${x - 4} ${ey + 1} q4 -6 8 0`)); break;
      case "sad":
        p.push(`<circle cx="${x}" cy="${ey + 1}" r="3.6" fill="#fff"/><circle cx="${x}" cy="${ey + 2}" r="2.2" fill="${INK}"/>`);
        p.push(line(x < 64 ? `M${x - 5} ${ey - 5} l8 3` : `M${x + 5} ${ey - 5} l-8 3`));
        break;
      case "half": p.push(line(`M${x - 4} ${ey} h8`)); break;
    }
  }
  const nx = 64 + fx, ny = headY + 13;
  p.push(`<ellipse cx="${nx}" cy="${ny}" rx="4.5" ry="3.2" fill="${INK}"/>`);
  const m = (d, fill = "none") => `<path d="${d}" fill="${fill}" stroke="${INK}" stroke-width="2" stroke-linecap="round"/>`;
  switch (mouth) {
    case "smile": p.push(m(`M${nx - 6} ${ny + 5} q3 4 6 0 q3 4 6 0`)); break;
    case "open": p.push(m(`M${nx - 7} ${ny + 5} q7 11 14 0 z`, "#e8576b")); break;
    case "frown": p.push(m(`M${nx - 6} ${ny + 9} q6 -6 12 0`)); break;
    case "o": p.push(`<ellipse cx="${nx}" cy="${ny + 9}" rx="3.5" ry="4.5" fill="#e8576b" stroke="${INK}" stroke-width="2"/>`); break;
    case "flat": p.push(m(`M${nx - 5} ${ny + 7} h10`)); break;
  }
  if (eyes !== "sad") {
    p.push(`<circle cx="${40 + fx}" cy="${headY + 14}" r="5" fill="${BLUSH}" opacity=".75"/>`);
    p.push(`<circle cx="${88 + fx}" cy="${headY + 14}" r="5" fill="${BLUSH}" opacity=".75"/>`);
  }

  // Extras
  const text = (x, y, size, s, fill = INK) =>
    `<text x="${x}" y="${y}" font-family="Segoe UI, Arial, sans-serif" font-weight="700" font-size="${size}" fill="${fill}">${s}</text>`;
  for (const e of extras) {
    switch (e) {
      case "zzz1": p.push(text(104, 16, 12, "z"), text(114, 8, 9, "z")); break;
      case "zzz2": p.push(text(103, 18, 14, "z"), text(114, 9, 10, "z")); break;
      case "heart":
        p.push(`<path d="M108 22 c-4 -6 -12 -2 -8 4 l8 8 l8 -8 c4 -6 -4 -10 -8 -4z" fill="#ff5c7a"/>`);
        break;
      case "tear1": p.push(`<path d="M${lx - 2} ${ey + 9} q-3 5 0 7 q3 -2 0 -7z" fill="#7ec8ff"/>`); break;
      case "tear2": p.push(`<path d="M${lx - 2} ${ey + 15} q-3 5 0 7 q3 -2 0 -7z" fill="#7ec8ff"/>`); break;
      case "bang":
        p.push(`<circle cx="110" cy="18" r="11" fill="#ffcc33" stroke="${INK}" stroke-width="2"/>`);
        p.push(text(106.5, 24.5, 17, "!"));
        break;
      case "dots": p.push(text(102, 14, 16, "…")); break;
      case "sweat": p.push(`<path d="M100 ${headY - 16} q-4 6 0 9 q4 -3 0 -9z" fill="#7ec8ff"/>`); break;
    }
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">\n  ${p.join("\n  ")}\n</svg>\n`;
}

const ANIMATIONS = {
  idle: {
    fps: 3,
    frames: [{}, {}, { dy: 1 }, { dy: 1 }, {}, { eyes: "closed" }],
  },
  walk: {
    fps: 6,
    frames: [
      { fx: 4, legs: "walkA", dy: -1 },
      { fx: 4, legs: "stand" },
      { fx: 4, legs: "walkB", dy: -1 },
      { fx: 4, legs: "stand" },
    ],
  },
  sit: {
    fps: 2,
    frames: [{ sitting: true, legs: "sit", arms: "rest" }, { sitting: true, legs: "sit", arms: "rest" }, { sitting: true, legs: "sit", arms: "rest", eyes: "closed" }],
  },
  sleep: {
    fps: 1.5,
    frames: [
      { sitting: true, legs: "sit", arms: "rest", eyes: "closed", mouth: "flat", extras: ["zzz1"] },
      { sitting: true, legs: "sit", arms: "rest", eyes: "closed", mouth: "flat", extras: ["zzz2"], dy: 1 },
    ],
  },
  happy: {
    fps: 6,
    frames: [
      { eyes: "happy", mouth: "open", arms: "up", dy: -6, extras: ["heart"] },
      { eyes: "happy", mouth: "open", arms: "up", dy: -2, extras: ["heart"] },
      { eyes: "happy", mouth: "open", arms: "down" },
      { eyes: "happy", mouth: "smile", arms: "down" },
    ],
  },
  sad: {
    fps: 2,
    frames: [
      { eyes: "sad", mouth: "frown", arms: "down", extras: ["tear1"] },
      { eyes: "sad", mouth: "frown", arms: "down", extras: ["tear2"], dy: 1 },
    ],
  },
  bored: {
    fps: 1.5,
    frames: [
      { sitting: true, legs: "sit", arms: "rest", eyes: "half", mouth: "flat", extras: ["dots"] },
      { sitting: true, legs: "sit", arms: "rest", eyes: "half", mouth: "flat", dy: 1 },
    ],
  },
  surprised: {
    fps: 4,
    frames: [{ eyes: "wide", mouth: "o", arms: "up", extras: ["sweat"] }, { eyes: "wide", mouth: "o", arms: "up", dy: -2, extras: ["sweat"] }],
  },
  drag: {
    fps: 4,
    frames: [
      { eyes: "wide", mouth: "o", arms: "dangleA", legs: "dangle", shadow: false },
      { eyes: "wide", mouth: "o", arms: "dangleB", legs: "dangle", shadow: false, dy: 1 },
    ],
  },
  alert: {
    fps: 4,
    frames: [
      { eyes: "wide", mouth: "open", arms: "waveL", extras: ["bang"] },
      { eyes: "wide", mouth: "open", arms: "waveR", extras: ["bang"], dy: -2 },
    ],
  },
};

mkdirSync(OUT, { recursive: true });
for (const f of readdirSync(OUT)) if (f.endsWith(".svg")) rmSync(join(OUT, f));

const manifest = {
  id: "panda",
  name: "Panda",
  author: "DesktopPet (generated by tools/gen-panda.mjs)",
  license: "MIT",
  width: 128,
  height: 128,
  facing: "right",
  animations: {},
};

for (const [name, { fps, frames }] of Object.entries(ANIMATIONS)) {
  manifest.animations[name] = { fps, frames: [] };
  frames.forEach((opts, i) => {
    const file = `${name}-${i}.svg`;
    writeFileSync(join(OUT, file), panda(opts));
    manifest.animations[name].frames.push(file);
  });
}
writeFileSync(join(OUT, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`wrote ${Object.values(manifest.animations).reduce((n, a) => n + a.frames.length, 0)} frames to ${OUT}`);
