// Generates the built-in pet packs (SVG frames + manifest.json):
//
//   node tools/gen-pets.mjs
//
// Both pets share one skeleton (body, head, limbs, face, props) and differ in
// colours and markings, so every animation exists for both. Tweak the shapes
// here and regenerate. Hand-drawn frames can replace these any time: the app
// only reads pets/<id>/manifest.json and the files it lists.

import { mkdirSync, writeFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const PETS = join(dirname(fileURLToPath(import.meta.url)), "..", "pets");
const INK = "#2b2b2b";

const SPECIES = {
  panda: {
    name: "Panda",
    fur: "#ffffff",
    belly: "#f3f3f3",
    limbs: INK,
    ears: "round",
    earColor: INK,
    eyePatches: true,
    nose: INK,
    blush: "#ffb3c1",
    tail: null,
    food: "bamboo",
  },
  cat: {
    name: "Ginger Cat",
    fur: "#f4a259",
    belly: "#fde7cf",
    limbs: "#e58e3f",
    ears: "pointy",
    earColor: "#f4a259",
    eyePatches: false,
    nose: "#e8576b",
    blush: "#ff9aa8",
    tail: "#e58e3f",
    food: "fish",
  },
};

/** Draw one 128x128 frame. Faces right when `fx` > 0. */
function pet(sp, {
  dy = 0, // whole-body vertical offset (bob / hop)
  fx = 0, // face shift, gives a slight head turn
  legs = "stand", // stand | walkA | walkB | sit | dangle
  arms = "down", // down | up | waveL | waveR | rest | dangleA | dangleB | hold | fan
  eyes = "open", // open | closed | happy | sad | wide | half | droopy
  mouth = "smile", // smile | open | frown | o | flat | chew | wavy
  extras = [], // see the switch at the bottom
  shadow = true,
  sitting = false,
  tailSwing = 0, // degrees
} = {}) {
  const p = [];
  const bodyY = (sitting ? 96 : 90) + dy;
  const headY = (sitting ? 56 : 50) + dy;
  const limb = (x, y, rx, ry, rot) =>
    `<ellipse cx="${x}" cy="${y}" rx="${rx}" ry="${ry}" fill="${sp.limbs}" ${sp.limbs === INK ? "" : `stroke="${INK}" stroke-width="2"`} transform="rotate(${rot} ${x} ${y})"/>`;

  if (shadow) p.push(`<ellipse cx="64" cy="123" rx="${sitting ? 34 : 28}" ry="4" fill="#000" opacity=".15"/>`);

  // Tail (behind everything)
  if (sp.tail) {
    const ty = bodyY + 10;
    p.push(
      `<path d="M86 ${ty} q22 -4 20 -26 q-1 -10 6 -14" fill="none" stroke="${INK}" stroke-width="11" stroke-linecap="round" transform="rotate(${tailSwing} 86 ${ty})"/>`,
      `<path d="M86 ${ty} q22 -4 20 -26 q-1 -10 6 -14" fill="none" stroke="${sp.tail}" stroke-width="7" stroke-linecap="round" transform="rotate(${tailSwing} 86 ${ty})"/>`,
    );
  }

  // Legs
  switch (legs) {
    case "stand": p.push(limb(48, 114 + dy, 10, 8, 0), limb(80, 114 + dy, 10, 8, 0)); break;
    case "walkA": p.push(limb(44, 113 + dy, 10, 8, -15), limb(82, 116 + dy, 10, 7, 10)); break;
    case "walkB": p.push(limb(46, 116 + dy, 10, 7, -10), limb(84, 113 + dy, 10, 8, 15)); break;
    case "sit": p.push(limb(40, 116 + dy, 13, 8, -10), limb(88, 116 + dy, 13, 8, 10)); break;
    case "dangle": p.push(limb(50, 118 + dy, 8, 11, 8), limb(78, 118 + dy, 8, 11, -8)); break;
  }

  // Body
  p.push(`<ellipse cx="64" cy="${bodyY}" rx="${sitting ? 32 : 29}" ry="${sitting ? 24 : 25}" fill="${sp.fur}" stroke="${INK}" stroke-width="2.5"/>`);
  p.push(`<ellipse cx="64" cy="${bodyY + 4}" rx="15" ry="13" fill="${sp.belly}"/>`);

  // Arms
  const by = bodyY;
  const arm = (x, y, rot) => limb(x, y, 8, 13, rot);
  const armSets = {
    down: [arm(38, by, 20), arm(90, by, -20)],
    rest: [arm(42, by + 8, 50), arm(86, by + 8, -50)],
    up: [arm(32, by - 18, -35), arm(96, by - 18, 35)],
    waveL: [arm(32, by - 18, -35), arm(90, by, -20)],
    waveR: [arm(38, by, 20), arm(96, by - 18, 35)],
    dangleA: [arm(34, by - 20, -20), arm(94, by - 20, 20)],
    dangleB: [arm(36, by - 22, -10), arm(92, by - 22, 10)],
    hold: [arm(50, by - 8, 45), arm(78, by - 8, -45)],
    fan: [arm(38, by, 20), arm(98, by - 26, 60)],
  };
  const armsSvg = armSets[arms];

  // Ears, then head on top
  const ex = (x) => x + fx * 0.5;
  if (sp.ears === "round") {
    p.push(`<circle cx="${ex(36)}" cy="${headY - 26}" r="12" fill="${sp.earColor}"/>`);
    p.push(`<circle cx="${ex(92)}" cy="${headY - 26}" r="12" fill="${sp.earColor}"/>`);
  } else {
    const ear = (x, flip) => {
      const s = flip ? -1 : 1;
      return `<path d="M${ex(x) - 13 * s} ${headY - 18} L${ex(x) - 6 * s} ${headY - 44} L${ex(x) + 12 * s} ${headY - 26} Z" fill="${sp.earColor}" stroke="${INK}" stroke-width="2.5" stroke-linejoin="round"/>` +
        `<path d="M${ex(x) - 8 * s} ${headY - 22} L${ex(x) - 5 * s} ${headY - 37} L${ex(x) + 5 * s} ${headY - 27} Z" fill="#ffc0cb"/>`;
    };
    p.push(ear(38, false), ear(90, true));
  }
  p.push(`<circle cx="64" cy="${headY}" r="34" fill="${sp.fur}" stroke="${INK}" stroke-width="2.5"/>`);
  if (sp.name === "Ginger Cat") {
    // Forehead stripes and a pale muzzle.
    for (const [x, len] of [[56, 9], [64, 12], [72, 9]]) {
      p.push(`<path d="M${x + fx * 0.6} ${headY - 33} v${len}" stroke="#c96f22" stroke-width="3.5" stroke-linecap="round"/>`);
    }
    p.push(`<ellipse cx="${64 + fx}" cy="${headY + 15}" rx="15" ry="10" fill="${sp.belly}"/>`);
  }

  // Face
  const lx = 50 + fx, rx = 78 + fx, ey = headY + 2;
  const eyeInk = sp.eyePatches ? "#fff" : INK;
  if (sp.eyePatches) {
    p.push(`<ellipse cx="${lx}" cy="${ey}" rx="9" ry="11" fill="${INK}" transform="rotate(25 ${lx} ${ey})"/>`);
    p.push(`<ellipse cx="${rx}" cy="${ey}" rx="9" ry="11" fill="${INK}" transform="rotate(-25 ${rx} ${ey})"/>`);
  }
  const line = (d) => `<path d="${d}" fill="none" stroke="${eyeInk}" stroke-width="2.4" stroke-linecap="round"/>`;
  const white = sp.eyePatches ? "#fff" : "#fff";
  for (const x of [lx + 1, rx - 1]) {
    const ring = sp.eyePatches ? "" : ` stroke="${INK}" stroke-width="1.5"`;
    switch (eyes) {
      case "open":
        p.push(`<circle cx="${x}" cy="${ey - 1}" r="4.4" fill="${white}"${ring}/><circle cx="${x + 1 + fx * 0.25}" cy="${ey}" r="2.6" fill="${INK}"/>`);
        break;
      case "wide":
        p.push(`<circle cx="${x}" cy="${ey - 1}" r="5.8" fill="${white}"${ring}/><circle cx="${x}" cy="${ey - 1}" r="2" fill="${INK}"/>`);
        break;
      case "closed": p.push(line(`M${x - 4} ${ey} q4 3 8 0`)); break;
      case "happy": p.push(line(`M${x - 4} ${ey + 1} q4 -6 8 0`)); break;
      case "sad":
        p.push(`<circle cx="${x}" cy="${ey + 1}" r="3.8" fill="${white}"${ring}/><circle cx="${x}" cy="${ey + 2}" r="2.2" fill="${INK}"/>`);
        p.push(line(x < 64 ? `M${x - 5} ${ey - 5} l8 3` : `M${x + 5} ${ey - 5} l-8 3`));
        break;
      case "half": p.push(line(`M${x - 4} ${ey} h8`)); break;
      case "droopy":
        p.push(`<circle cx="${x}" cy="${ey + 1}" r="3.8" fill="${white}"${ring}/><circle cx="${x}" cy="${ey + 2}" r="2.2" fill="${INK}"/>`);
        p.push(`<path d="M${x - 5} ${ey - 1} h10" stroke="${sp.eyePatches ? INK : sp.fur}" stroke-width="4"/>`);
        p.push(line(`M${x - 5} ${ey + 1} h10`));
        break;
    }
  }
  const nx = 64 + fx, ny = headY + 13;
  p.push(sp.eyePatches
    ? `<ellipse cx="${nx}" cy="${ny}" rx="4.5" ry="3.2" fill="${sp.nose}"/>`
    : `<path d="M${nx - 4} ${ny - 2} h8 l-4 4 z" fill="${sp.nose}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>`);
  const m = (d, fill = "none") => `<path d="${d}" fill="${fill}" stroke="${INK}" stroke-width="2" stroke-linecap="round"/>`;
  switch (mouth) {
    case "smile": p.push(m(`M${nx - 6} ${ny + 5} q3 4 6 0 q3 4 6 0`)); break;
    case "open": p.push(m(`M${nx - 7} ${ny + 5} q7 11 14 0 z`, "#e8576b")); break;
    case "frown": p.push(m(`M${nx - 6} ${ny + 9} q6 -6 12 0`)); break;
    case "o": p.push(`<ellipse cx="${nx}" cy="${ny + 9}" rx="3.5" ry="4.5" fill="#e8576b" stroke="${INK}" stroke-width="2"/>`); break;
    case "flat": p.push(m(`M${nx - 5} ${ny + 7} h10`)); break;
    case "chew": p.push(m(`M${nx - 5} ${ny + 6} q5 5 10 0 q-5 2 -10 0z`, "#e8576b")); break;
    case "wavy": p.push(m(`M${nx - 7} ${ny + 8} q2 -3 4 0 q2 3 4 0 q2 -3 4 0`)); break;
  }
  if (!sp.eyePatches) {
    // Whiskers.
    for (const s of [-1, 1]) {
      for (const dyw of [-2, 3]) {
        p.push(`<path d="M${nx + s * 12} ${ny + 4 + dyw} l${s * 16} ${dyw * 1.5 - 1}" stroke="${INK}" stroke-width="1.3" stroke-linecap="round"/>`);
      }
    }
  }
  const flush = extras.includes("flush");
  if (eyes !== "sad" && eyes !== "droopy") {
    for (const cx of [40 + fx, 88 + fx]) {
      p.push(`<ellipse cx="${cx}" cy="${headY + 14}" rx="${flush ? 7 : 5}" ry="${flush ? 5 : 5}" fill="${flush ? "#ff6b6b" : sp.blush}" opacity="${flush ? ".8" : ".75"}"/>`);
    }
  }

  // Props held in the arms go between the head and the arms.
  const text = (x, y, size, s, fill = INK) =>
    `<text x="${x}" y="${y}" font-family="Segoe UI, Arial, sans-serif" font-weight="700" font-size="${size}" fill="${fill}">${s}</text>`;
  const props = [];
  const over = [];
  for (const e of extras) {
    switch (e) {
      case "food1":
      case "food2": {
        const bite = e === "food2";
        if (sp.food === "bamboo") {
          const top = bite ? 66 : 60;
          props.push(`<g transform="rotate(-28 64 ${by})"><rect x="60" y="${top + dy}" width="8" height="${by + 10 - top}" rx="2" fill="#8bc34a" stroke="#33691e" stroke-width="1.5"/><path d="M60 ${by - 8} h8 M60 ${by + 2} h8" stroke="#33691e" stroke-width="1.5"/><path d="M68 ${by - 4} q12 -6 16 2 q-10 2 -16 -2z" fill="#7cb342" stroke="#33691e" stroke-width="1"/></g>`);
        } else {
          const len = bite ? 20 : 28;
          props.push(`<g transform="rotate(-20 64 ${by - 6})"><ellipse cx="${64 + (28 - len) / 2}" cy="${by - 6}" rx="${len / 2}" ry="6" fill="#7fb3d5" stroke="${INK}" stroke-width="1.5"/><path d="M${64 + len / 2 + (28 - len) / 2} ${by - 6} l8 -6 v12 z" fill="#7fb3d5" stroke="${INK}" stroke-width="1.5" stroke-linejoin="round"/><circle cx="${58 + (28 - len) / 2}" cy="${by - 8}" r="1.3" fill="${INK}"/></g>`);
        }
        break;
      }
      case "crumbs": over.push(`<circle cx="${82 + fx}" cy="${headY + 26}" r="1.5" fill="#33691e"/><circle cx="${86 + fx}" cy="${headY + 31}" r="1.2" fill="#33691e"/>`); break;
      case "ball1": over.push(`<circle cx="102" cy="${by - 30}" r="9" fill="#ff7043" stroke="${INK}" stroke-width="2"/><path d="M93 ${by - 30} q9 6 18 0" fill="none" stroke="#fff" stroke-width="2"/>`); break;
      case "ball2": over.push(`<circle cx="108" cy="${by - 52}" r="9" fill="#ff7043" stroke="${INK}" stroke-width="2"/><path d="M99 ${by - 52} q9 -6 18 0" fill="none" stroke="#fff" stroke-width="2"/>`); break;
      case "ball3": over.push(`<circle cx="104" cy="${by + 18}" r="9" fill="#ff7043" stroke="${INK}" stroke-width="2"/><path d="M95 ${by + 18} q9 6 18 0" fill="none" stroke="#fff" stroke-width="2"/>`); break;
      case "zzz1": over.push(text(104, 16, 12, "z"), text(114, 8, 9, "z")); break;
      case "zzz2": over.push(text(103, 18, 14, "z"), text(114, 9, 10, "z")); break;
      case "heart": over.push(`<path d="M108 22 c-4 -6 -12 -2 -8 4 l8 8 l8 -8 c4 -6 -4 -10 -8 -4z" fill="#ff5c7a"/>`); break;
      case "tear1": over.push(`<path d="M${lx - 2} ${ey + 9} q-3 5 0 7 q3 -2 0 -7z" fill="#7ec8ff"/>`); break;
      case "tear2": over.push(`<path d="M${lx - 2} ${ey + 15} q-3 5 0 7 q3 -2 0 -7z" fill="#7ec8ff"/>`); break;
      case "bang":
        over.push(`<circle cx="110" cy="18" r="11" fill="#ffcc33" stroke="${INK}" stroke-width="2"/>`, text(106.5, 24.5, 17, "!"));
        break;
      case "dots": over.push(text(102, 14, 16, "…")); break;
      case "sweat": over.push(`<path d="M100 ${headY - 16} q-4 6 0 9 q4 -3 0 -9z" fill="#7ec8ff"/>`); break;
      case "sweat2":
        over.push(`<path d="M100 ${headY - 12} q-4 6 0 9 q4 -3 0 -9z" fill="#7ec8ff"/>`, `<path d="M26 ${headY - 6} q-4 6 0 9 q4 -3 0 -9z" fill="#7ec8ff"/>`);
        break;
      case "steam1": over.push(`<path d="M44 ${headY - 30} q-4 -4 0 -8 q4 -4 0 -8 M84 ${headY - 30} q-4 -4 0 -8" fill="none" stroke="#bbb" stroke-width="2.5" stroke-linecap="round"/>`); break;
      case "steam2": over.push(`<path d="M44 ${headY - 30} q4 -4 0 -8 q-4 -4 0 -8 M84 ${headY - 30} q4 -4 0 -8" fill="none" stroke="#bbb" stroke-width="2.5" stroke-linecap="round"/>`); break;
      case "rumble1": over.push(`<path d="M50 ${by + 2} q3 -3 6 0 q3 3 6 0 M66 ${by + 8} q3 -3 6 0 q3 3 6 0" fill="none" stroke="${INK}" stroke-width="1.5" opacity=".6"/>`); break;
      case "rumble2": over.push(`<path d="M52 ${by + 6} q3 3 6 0 q3 -3 6 0 M64 ${by} q3 3 6 0 q3 -3 6 0" fill="none" stroke="${INK}" stroke-width="1.5" opacity=".6"/>`); break;
      case "thought": over.push(`<circle cx="98" cy="20" r="2" fill="#ccc"/><circle cx="104" cy="14" r="3" fill="#ccc"/><ellipse cx="114" cy="7" rx="10" ry="6" fill="#eee" stroke="#ccc"/>`, sp.food === "bamboo" ? `<rect x="108" y="5" width="12" height="4" rx="1" fill="#8bc34a" transform="rotate(-15 114 7)"/>` : `<ellipse cx="114" cy="7" rx="6" ry="3" fill="#7fb3d5"/>`); break;
    }
  }
  p.push(...props, ...armsSvg, ...over);

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">\n  ${p.join("\n  ")}\n</svg>\n`;
}

const ANIMATIONS = {
  idle: { fps: 3, frames: [{}, {}, { dy: 1, tailSwing: 6 }, { dy: 1, tailSwing: 6 }, {}, { eyes: "closed" }] },
  walk: {
    fps: 6,
    frames: [
      { fx: 4, legs: "walkA", dy: -1, tailSwing: -6 },
      { fx: 4, legs: "stand" },
      { fx: 4, legs: "walkB", dy: -1, tailSwing: 6 },
      { fx: 4, legs: "stand" },
    ],
  },
  sit: {
    fps: 2,
    frames: [
      { sitting: true, legs: "sit", arms: "rest" },
      { sitting: true, legs: "sit", arms: "rest", tailSwing: 8 },
      { sitting: true, legs: "sit", arms: "rest", eyes: "closed" },
    ],
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
      { eyes: "happy", mouth: "open", arms: "up", dy: -6, extras: ["heart"], tailSwing: -10 },
      { eyes: "happy", mouth: "open", arms: "up", dy: -2, extras: ["heart"], tailSwing: 10 },
      { eyes: "happy", mouth: "open", arms: "down" },
      { eyes: "happy", mouth: "smile", arms: "down" },
    ],
  },
  sad: {
    fps: 2,
    frames: [
      { eyes: "sad", mouth: "frown", arms: "down", extras: ["tear1"], tailSwing: 20 },
      { eyes: "sad", mouth: "frown", arms: "down", extras: ["tear2"], dy: 1, tailSwing: 20 },
    ],
  },
  bored: {
    fps: 1.5,
    frames: [
      { sitting: true, legs: "sit", arms: "rest", eyes: "half", mouth: "flat", extras: ["dots"] },
      { sitting: true, legs: "sit", arms: "rest", eyes: "half", mouth: "flat", dy: 1, tailSwing: 8 },
    ],
  },
  surprised: {
    fps: 4,
    frames: [
      { eyes: "wide", mouth: "o", arms: "up", extras: ["sweat"], tailSwing: -25 },
      { eyes: "wide", mouth: "o", arms: "up", dy: -2, extras: ["sweat"], tailSwing: -25 },
    ],
  },
  drag: {
    fps: 4,
    frames: [
      { eyes: "wide", mouth: "o", arms: "dangleA", legs: "dangle", shadow: false, tailSwing: 30 },
      { eyes: "wide", mouth: "o", arms: "dangleB", legs: "dangle", shadow: false, dy: 1, tailSwing: 20 },
    ],
  },
  alert: {
    fps: 4,
    frames: [
      { eyes: "wide", mouth: "open", arms: "waveL", extras: ["bang"] },
      { eyes: "wide", mouth: "open", arms: "waveR", extras: ["bang"], dy: -2 },
    ],
  },
  eat: {
    fps: 4,
    frames: [
      { sitting: true, legs: "sit", arms: "hold", eyes: "happy", mouth: "o", extras: ["food1"] },
      { sitting: true, legs: "sit", arms: "hold", eyes: "closed", mouth: "chew", extras: ["food2"], dy: 1 },
      { sitting: true, legs: "sit", arms: "hold", eyes: "happy", mouth: "chew", extras: ["food2", "crumbs"] },
      { sitting: true, legs: "sit", arms: "hold", eyes: "closed", mouth: "chew", extras: ["food2", "crumbs"], dy: 1 },
    ],
  },
  play: {
    fps: 6,
    frames: [
      { eyes: "happy", mouth: "open", arms: "waveR", extras: ["ball1"], tailSwing: -10 },
      { eyes: "happy", mouth: "open", arms: "up", dy: -5, extras: ["ball2"], tailSwing: 10 },
      { eyes: "wide", mouth: "open", arms: "up", dy: -3, extras: ["ball2"] },
      { eyes: "happy", mouth: "smile", arms: "down", extras: ["ball3"] },
    ],
  },
  sweat: {
    fps: 3,
    frames: [
      { eyes: "half", mouth: "wavy", arms: "fan", extras: ["flush", "sweat2", "steam1"] },
      { eyes: "half", mouth: "wavy", arms: "down", dy: 1, extras: ["flush", "sweat2", "steam2"] },
    ],
  },
  hungry: {
    fps: 2,
    frames: [
      { eyes: "droopy", mouth: "o", arms: "rest", extras: ["rumble1", "thought"] },
      { eyes: "droopy", mouth: "flat", arms: "rest", dy: 1, extras: ["rumble2", "thought"] },
    ],
  },
};

for (const [id, sp] of Object.entries(SPECIES)) {
  const out = join(PETS, id);
  mkdirSync(out, { recursive: true });
  for (const f of readdirSync(out)) if (f.endsWith(".svg")) rmSync(join(out, f));
  const manifest = {
    id,
    name: sp.name,
    author: "DesktopPet (generated by tools/gen-pets.mjs)",
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
      writeFileSync(join(out, file), pet(sp, opts));
      manifest.animations[name].frames.push(file);
    });
  }
  writeFileSync(join(out, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  const n = Object.values(manifest.animations).reduce((sum, a) => sum + a.frames.length, 0);
  console.log(`${id}: wrote ${n} frames`);
}
