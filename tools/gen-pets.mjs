// Generates the built-in pet packs (SVG frames + manifest.json):
//
//   node tools/gen-pets.mjs
//
// Both pets share one chibi skeleton (big head, small body, capsule limbs)
// and differ in colours and markings, so every animation exists for both.
// Motion cycles (walking, climbing, dancing, pedalling...) are computed from
// a phase angle, so frames flow into each other. Tweak and regenerate; the app
// only reads pets/<id>/manifest.json and the files it lists.

import { mkdirSync, writeFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const PETS = join(dirname(fileURLToPath(import.meta.url)), "..", "pets");
const INK = "#2d2a2e";
const TAU = Math.PI * 2;
const r1 = (n) => Math.round(n * 10) / 10;

const SPECIES = {
  panda: {
    name: "Panda",
    fur: "#ffffff",
    furShade: "#e6e6ee",
    belly: "#f6f6fa",
    limb: "#2d2a2e",
    limbLine: "#2d2a2e",
    ears: "round",
    earColor: "#2d2a2e",
    earInner: "#57515c",
    patches: true,
    band: true,
    nose: "#2d2a2e",
    blush: "#ff9fb6",
    pupil: "#1d1a1e",
    tail: null,
    food: "bamboo",
  },
  cat: {
    name: "Ginger Cat",
    fur: "#f9b25c",
    furShade: "#ec9537",
    belly: "#fff1de",
    limb: "#f5a246",
    limbLine: INK,
    ears: "pointy",
    earColor: "#f9b25c",
    earInner: "#ffb8c9",
    patches: false,
    band: false,
    stripes: "#d97a22",
    muzzle: "#fff1de",
    nose: "#f06b85",
    blush: "#ff8fa6",
    pupil: "#3b2a22",
    tail: "#ef9a3c",
    food: "fish",
  },
};

// ------------------------------------------------------------ primitives --

const svgOpen = (sp, id) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">
  <defs>
    <radialGradient id="fur${id}" cx="42%" cy="32%" r="75%"><stop offset="0" stop-color="${sp.fur}"/><stop offset="0.72" stop-color="${sp.fur}"/><stop offset="1" stop-color="${sp.furShade}"/></radialGradient>
  </defs>`;

/** A rounded limb from (x1,y1) to (x2,y2), outlined. */
function capsule(x1, y1, x2, y2, w, fill, line = INK) {
  const a = `x1="${r1(x1)}" y1="${r1(y1)}" x2="${r1(x2)}" y2="${r1(y2)}" stroke-linecap="round"`;
  return `<line ${a} stroke="${line}" stroke-width="${w + 5}"/><line ${a} stroke="${fill}" stroke-width="${w}"/>`;
}

const ellipse = (cx, cy, rx, ry, fill, extra = "") =>
  `<ellipse cx="${r1(cx)}" cy="${r1(cy)}" rx="${r1(rx)}" ry="${r1(ry)}" fill="${fill}" ${extra}/>`;

const text = (x, y, size, s, fill = INK, w = 700) =>
  `<text x="${r1(x)}" y="${r1(y)}" font-family="Segoe UI, Arial, sans-serif" font-weight="${w}" font-size="${size}" fill="${fill}">${s}</text>`;

// ---------------------------------------------------------------- the pet --
//
// o = {
//   dx, dy:      whole-pet offset           rot:  body tilt around the feet (deg)
//   sitting:     sitting body               face: face shift (head turn), look: [x,y] pupils
//   eyes, mouth: expression                 ears: [leftDeg, rightDeg] wiggle
//   arms:        [[x,y],[x,y]] paw positions   legs: "stand" | "sit" | "none" | [[x,y,rot],[x,y,rot]] feet
//   tail:        swing (deg)                helmet, glasses: accessories
//   blush, shadow (scale or 0), headDy
// }

function feet(sp, list, sitting) {
  return list
    .map(([x, y, rot]) => {
      if (sitting) {
        // Soles towards us, with paw pads.
        return `<g transform="rotate(${rot} ${x} ${y})">${ellipse(x, y, 10, 11.5, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.4"`)}${ellipse(x, y + 2.5, 5, 4.2, "#ffb3c6")}${ellipse(x - 4.5, y - 5, 1.9, 2.2, "#ffb3c6")}${ellipse(x, y - 6.5, 1.9, 2.2, "#ffb3c6")}${ellipse(x + 4.5, y - 5, 1.9, 2.2, "#ffb3c6")}</g>`;
      }
      return ellipse(x, y, 11, 7.5, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.4" transform="rotate(${rot} ${x} ${y})"`);
    })
    .join("");
}

function eye(sp, cx, cy, kind, look) {
  const [lx, ly] = look;
  const lid = sp.patches ? "#fff" : INK;
  const stroke = (d, w = 2.6) => `<path d="${d}" fill="none" stroke="${lid}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"/>`;
  const shine = (x, y, big = 2.1) => ellipse(x + 1.9, y - 2.2, big, big, "#fff") + ellipse(x - 1.7, y + 2, big / 2.2, big / 2.2, "#fff");
  const glossy = (rw, rp) => {
    const sclera = sp.patches ? ellipse(cx, cy, rw, rw * 1.05, "#fff") : "";
    return sclera + ellipse(cx + lx, cy + ly, rp, rp * 1.12, sp.pupil) + shine(cx + lx, cy + ly, rp * 0.42);
  };
  switch (kind) {
    case "open": return glossy(6.4, 4.8);
    case "wide": return glossy(7.6, 4.2) + ellipse(cx + lx, cy + ly - 0.5, 1.2, 1.2, "#fff");
    case "closed": return stroke(`M${cx - 5} ${cy} q5 4.5 10 0`);
    case "happy": return stroke(`M${cx - 5.5} ${cy + 2} q5.5 -7 11 0`);
    case "sad": return glossy(6, 4.6) + stroke(cx < 64 ? `M${cx - 6} ${cy - 7} l10 3` : `M${cx + 6} ${cy - 7} l-10 3`, 2.4);
    case "half": {
      const c = sp.patches ? sp.limb : sp.fur;
      return glossy(6.2, 4.6) + `<path d="M${cx - 7.5} ${cy - 8} h15 v7.5 h-15 z" fill="${c}"/>` + stroke(`M${cx - 6} ${cy - 0.5} h12`, 2.4);
    }
    case "squint": return stroke(cx < 64 ? `M${cx - 5} ${cy - 4} l8 4 l-8 4` : `M${cx + 5} ${cy - 4} l-8 4 l8 4`);
    case "star": return `<path d="M${cx} ${cy - 7} l2 5 l5.5 0.4 l-4.2 3.4 l1.5 5.4 l-4.8 -3 l-4.8 3 l1.5 -5.4 l-4.2 -3.4 l5.5 -0.4 z" fill="${sp.patches ? "#ffe066" : "#ffcf33"}" stroke="${INK}" stroke-width="1"/>`;
    case "down": return glossy(6.2, 4.6);
    default: return "";
  }
}

function mouth(sp, nx, ny, kind) {
  const m = (d, fill = "none", w = 2.2) => `<path d="${d}" fill="${fill}" stroke="${INK}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"/>`;
  switch (kind) {
    case "w": return m(`M${nx - 6.5} ${ny + 4.5} q3.25 3.8 6.5 0 q3.25 3.8 6.5 0`);
    case "smile": return m(`M${nx - 6} ${ny + 5} q6 6 12 0`);
    case "open": return m(`M${nx - 7} ${ny + 4.5} q7 12 14 0 z`, "#e8576b") + ellipse(nx, ny + 10, 3.6, 2.2, "#ff9aa8");
    case "o": return ellipse(nx, ny + 8.5, 3.4, 4.2, "#e8576b", `stroke="${INK}" stroke-width="2"`);
    case "small": return ellipse(nx, ny + 7.5, 2.2, 2.6, "#e8576b", `stroke="${INK}" stroke-width="1.8"`);
    case "frown": return m(`M${nx - 6} ${ny + 9.5} q6 -6 12 0`);
    case "flat": return m(`M${nx - 5} ${ny + 7} h10`);
    case "chew": return m(`M${nx - 5} ${ny + 6} q5 5 10 0 q-5 2 -10 0z`, "#e8576b");
    case "wavy": return m(`M${nx - 8} ${ny + 8} q2 -3 4 0 q2 3 4 0 q2 -3 4 0 q2 3 4 0`);
    case "yawn": return ellipse(nx, ny + 9, 5.5, 7, "#c94a5c", `stroke="${INK}" stroke-width="2"`) + ellipse(nx, ny + 12.5, 3.2, 2, "#ff9aa8");
    case "grit": return m(`M${nx - 6} ${ny + 6} h12 v4 h-12 z`, "#fff", 1.8) + m(`M${nx - 2} ${ny + 6} v4 M${nx + 2} ${ny + 6} v4`, "none", 1.2);
    default: return "";
  }
}

function head(sp, o, id) {
  const p = [];
  const hy = 50 + (o.headDy ?? 0);
  const f = o.face ?? 0;
  const [el, er] = o.ears ?? [0, 0];
  // Ears behind the head.
  if (!o.helmet) {
    if (sp.ears === "round") {
      p.push(`<g transform="rotate(${el} 38 ${hy - 20})">${ellipse(33 + f * 0.4, hy - 28, 13.5, 13, sp.earColor)}${ellipse(35 + f * 0.4, hy - 26, 6.5, 6.5, sp.earInner)}</g>`);
      p.push(`<g transform="rotate(${er} 90 ${hy - 20})">${ellipse(95 + f * 0.4, hy - 28, 13.5, 13, sp.earColor)}${ellipse(93 + f * 0.4, hy - 26, 6.5, 6.5, sp.earInner)}</g>`);
    } else {
      const ear = (x, s, rot) => `<g transform="rotate(${rot} ${x} ${hy - 22})"><path d="M${x - 14 * s} ${hy - 16} L${x - 7 * s} ${hy - 47} L${x + 13 * s} ${hy - 25} Z" fill="${sp.earColor}" stroke="${INK}" stroke-width="2.6" stroke-linejoin="round"/><path d="M${x - 8 * s} ${hy - 21} L${x - 5.5 * s} ${hy - 38} L${x + 5 * s} ${hy - 26} Z" fill="${sp.earInner}"/></g>`;
      p.push(ear(37 + f * 0.4, 1, el), ear(91 + f * 0.4, -1, er));
    }
  }
  p.push(ellipse(64, hy, 38.5, 34, `url(#fur${id})`, `stroke="${INK}" stroke-width="2.6"`));
  if (sp.stripes) {
    for (const [x, len] of [[55, 10], [64, 13], [73, 10]]) {
      p.push(`<path d="M${x + f * 0.6} ${hy - 33} v${len}" stroke="${sp.stripes}" stroke-width="4" stroke-linecap="round"/>`);
    }
    p.push(`<path d="M27 ${hy + 2} h8 M27 ${hy + 9} h7 M101 ${hy + 2} h-8 M101 ${hy + 9} h-7" stroke="${sp.stripes}" stroke-width="3.2" stroke-linecap="round"/>`);
  }
  // Face
  const ex = 47 + f, ex2 = 81 + f, ey = hy + 2;
  if (sp.muzzle) p.push(ellipse(64 + f, hy + 15, 15, 10, sp.muzzle));
  if (sp.patches) {
    p.push(ellipse(ex, ey + 1, 11, 13.5, sp.limb, `transform="rotate(28 ${ex} ${ey + 1})"`));
    p.push(ellipse(ex2, ey + 1, 11, 13.5, sp.limb, `transform="rotate(-28 ${ex2} ${ey + 1})"`));
  }
  const look = o.look ?? [f * 0.25, 0];
  const eyes = o.eyes ?? "open";
  p.push(eye(sp, ex + 1, ey, eyes, eyes === "down" ? [look[0], 2.2] : look));
  p.push(eye(sp, ex2 - 1, ey, eyes, eyes === "down" ? [look[0], 2.2] : look));
  if (o.glasses) {
    p.push(`<g fill="none" stroke="#6b4b2a" stroke-width="2"><circle cx="${ex + 1}" cy="${ey}" r="9"/><circle cx="${ex2 - 1}" cy="${ey}" r="9"/><path d="M${ex + 10} ${ey - 1} q${(ex2 - ex - 20) / 2} -4 ${ex2 - ex - 20} 0"/></g>`);
  }
  // Blush
  if (o.blush !== false) {
    const hot = o.blush === "hot";
    p.push(ellipse(37 + f, hy + 15, hot ? 8 : 6.2, hot ? 5 : 3.8, hot ? "#ff6b6b" : sp.blush, `opacity="${hot ? 0.85 : 0.72}"`));
    p.push(ellipse(91 + f, hy + 15, hot ? 8 : 6.2, hot ? 5 : 3.8, hot ? "#ff6b6b" : sp.blush, `opacity="${hot ? 0.85 : 0.72}"`));
  }
  // Nose and mouth
  const nx = 64 + f, ny = hy + 12;
  if (sp.patches) {
    p.push(`<path d="M${nx - 5} ${ny - 2.5} q5 -2 10 0 q-1 4.5 -5 5 q-4 -0.5 -5 -5z" fill="${sp.nose}"/>` + ellipse(nx - 1.5, ny - 1.8, 1.3, 0.8, "#fff", `opacity=".7"`));
  } else {
    p.push(`<path d="M${nx - 4} ${ny - 2.5} h8 l-4 4.5 z" fill="${sp.nose}" stroke="${INK}" stroke-width="1.3" stroke-linejoin="round"/>`);
    for (const s of [-1, 1]) {
      for (const d of [-2, 3]) p.push(`<path d="M${nx + s * 13} ${ny + 3 + d} l${s * 15} ${d * 1.4 - 1}" stroke="${INK}" stroke-width="1.3" stroke-linecap="round"/>`);
    }
  }
  p.push(mouth(sp, nx, ny, o.mouth ?? "w"));
  if (o.helmet) {
    p.push(`<path d="M${24 + f * 0.3} ${hy - 4} q40 -62 80 0 z" fill="${o.helmet}" stroke="${INK}" stroke-width="2.6" stroke-linejoin="round"/>`);
    p.push(`<path d="M${30 + f * 0.3} ${hy - 9} q34 -14 68 0" fill="none" stroke="#fff" stroke-width="3" opacity=".6"/>`);
    p.push(ellipse(64 + f * 0.3, hy - 28, 4, 2.2, "#fff", `opacity=".8"`));
  }
  return p.join("");
}

function pet(sp, o, id) {
  const p = [];
  const sitting = !!o.sitting;
  const by = (sitting ? 100 : 95) + (o.bodyDy ?? 0);
  const g = [];
  // Tail (cat) behind everything.
  if (sp.tail && o.tail !== false) {
    const sw = o.tail ?? 0;
    const d = `M84 ${by + 10} q24 -2 22 -24 q-1 -10 7 -15`;
    g.push(`<g transform="rotate(${sw} 84 ${by + 10})"><path d="${d}" fill="none" stroke="${INK}" stroke-width="12.5" stroke-linecap="round"/><path d="${d}" fill="none" stroke="${sp.tail}" stroke-width="8" stroke-linecap="round"/><path d="M104 ${by - 12} l4 -3 M106 ${by - 4} l5 -1" stroke="${sp.stripes}" stroke-width="3" stroke-linecap="round"/></g>`);
  }
  // Feet
  const legSpec =
    o.legs === "none" ? [] : Array.isArray(o.legs) ? o.legs : sitting ? [[42, 114, -18], [86, 114, 18]] : [[50, 117, 0], [78, 117, 0]];
  let feetSvg = feet(sp, legSpec, sitting && !Array.isArray(o.legs));
  if (o.hang) {
    // Legs dangling from the hips, swinging by the given angles.
    feetSvg = o.hang
      .map((a, i) => {
        const hx = i ? 76 : 52, hy = by + 8;
        const rad = (a * Math.PI) / 180;
        const fx = hx + Math.sin(rad) * 20, fy = hy + Math.cos(rad) * 20;
        return capsule(hx, hy, fx, fy, 12, sp.limb, sp.limbLine) + ellipse(fx, fy + 2, 8.5, 7, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`) + ellipse(fx, fy + 4, 4, 2.6, "#ffb3c6");
      })
      .join("");
  }
  if (!sitting || o.hang) g.push(feetSvg);
  // Body
  g.push(ellipse(64, by, sitting ? 29 : 26.5, sitting ? 21 : 22.5, `url(#fur${id})`, `stroke="${INK}" stroke-width="2.6"`));
  g.push(ellipse(64, by + 4, 15, 12.5, sp.belly));
  if (sp.band) g.push(`<path d="M${38} ${by - 12} q26 -12 52 0 q-2 8 -6 10 q-20 -8 -40 0 q-4 -2 -6 -10z" fill="${sp.limb}"/>`);
  if (sitting && !o.hang) g.push(feetSvg);
  // Arms: capsules from the shoulders to the paws. Overhead poses go behind
  // the head (only the paws show in front) so they never cover the face.
  const sh = [[45, by - 10], [83, by - 10]];
  const arms = o.arms ?? [[37, by + 6], [91, by + 6]];
  const armSvg = arms.filter(([x]) => x != null).map(([x, y], i) => capsule(sh[i][0], sh[i][1], x, y, 12, sp.limb, sp.limbLine));
  const paws = arms.filter(([x]) => x != null).map(([x, y]) => ellipse(x, y, 7.5, 7, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.4"`));
  if (o.armsBehind) g.push(...armSvg);
  // Head, then anything held in front of it but behind the paws.
  g.push(head(sp, o, id));
  if (o.held) g.push(o.held);
  if (o.armsBehind) g.push(...paws);
  else g.push(...armSvg);
  if (o.front) g.push(o.front);

  const rot = o.rot ?? 0;
  const tx = o.dx ?? 0, ty = o.dy ?? 0, s = o.scale ?? 1;
  if (o.shadow !== 0) {
    const k = o.shadow ?? 1;
    p.push(ellipse(64 + tx, 124, 30 * k * s, 4 * k, "#000", `opacity=".14"`));
  }
  p.push(`<g transform="translate(${tx} ${ty}) scale(${s}) rotate(${rot} 64 ${sitting ? 118 : 122})">${g.join("")}</g>`);
  return p.join("");
}

/** Seen from behind, for climbing: c = cycle phase in radians. */
function petBack(sp, c, id, o = {}) {
  const up = (Math.sin(c) + 1) / 2; // 1: left paw high
  const sway = Math.sin(c) * 2.5;
  const lift = -Math.abs(Math.cos(c)) * 2.5;
  const g = [];
  const by = 92;
  // Legs push alternately (opposite to the arms).
  const lfy = 112 - (1 - up) * 12, rfy = 112 - up * 12;
  g.push(capsule(52, by + 12, 46, lfy, 13, sp.limb, sp.limbLine), capsule(76, by + 12, 82, rfy, 13, sp.limb, sp.limbLine));
  g.push(ellipse(45, lfy + 3, 9, 6.5, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`), ellipse(83, rfy + 3, 9, 6.5, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`));
  if (sp.tail) {
    const d = `M64 ${by + 18} q-2 14 10 18 q8 2 8 -6`;
    g.push(`<path d="${d}" fill="none" stroke="${INK}" stroke-width="12" stroke-linecap="round"/><path d="${d}" fill="none" stroke="${sp.tail}" stroke-width="7.5" stroke-linecap="round"/>`);
  }
  // Back
  g.push(ellipse(64, by, 26, 24, `url(#fur${id})`, `stroke="${INK}" stroke-width="2.6"`));
  if (sp.band) g.push(`<path d="M38 ${by - 14} q26 -14 52 0 q0 10 -4 12 q-22 -9 -44 0 q-4 -2 -4 -12z" fill="${sp.limb}"/>`);
  if (sp.stripes) {
    for (const yy of [by - 6, by + 3, by + 12]) g.push(`<path d="M52 ${yy} q12 6 24 0" fill="none" stroke="${sp.stripes}" stroke-width="3.5" stroke-linecap="round"/>`);
  }
  // Upper arms, behind the head.
  {
    const lpy = 44 - up * 38, rpy = 44 - (1 - up) * 38;
    g.push(capsule(42, by - 14, 19, lpy + 2, 12, sp.limb, sp.limbLine), capsule(86, by - 14, 109, rpy + 2, 12, sp.limb, sp.limbLine));
  }
  // Head from behind
  if (sp.ears === "round") {
    g.push(ellipse(33, 24, 13.5, 13, sp.earColor), ellipse(95, 24, 13.5, 13, sp.earColor));
  } else {
    g.push(`<path d="M23 34 L30 3 L50 25 Z M105 34 L98 3 L78 25 Z" fill="${sp.earColor}" stroke="${INK}" stroke-width="2.6" stroke-linejoin="round"/>`);
  }
  g.push(ellipse(64, 48, 37, 32, `url(#fur${id})`, `stroke="${INK}" stroke-width="2.6"`));
  if (sp.stripes) {
    for (const [x, len] of [[54, 11], [64, 14], [74, 11]]) g.push(`<path d="M${x} 20 v${len}" stroke="${sp.stripes}" stroke-width="4" stroke-linecap="round"/>`);
  } else {
    g.push(`<path d="M60 18 q4 -6 8 0" fill="none" stroke="${INK}" stroke-width="2" stroke-linecap="round"/>`); // a tuft
  }
  // Arms reaching up beside the head, paws gripping (drawn before the head
  // above, so they are re-added here only as paws).
  const lpy = 44 - up * 38, rpy = 44 - (1 - up) * 38;
  g.push(ellipse(19, lpy, 8.5, 7, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`), ellipse(109, rpy, 8.5, 7, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`));
  g.push(`<path d="M15 ${lpy - 3} v4 M19 ${lpy - 4} v4 M23 ${lpy - 3} v4 M105 ${rpy - 3} v4 M109 ${rpy - 4} v4 M113 ${rpy - 3} v4" stroke="${sp.limbLine === INK ? "#8a8590" : INK}" stroke-width="1.2" stroke-linecap="round"/>`);
  if (o.sweat) g.push(`<path d="M104 18 q-4 6 0 9 q4 -3 0 -9z" fill="#7ec8ff"/>`);
  return `<g transform="translate(${r1(sway)} ${r1(lift)})">${g.join("")}</g>`;
}

// ----------------------------------------------------------------- props --

const heart = (x, y, s = 1, op = 1) => `<path transform="translate(${r1(x)} ${r1(y)}) scale(${s})" d="M0 -3 c-4 -6 -12 -2 -8 4 l8 8 l8 -8 c4 -6 -4 -10 -8 -4z" fill="#ff5c7a" opacity="${op}"/>`;
const drop = (x, y, s = 1) => `<path transform="translate(${r1(x)} ${r1(y)}) scale(${s})" d="M0 0 q-4 6 0 9 q4 -3 0 -9z" fill="#7ec8ff"/>`;
const note = (x, y, ch, rot = 0) => `<g transform="rotate(${rot} ${x} ${y})">${text(x, y, 15, ch, "#6a4cc2")}</g>`;
const sparkle = (x, y, s = 1) => `<path transform="translate(${r1(x)} ${r1(y)}) scale(${s})" d="M0 -6 L1.5 -1.5 L6 0 L1.5 1.5 L0 6 L-1.5 1.5 L-6 0 L-1.5 -1.5 Z" fill="#ffd84d"/>`;
const zz = (x, y, s) => text(x, y, s, "z", "#5b6b8c");

function food(sp, bite) {
  if (sp.food === "bamboo") {
    const top = bite ? 70 : 62;
    return `<g transform="rotate(-24 64 92)"><rect x="60" y="${top}" width="8.5" height="${108 - top}" rx="3" fill="#8bc34a" stroke="#33691e" stroke-width="1.6"/><path d="M60 82 h8.5 M60 96 h8.5" stroke="#33691e" stroke-width="1.6"/><path d="M68.5 86 q13 -7 18 2 q-11 3 -18 -2z" fill="#7cb342" stroke="#33691e" stroke-width="1.2"/></g>`;
  }
  const len = bite ? 20 : 28;
  return `<g transform="rotate(-18 64 84)"><ellipse cx="${64 + (28 - len) / 2}" cy="84" rx="${len / 2}" ry="6.5" fill="#86bfe3" stroke="${INK}" stroke-width="1.6"/><path d="M${78 - (28 - len) / 2 + (28 - len) / 2} 84 l8 -6 v12 z" fill="#86bfe3" stroke="${INK}" stroke-width="1.6" stroke-linejoin="round"/><circle cx="${57 + (28 - len) / 2}" cy="82" r="1.4" fill="${INK}"/></g>`;
}

const ball = (x, y) => `${ellipse(x, y, 9.5, 9.5, "#ff7043", `stroke="${INK}" stroke-width="2"`)}<path d="M${x - 9} ${y} q9 6 18 0" fill="none" stroke="#fff" stroke-width="2"/>`;

function book(flip) {
  return `<g transform="translate(64 92)"><path d="M-22 -10 L0 -6 L22 -10 L22 10 L0 14 L-22 10 Z" fill="#7a5cc9" stroke="${INK}" stroke-width="2" stroke-linejoin="round"/><path d="M-19 -8 L0 -4 L0 11 L-19 7 Z M19 -8 L0 -4 L0 11 L19 7 Z" fill="#fffaf0" stroke="#bbb" stroke-width="1"/><path d="M-15 -3 l11 2 M-15 1 l11 2 M15 -3 l-11 2 M15 1 l-11 2" stroke="#bbb" stroke-width="1.2"/>${flip ? `<path d="M0 -4 Q8 -18 16 -10 L0 11 Z" fill="#fff" stroke="#bbb" stroke-width="1"/>` : ""}</g>`;
}

function ladderFlat(y) {
  const rungs = [];
  for (let x = 14; x <= 114; x += 12) rungs.push(`M${x} ${y - 6} v12`);
  return `<path d="M6 ${y - 6} h116 M6 ${y + 6} h116" stroke="#7a4f24" stroke-width="5" stroke-linecap="round"/><path d="M6 ${y - 6} h116 M6 ${y + 6} h116" stroke="#c58b4e" stroke-width="3" stroke-linecap="round"/><path d="${rungs.join(" ")}" stroke="#a8743e" stroke-width="3"/>`;
}

function umbrella(sway) {
  const scallop = [];
  for (let i = 0; i < 6; i++) scallop.push(`q-8.33 -7 -16.67 0`);
  return `<g transform="rotate(${sway} 64 70)"><path d="M64 26 v-4" stroke="${INK}" stroke-width="3"/><path d="M14 26 Q64 -22 114 26 ${scallop.join(" ")} Z" fill="#ff6b6b" stroke="${INK}" stroke-width="2.4" stroke-linejoin="round"/><path d="M64 3 Q52 12 47.3 26 M64 3 Q76 12 80.7 26" fill="none" stroke="#fff" stroke-width="4" opacity=".75"/><path d="M64 26 v8" stroke="${INK}" stroke-width="3.2" stroke-linecap="round"/><circle cx="64" cy="2" r="3" fill="${INK}"/></g>`;
}

function cloud(k, fade) {
  const puffs = [[64, 70, 26], [40, 78, 18], [88, 78, 18], [52, 56, 17], [78, 56, 17], [64, 92, 18]];
  return puffs
    .map(([x, y, r]) => ellipse(64 + (x - 64) * k, 70 + (y - 70) * k, r * k, r * k, "#fff", `stroke="#cfd6e0" stroke-width="2.5" opacity="${fade}"`))
    .join("") + sparkle(22 + 6 * k, 30, k) + sparkle(106 - 6 * k, 40, k * 0.8) + sparkle(30, 112 - 8 * k, k * 0.7);
}

function wheel(cx, cy, r, angle, tire = INK, hub = "#bbb") {
  const spokes = [0, 1, 2, 3].map((i) => {
    const a = angle + (i * Math.PI) / 4;
    return `M${r1(cx - Math.cos(a) * (r - 3))} ${r1(cy - Math.sin(a) * (r - 3))} L${r1(cx + Math.cos(a) * (r - 3))} ${r1(cy + Math.sin(a) * (r - 3))}`;
  });
  return `${ellipse(cx, cy, r, r, "none", `stroke="${tire}" stroke-width="4"`)}<path d="${spokes.join(" ")}" stroke="${hub}" stroke-width="1.6"/>${ellipse(cx, cy, 3, 3, hub, `stroke="${INK}" stroke-width="1"`)}`;
}

function puffs(t, x = 8, y = 100) {
  return [0, 1, 2].map((i) => {
    const k = (t + i / 3) % 1;
    return ellipse(x - k * 10, y - k * 6, 3 + k * 5, 3 + k * 5, "#d6d9de", `opacity="${r1(0.85 - k * 0.8)}"`);
  }).join("");
}

const speedLines = (t) => [0, 1, 2].map((i) => `<path d="M${r1(2 + ((t * 20 + i * 9) % 18))} ${78 + i * 12} h14" stroke="#b9c3d1" stroke-width="2.5" stroke-linecap="round"/>`).join("");

// ------------------------------------------------------------ animations --

const cyc = (n, f) => Array.from({ length: n }, (_, i) => f((i / n) * TAU, i));
const walkFeet = (t, lift = 5, stride = 8) => [
  [50 + stride * Math.sin(t), 117 - lift * Math.max(0, Math.sin(t)), 0],
  [78 - stride * Math.sin(t), 117 - lift * Math.max(0, -Math.sin(t)), 0],
];

const FRAMES = {
  idle: {
    fps: 6,
    frames: [
      {}, {}, { headDy: 1, bodyDy: 0.5 }, { headDy: 1, bodyDy: 0.5, ears: [-8, 0] },
      { headDy: 1, bodyDy: 0.5 }, { eyes: "closed" }, {}, { look: [-1.6, 0], face: -1 },
    ],
  },
  walk: {
    fps: 10,
    frames: cyc(8, (t) => ({
      face: 4,
      look: [1.5, 0],
      dy: -Math.abs(Math.sin(t)) * 2.5,
      legs: walkFeet(t),
      arms: [[37 - 5 * Math.sin(t), 100], [91 + 5 * Math.sin(t), 100]],
      tail: Math.sin(t) * 8,
    })),
  },
  sit: {
    fps: 3,
    frames: [
      { sitting: true, arms: [[36, 106], [92, 106]] },
      { sitting: true, arms: [[36, 106], [92, 106]], headDy: 1, tail: 10 },
      { sitting: true, arms: [[36, 106], [92, 106]], eyes: "closed" },
      { sitting: true, arms: [[36, 106], [92, 106]], tail: -6 },
    ],
  },
  sleep: {
    fps: 2,
    frames: [3, 5, 7, 4].map((b, i) => ({
      sitting: true,
      arms: [[36, 106], [92, 106]],
      eyes: "closed",
      mouth: "small",
      headDy: 3 + (i % 2),
      front: `${ellipse(72, 66, b, b, "#bfe6ff", `opacity=".7" stroke="#8fc9ee" stroke-width="1.2"`)}${zz(98, 26 - i * 3, 11 + i)}${zz(108, 14 - i * 2, 8)}`,
    })),
  },
  happy: {
    fps: 9,
    frames: [0, -4, -9, -9, -4, 0].map((h, i) => ({
      dy: h,
      eyes: "happy",
      mouth: "open",
      arms: i < 4 ? [[28, 60], [100, 60]] : [[34, 92], [94, 92]],
      legs: h < -2 ? [[52, 114, -10], [76, 114, 10]] : undefined,
      shadow: 1 + h / 30,
      tail: -h * 1.5,
      front: heart(110, 16 - i * 2, 1, 1 - i * 0.12),
    })),
  },
  sad: {
    fps: 3,
    frames: [0, 1, 2, 3].map((i) => ({
      eyes: "sad",
      mouth: "frown",
      ears: [-14, 14],
      look: [0, 1.5],
      arms: [[40, 104], [88, 104]],
      headDy: 2,
      tail: 25,
      front: drop(42, 64 + i * 4, 1),
    })),
  },
  bored: {
    fps: 2,
    frames: [
      { sitting: true, eyes: "half", mouth: "flat", arms: [[36, 106], [92, 106]], front: text(100, 18, 16, "…", "#777") },
      { sitting: true, eyes: "half", mouth: "flat", arms: [[36, 106], [92, 106]], headDy: 1 },
      { sitting: true, eyes: "closed", mouth: "yawn", arms: [[40, 70], [80, 108]], headDy: -1 },
      { sitting: true, eyes: "half", mouth: "flat", arms: [[36, 106], [92, 106]], tail: 8 },
    ],
  },
  surprised: {
    fps: 6,
    frames: [0, -5, -2].map((h) => ({
      dy: h,
      eyes: "wide",
      mouth: "o",
      arms: [[26, 62], [102, 62]],
      ears: [-6, 6],
      tail: -30,
      front: drop(104, 26, 1.1) + text(12, 26, 18, "!", "#ff6b6b"),
    })),
  },
  drag: {
    fps: 5,
    frames: cyc(4, (t) => ({
      eyes: "squint",
      mouth: "o",
      arms: [[34, 54], [94, 54]],
      legs: [[50 + 4 * Math.sin(t), 119, 12 * Math.sin(t)], [78 + 4 * Math.sin(t), 119, 12 * Math.sin(t)]],
      shadow: 0,
      tail: 30 * Math.sin(t),
    })),
  },
  alert: {
    fps: 6,
    frames: cyc(4, (t, i) => ({
      eyes: "wide",
      mouth: "open",
      dy: i % 2 ? -2 : 0,
      arms: [[26, 58 + 10 * Math.sin(t)], [91, 100]],
      front: `${ellipse(110, 20, 12, 12, "#ffcc33", `stroke="${INK}" stroke-width="2"`)}${text(106, 27, 18, "!")}`,
    })),
  },
  eat: {
    fps: 6,
    frames: cyc(6, (t, i) => ({
      sitting: true,
      eyes: i % 3 === 2 ? "closed" : "happy",
      mouth: i % 2 ? "chew" : "o",
      arms: [[56, 90], [72, 90]],
      headDy: i % 2,
      held: null,
      bite: i >= 2,
      crumbs: i >= 3,
    })),
  },
  play: {
    fps: 8,
    frames: [
      { eyes: "happy", mouth: "open", arms: [[34, 92], [104, 62]], ball: [108, 46] },
      { eyes: "happy", mouth: "open", dy: -5, arms: [[28, 60], [100, 60]], ball: [104, 16], legs: [[52, 114, -10], [76, 114, 10]] },
      { eyes: "wide", mouth: "open", dy: -3, arms: [[28, 60], [100, 60]], ball: [96, 12] },
      { eyes: "happy", mouth: "w", arms: [[34, 92], [100, 80]], ball: [104, 50] },
      { eyes: "happy", mouth: "open", arms: [[34, 92], [104, 70]], ball: [108, 96] },
      { eyes: "happy", mouth: "w", arms: [[37, 100], [91, 100]], ball: [112, 112] },
    ],
  },
  sweat: {
    fps: 4,
    frames: cyc(4, (t, i) => ({
      eyes: "half",
      mouth: "wavy",
      blush: "hot",
      arms: [[37, 100], [100 + 4 * Math.sin(t * 2), 44]],
      front: drop(102, 34 + i * 3, 1) + drop(24, 44 + ((i + 2) % 4) * 3, 0.9) +
        `<path d="M${40 + i} 8 q-4 -4 0 -8 M${86 - i} 8 q4 -4 0 -8" fill="none" stroke="#c9c9c9" stroke-width="2.5" stroke-linecap="round"/>`,
    })),
  },
  hungry: {
    fps: 3,
    frames: [0, 1, 2, 3].map((i) => ({
      eyes: "half",
      mouth: i % 2 ? "o" : "small",
      arms: [[56, 104], [72, 104]],
      headDy: 1,
      front:
        `<path d="M${50 + (i % 2) * 2} 98 q3 -3 6 0 q3 3 6 0 M${66 - (i % 2) * 2} 104 q3 -3 6 0 q3 3 6 0" fill="none" stroke="${INK}" stroke-width="1.6" opacity=".55"/>` +
        `${ellipse(98, 22, 2, 2, "#ddd")}${ellipse(104, 15, 3, 3, "#ddd")}${ellipse(114, 8, 11, 7, "#f3f3f3", `stroke="#ccc"`)}` +
        drop(70, 74 + i, 0.6),
    })),
  },
  jump: {
    fps: 6,
    frames: [
      { eyes: "closed", mouth: "open", arms: [[28, 56], [100, 56]], legs: [[52, 110, -20], [76, 110, 20]], shadow: 0.5, tail: -20 },
      { eyes: "happy", mouth: "open", arms: [[24, 62], [104, 62]], legs: [[50, 112, -14], [78, 112, 14]], shadow: 0.4, tail: 20 },
      { eyes: "wide", mouth: "o", arms: [[26, 70], [102, 70]], legs: [[50, 116, -6], [78, 116, 6]], shadow: 0.6 },
    ],
  },
  perch: {
    fps: 4,
    // The seat is at y ≈ 108; legs hang below it, over the window's edge.
    frames: cyc(6, (t, i) => ({
      sitting: true,
      bodyDy: -12,
      headDy: -12,
      hang: [18 * Math.sin(t), -18 * Math.sin(t + 1)],
      arms: [[34, 100], [94, 100]],
      look: [2 * Math.sin(t / 2), 0],
      eyes: i === 4 ? "closed" : "open",
      shadow: 0,
      tail: 12 * Math.sin(t),
    })),
  },
  carry: {
    fps: 10,
    frames: cyc(8, (t) => ({
      face: 3,
      dy: -Math.abs(Math.sin(t)) * 2,
      legs: walkFeet(t, 4, 7),
      arms: [[34, 10], [94, 10]],
      armsBehind: true,
      eyes: "open",
      mouth: "grit",
      ladder: true,
      tail: Math.sin(t) * 8,
    })),
  },
  float: {
    fps: 3,
    frames: cyc(4, (t) => ({
      dy: 22,
      scale: 0.8,
      dx: 12.8,
      eyes: "happy",
      mouth: "w",
      arms: [[56, 10], [72, 10]],
      armsBehind: true,
      legs: [[50 + 3 * Math.sin(t), 120, 14 * Math.sin(t)], [78 + 3 * Math.sin(t), 120, 14 * Math.sin(t)]],
      shadow: 0,
      umbrella: 4 * Math.sin(t),
    })),
  },
  poof: { fps: 8, frames: [0.5, 0.85, 1, 0.8].map((k, i) => ({ poof: [k, i === 3 ? 0.6 : 1] })) },
  rideBike: { fps: 10, frames: cyc(4, (t) => ({ vehicle: "bike", t })) },
  rideMoto: { fps: 12, frames: cyc(4, (t) => ({ vehicle: "moto", t })) },
  rideCar: { fps: 10, frames: cyc(4, (t) => ({ vehicle: "car", t })) },
  rideSkate: { fps: 8, frames: cyc(4, (t) => ({ vehicle: "skate", t })) },
  dance: {
    fps: 8,
    frames: cyc(8, (t, i) => ({
      rot: 7 * Math.sin(t),
      dy: -3 * Math.abs(Math.sin(t * 2)),
      eyes: i % 4 === 3 ? "star" : "happy",
      mouth: "open",
      arms: Math.sin(t) > 0 ? [[26, 52], [92, 100]] : [[36, 100], [102, 52]],
      legs: [[48 - 3 * Math.sin(t), 117 - 4 * Math.max(0, Math.sin(t)), -10 * Math.sin(t)], [80 - 3 * Math.sin(t), 117 - 4 * Math.max(0, -Math.sin(t)), -10 * Math.sin(t)]],
      tail: 20 * Math.sin(t),
      front: note(12 + (i % 4) * 2, 34 - (i % 4) * 5, "♪", -10) + note(104 - (i % 3) * 2, 26 - (i % 3) * 4, "♫", 10),
    })),
  },
  read: {
    fps: 2,
    frames: [0, 1, 2, 3].map((i) => ({
      sitting: true,
      eyes: i === 2 ? "closed" : "down",
      mouth: i === 3 ? "small" : "w",
      glasses: true,
      arms: [[48, 98], [80, 98]],
      headDy: 2,
      book: i === 3,
    })),
  },
  climb: {
    fps: 7,
    frames: cyc(6, (t, i) => ({ back: t, sweat: i >= 4 })),
  },
};

function vehicle(sp, kind, t, id) {
  const ang = -t * 2; // wheel spin
  const p = [];
  if (kind === "bike") {
    // Pedals turn around the bottom bracket.
    const bb = [60, 104];
    const ped = [0, Math.PI].map((o) => [bb[0] + 9 * Math.cos(t + o), bb[1] + 9 * Math.sin(t + o)]);
    const s = 0.68, tx = 16, ty = -2;
    const toPet = ([x, y]) => [(x - tx) / s, (y - ty) / s];
    const hips = [[tx + 52 * s, ty + 108 * s], [tx + 72 * s, ty + 108 * s]];
    p.push(speedLines(t / TAU));
    p.push(wheel(30, 104, 18, ang), wheel(100, 104, 18, ang));
    p.push(`<path d="M30 104 L60 104 L54 76 Z M54 78 L90 78 L60 104 M90 78 L100 104 M88 70 L90 78" fill="none" stroke="#3f9ad8" stroke-width="4" stroke-linejoin="round" stroke-linecap="round"/>`);
    p.push(`<path d="M84 68 h12" stroke="${INK}" stroke-width="4" stroke-linecap="round"/>`, ellipse(52, 73, 9, 3.5, INK));
    p.push(capsule(hips[1][0], hips[1][1], ped[1][0], ped[1][1], 9, sp.limb, sp.limbLine));
    p.push(pet(sp, { scale: s, dx: tx, dy: ty, face: 5, look: [2, 0], eyes: "happy", mouth: "open", legs: "none", arms: [toPet([90, 68]), toPet([95, 68])], armsBehind: true, shadow: 0, tail: 15 }, id));
    p.push(capsule(hips[0][0], hips[0][1], ped[0][0], ped[0][1], 9, sp.limb, sp.limbLine));
    p.push(ellipse(ped[0][0], ped[0][1], 6, 3, "#555"), ellipse(ped[1][0], ped[1][1], 6, 3, "#555"));
  } else if (kind === "moto") {
    const s = 0.66, tx = 16, ty = -4;
    const toPet = ([x, y]) => [(x - tx) / s, (y - ty) / s];
    p.push(puffs(t / TAU, 14, 100));
    p.push(wheel(32, 106, 16, ang, INK, "#999"), wheel(100, 106, 16, ang, INK, "#999"));
    p.push(`<path d="M20 96 h20" stroke="#888" stroke-width="5" stroke-linecap="round"/>`);
    p.push(`<path d="M30 92 Q34 78 52 80 L86 76 Q100 76 104 90 L96 94 L64 98 Q46 100 30 92 Z" fill="#ef5350" stroke="${INK}" stroke-width="2.4" stroke-linejoin="round"/>`);
    p.push(ellipse(70, 79, 16, 7, "#e53935", `stroke="${INK}" stroke-width="2"`), ellipse(48, 80, 13, 4, INK));
    p.push(`<path d="M96 76 L102 66 M96 66 h10" stroke="${INK}" stroke-width="4" stroke-linecap="round"/>`, ellipse(106, 82, 5, 5, "#ffe066", `stroke="${INK}" stroke-width="1.5"`));
    p.push(pet(sp, { scale: s, dx: tx, dy: ty, face: 6, look: [2, 0], eyes: "open", mouth: "open", legs: "none", arms: [toPet([98, 68]), toPet([102, 67])], armsBehind: true, shadow: 0, helmet: "#ffcc33", tail: 25 + 5 * Math.sin(t * 2) }, id));
    p.push(capsule(tx + 70 * s, ty + 108 * s, 70, 98, 9, sp.limb, sp.limbLine), ellipse(72, 99, 7, 4, sp.limb, `stroke="${sp.limbLine}" stroke-width="2"`));
  } else if (kind === "car") {
    const s = 0.62, tx = 14, ty = -6 + Math.sin(t * 2) * 1.2;
    const toPet = ([x, y]) => [(x - tx) / s, (y - ty) / s];
    const wave = Math.sin(t) > 0;
    p.push(puffs(t / TAU, 8, 104));
    p.push(pet(sp, { scale: s, dx: tx, dy: ty, face: 5, look: [2, 0], eyes: wave ? "happy" : "open", mouth: "open", legs: "none", arms: [wave ? toPet([26, 26]) : toPet([60, 70]), toPet([78, 66])], armsBehind: true, shadow: 0, tail: false }, id));
    p.push(`<path d="M8 100 Q6 74 22 70 L96 68 Q118 70 122 92 Q123 104 112 106 L16 106 Q8 106 8 100 Z" fill="#5ab0ff" stroke="${INK}" stroke-width="2.6" stroke-linejoin="round"/>`);
    p.push(`<path d="M12 92 L120 90" stroke="#3d8fd9" stroke-width="5"/>`, ellipse(116, 82, 4, 3.5, "#ffe066", `stroke="${INK}" stroke-width="1.4"`));
    p.push(`<path d="M78 68 L90 52 L96 52 L94 68" fill="#d8f0ff" stroke="${INK}" stroke-width="2" stroke-linejoin="round" opacity=".9"/>`);
    p.push(ellipse(80, 66, 6, 2.5, INK));
    p.push(wheel(34, 106, 12, ang, INK, "#ccc"), wheel(98, 106, 12, ang, INK, "#ccc"));
  } else if (kind === "skate") {
    const tilt = 4 * Math.sin(t);
    p.push(speedLines(t / TAU));
    p.push(`<g transform="rotate(${r1(tilt)} 64 116)">`);
    p.push(pet(sp, { scale: 0.9, dx: 6.4, dy: 0, face: 3, eyes: "happy", mouth: "w", legs: [[46, 117, -10], [84, 117, 10]], arms: [[14, 82 + 6 * Math.sin(t)], [114, 82 - 6 * Math.sin(t)]], shadow: 0, tail: 10 * Math.sin(t) }, id));
    p.push(`<rect x="18" y="113" width="92" height="7" rx="3.5" fill="#ff8a3d" stroke="${INK}" stroke-width="2.2"/>`);
    p.push(ellipse(34, 123, 4.5, 4.5, "#888", `stroke="${INK}" stroke-width="1.5"`), ellipse(94, 123, 4.5, 4.5, "#888", `stroke="${INK}" stroke-width="1.5"`), `</g>`);
  }
  return p.join("");
}

function frame(sp, o, id) {
  const body = [];
  if (o.poof) {
    body.push(cloud(o.poof[0], o.poof[1]));
  } else if (o.vehicle) {
    body.push(vehicle(sp, o.vehicle, o.t, id));
  } else if (o.back != null) {
    body.push(petBack(sp, o.back, id, o));
  } else {
    const extra = { ...o };
    if (o.bite != null) extra.held = food(sp, o.bite);
    if (o.crumbs) extra.front = `${ellipse(84, 68, 1.6, 1.6, "#33691e")}${ellipse(88, 74, 1.3, 1.3, "#33691e")}`;
    if (o.ball) extra.front = (extra.front ?? "") + ball(...o.ball);
    if (o.book != null) extra.front = (extra.front ?? "") + book(o.book);
    if (o.ladder) extra.held = ladderFlat(8);
    if (o.umbrella != null) body.push(umbrella(o.umbrella));
    body.push(pet(sp, extra, id));
  }
  return `${svgOpen(sp, id)}\n  ${body.join("\n  ")}\n</svg>\n`;
}

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
  let n = 0;
  for (const [name, { fps, frames }] of Object.entries(FRAMES)) {
    manifest.animations[name] = { fps, frames: [] };
    frames.forEach((opts, i) => {
      const file = `${name}-${i}.svg`;
      // Unique gradient ids per frame keep inline SVGs from clashing.
      writeFileSync(join(out, file), frame(sp, opts, `${id}${name}${i}`));
      manifest.animations[name].frames.push(file);
      n++;
    });
  }
  writeFileSync(join(out, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  console.log(`${id}: wrote ${n} frames`);
}
