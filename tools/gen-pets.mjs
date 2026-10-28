// Generates the built-in pet packs (SVG frames + manifest.json):
//
//   node tools/gen-pets.mjs
//
// Both pets share one chibi skeleton (big head, small round body, capsule
// limbs) and differ in colours and markings, so every animation exists for
// both. Animations follow the classic principles: anticipation, squash and
// stretch, follow-through (ears, head and tail lag the body), easing, and
// loops computed from a phase so the last frame flows into the first.
//
// It also writes tools/preview/index.html, which plays every animation at
// its real speed (open it in a browser after running this).

import { mkdirSync, writeFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const PETS = join(ROOT, "pets");
const INK = "#3a2f36";
const TAU = Math.PI * 2;
const r1 = (n) => Math.round(n * 10) / 10;
const sin = (t) => Math.sin(TAU * t);
const cos = (t) => Math.cos(TAU * t);
/** Smooth 0→1→0 over t in [0,1). */
const bump = (t) => 0.5 - 0.5 * cos(t);
/** Everything is drawn a little smaller than the canvas so nothing clips. */
const FIT = 0.9;

const SPECIES = {
  panda: {
    name: "Panda",
    fur: "#ffffff",
    furShade: "#e4e3ec",
    belly: "#fbfbfe",
    limb: "#332b31",
    limbLine: INK,
    ears: "round",
    earColor: "#332b31",
    earInner: "#5d5360",
    patches: true,
    band: true,
    nose: "#332b31",
    blush: "#ff8fab",
    tail: null,
    food: "bamboo",
  },
  cat: {
    name: "Ginger Cat",
    fur: "#fbb55e",
    furShade: "#ea9238",
    belly: "#fff2e0",
    limb: "#f6a548",
    limbLine: INK,
    ears: "pointy",
    earColor: "#fbb55e",
    earInner: "#ffb3c6",
    patches: false,
    band: false,
    stripes: "#d97a22",
    muzzle: "#fff2e0",
    nose: "#f06b85",
    blush: "#ff7f9c",
    tail: "#f09c3f",
    food: "fish",
  },
};

// ------------------------------------------------------------ primitives --

const svgOpen = (sp) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" width="128" height="128">
  <defs>
    <radialGradient id="fur" cx="40%" cy="30%" r="78%"><stop offset="0" stop-color="${sp.fur}"/><stop offset="0.7" stop-color="${sp.fur}"/><stop offset="1" stop-color="${sp.furShade}"/></radialGradient>
    <radialGradient id="pupil" cx="38%" cy="32%" r="75%"><stop offset="0" stop-color="#6a5c6e"/><stop offset="0.55" stop-color="#2a2129"/><stop offset="1" stop-color="#140f14"/></radialGradient>
    <radialGradient id="blush"><stop offset="0" stop-color="${sp.blush}" stop-opacity=".85"/><stop offset="1" stop-color="${sp.blush}" stop-opacity="0"/></radialGradient>
    <radialGradient id="hot"><stop offset="0" stop-color="#ff4d4d" stop-opacity=".9"/><stop offset="1" stop-color="#ff4d4d" stop-opacity="0"/></radialGradient>
    <radialGradient id="shadow"><stop offset="0" stop-color="#000" stop-opacity=".26"/><stop offset="1" stop-color="#000" stop-opacity="0"/></radialGradient>
  </defs>`;

/** A rounded limb from (x1,y1) to (x2,y2), outlined. */
function capsule(x1, y1, x2, y2, w, fill, line = INK) {
  const a = `x1="${r1(x1)}" y1="${r1(y1)}" x2="${r1(x2)}" y2="${r1(y2)}" stroke-linecap="round"`;
  return `<line ${a} stroke="${line}" stroke-width="${w + 4.6}"/><line ${a} stroke="${fill}" stroke-width="${w}"/>`;
}

const ellipse = (cx, cy, rx, ry, fill, extra = "") =>
  `<ellipse cx="${r1(cx)}" cy="${r1(cy)}" rx="${r1(rx)}" ry="${r1(ry)}" fill="${fill}" ${extra}/>`;

const text = (x, y, size, s, fill = INK, w = 700, extra = "") =>
  `<text x="${r1(x)}" y="${r1(y)}" font-family="Segoe UI, Arial, sans-serif" font-weight="${w}" font-size="${size}" fill="${fill}" ${extra}>${s}</text>`;

// ---------------------------------------------------------------- the pet --
//
// o = {
//   dx, dy, scale, rot:  whole-pet placement and tilt (around the feet)
//   sq: [sx, sy]         squash/stretch around the feet
//   sitting, bodyDy
//   headDy, headDx, tilt head offset and tilt (degrees, around the neck)
//   face, look: [x,y]    face shift (head turn) and pupil offset
//   eyes, mouth, blush   expression ("hot" blush when overheated)
//   ears: [l, r]         ear angles
//   arms: [[x,y],[x,y]]  paw positions; raised arms (or all, with armsBehind) go behind the head
//   legs                 "stand" | "sit" | "none" | [[x,y,rot],[x,y,rot]]; hang: [a, b]
//   tail                 tail angle (cat), false for none
//   held, front, behind  extra SVG
//   shadow               shadow size (0 = none)
//   helmet, glasses
// }

function feet(sp, list, soles) {
  return list
    .map(([x, y, rot]) => {
      if (soles) {
        return `<g transform="rotate(${r1(rot)} ${r1(x)} ${r1(y)})">${ellipse(x, y, 10, 11.5, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.3"`)}${ellipse(x, y + 2.6, 5, 4.2, "#ffb3c6")}${ellipse(x - 4.6, y - 5, 1.9, 2.2, "#ffb3c6")}${ellipse(x, y - 6.6, 1.9, 2.2, "#ffb3c6")}${ellipse(x + 4.6, y - 5, 1.9, 2.2, "#ffb3c6")}</g>`;
      }
      return ellipse(x, y, 11, 7.2, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.3" transform="rotate(${r1(rot)} ${r1(x)} ${r1(y)})"`);
    })
    .join("");
}

function eye(sp, cx, cy, kind, look) {
  const [lx, ly] = look;
  const big = sp.patches ? 5.5 : 6.3; // pupil size; the cat has no white patch, so it gets bigger pupils
  const lid = sp.patches ? "#fff" : INK;
  const lidFill = sp.patches ? sp.limb : sp.fur;
  const line = (d, w = 2.5) => `<path d="${d}" fill="none" stroke="${lid}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"/>`;
  const glossy = (rw, rp) => {
    const px = cx + lx, py = cy + ly;
    const sclera = sp.patches ? ellipse(cx, cy, rw, rw * 1.06, "#fff") : "";
    return (
      sclera +
      ellipse(px, py, rp, rp * 1.12, "url(#pupil)") +
      `<path d="M${r1(px - rp * 0.62)} ${r1(py + rp * 0.55)} q${r1(rp * 0.62)} ${r1(rp * 0.42)} ${r1(rp * 1.24)} 0" fill="none" stroke="#9a8ca6" stroke-width="1.2" stroke-linecap="round" opacity=".7"/>` +
      ellipse(px + rp * 0.38, py - rp * 0.42, rp * 0.44, rp * 0.44, "#fff") +
      ellipse(px - rp * 0.38, py + rp * 0.3, rp * 0.2, rp * 0.2, "#fff")
    );
  };
  switch (kind) {
    case "open":
    case "down":
      return glossy(7, big);
    case "wide":
      return glossy(8.2, 4.6);
    case "halfblink": {
      // Mid-blink: the eye squeezed to a soft sliver under a curved lid.
      const sclera = sp.patches ? ellipse(cx, cy + 1.5, 7, 3.4, "#fff") : "";
      return sclera + ellipse(cx + lx, cy + 1.6, big, big * 0.5, "url(#pupil)") + line(`M${cx - 7} ${cy} q7 -3.2 14 0`);
    }
    case "closed":
      return line(`M${cx - 5.5} ${cy} q5.5 5 11 0`) + line(cx < 64 ? `M${cx - 5.5} ${cy} l-2 -1.5` : `M${cx + 5.5} ${cy} l2 -1.5`, 2);
    case "happy":
      return line(`M${cx - 5.5} ${cy + 2} q5.5 -7.5 11 0`);
    case "sad":
      return glossy(6.6, 5.2) + line(cx < 64 ? `M${cx - 6} ${cy - 8} l10 3.2` : `M${cx + 6} ${cy - 8} l-10 3.2`, 2.3) + `<path d="M${cx - 4} ${cy + 5.5} q4 1.5 8 0" stroke="#bfe7ff" stroke-width="1.6" fill="none"/>`;
    case "half":
      return glossy(6.8, 5.2) + `<path d="M${cx - 8.5} ${cy - 9} h17 v8 h-17 z" fill="${lidFill}"/>` + line(`M${cx - 6.5} ${cy - 1} h13`, 2.3);
    case "squint":
      return line(cx < 64 ? `M${cx - 5} ${cy - 4.5} l8.5 4.5 l-8.5 4.5` : `M${cx + 5} ${cy - 4.5} l-8.5 4.5 l8.5 4.5`);
    case "star":
      return `<path d="M${cx} ${cy - 7.5} l2.2 5.3 l5.8 0.4 l-4.5 3.6 l1.6 5.7 l-5.1 -3.2 l-5.1 3.2 l1.6 -5.7 l-4.5 -3.6 l5.8 -0.4 z" fill="#ffd84d" stroke="${INK}" stroke-width="1.1" stroke-linejoin="round"/>`;
    case "heart":
      return `<path transform="translate(${cx} ${cy - 1}) scale(0.85)" d="M0 -3 c-4 -6 -12 -2 -8 4 l8 8 l8 -8 c4 -6 -4 -10 -8 -4z" fill="#ff5c7a" stroke="${INK}" stroke-width="1.2"/>`;
    default:
      return "";
  }
}

function mouth(nx, ny, kind) {
  const m = (d, fill = "none", w = 2.1) => `<path d="${d}" fill="${fill}" stroke="${INK}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round"/>`;
  switch (kind) {
    case "w": return m(`M${nx - 5.5} ${ny + 4.6} q2.75 3.3 5.5 0 q2.75 3.3 5.5 0`);
    case "smile": return m(`M${nx - 5.5} ${ny + 5} q5.5 5.5 11 0`);
    case "open": return m(`M${nx - 6.5} ${ny + 4.6} q6.5 11 13 0 z`, "#d9485f") + ellipse(nx, ny + 9.4, 3.4, 2, "#ff9aaa");
    case "o": return ellipse(nx, ny + 8.4, 3.2, 3.9, "#d9485f", `stroke="${INK}" stroke-width="1.9"`);
    case "small": return ellipse(nx, ny + 7.4, 2, 2.4, "#d9485f", `stroke="${INK}" stroke-width="1.7"`);
    case "frown": return m(`M${nx - 5.5} ${ny + 9.5} q5.5 -5.5 11 0`);
    case "flat": return m(`M${nx - 4.5} ${ny + 7} h9`);
    case "chew": return m(`M${nx - 4.5} ${ny + 6} q4.5 4.5 9 0 q-4.5 2 -9 0z`, "#d9485f");
    case "wavy": return m(`M${nx - 7.5} ${ny + 8} q1.9 -3 3.75 0 q1.9 3 3.75 0 q1.9 -3 3.75 0 q1.9 3 3.75 0`);
    case "yawn": return ellipse(nx, ny + 9, 5.2, 6.8, "#b9425a", `stroke="${INK}" stroke-width="1.9"`) + ellipse(nx, ny + 12.3, 3, 1.9, "#ff9aaa");
    case "grit": return m(`M${nx - 5.5} ${ny + 5.5} h11 v4 h-11 z`, "#fff", 1.7) + m(`M${nx - 1.8} ${ny + 5.5} v4 M${nx + 1.8} ${ny + 5.5} v4`, "none", 1.1);
    default: return "";
  }
}

function head(sp, o) {
  const p = [];
  const hy = (o.sitting ? 57 : 52) + (o.headDy ?? 0);
  const f = o.face ?? 0;
  const [el, er] = o.ears ?? [0, 0];
  if (!o.helmet) {
    if (sp.ears === "round") {
      p.push(`<g transform="rotate(${r1(el)} 38 ${hy - 20})">${ellipse(31 + f * 0.4, hy - 28, 14, 13.5, sp.earColor)}${ellipse(33.5 + f * 0.4, hy - 26, 7, 7, sp.earInner)}</g>`);
      p.push(`<g transform="rotate(${r1(er)} 90 ${hy - 20})">${ellipse(97 + f * 0.4, hy - 28, 14, 13.5, sp.earColor)}${ellipse(94.5 + f * 0.4, hy - 26, 7, 7, sp.earInner)}</g>`);
    } else {
      const ear = (x, s, rot) =>
        `<g transform="rotate(${r1(rot)} ${x} ${hy - 22})"><path d="M${x - 15 * s} ${hy - 15} Q${x - 10 * s} ${hy - 40} ${x - 6 * s} ${hy - 47} Q${x + 4 * s} ${hy - 36} ${x + 14 * s} ${hy - 26} Z" fill="${sp.earColor}" stroke="${INK}" stroke-width="2.5" stroke-linejoin="round"/><path d="M${x - 9 * s} ${hy - 20} Q${x - 7 * s} ${hy - 33} ${x - 5.5 * s} ${hy - 38} Q${x + 1 * s} ${hy - 31} ${x + 6 * s} ${hy - 26} Z" fill="${sp.earInner}"/></g>`;
      p.push(ear(36 + f * 0.4, 1, el), ear(92 + f * 0.4, -1, er));
    }
  }
  // Head: a wide, soft round shape.
  p.push(ellipse(64, hy, 40, 35, "url(#fur)", `stroke="${INK}" stroke-width="2.5"`));
  if (sp.stripes) {
    for (const [x, len] of [[55, 10], [64, 13], [73, 10]]) {
      p.push(`<path d="M${x + f * 0.6} ${hy - 34} v${len}" stroke="${sp.stripes}" stroke-width="4" stroke-linecap="round"/>`);
    }
    // Cheek fluff
    p.push(`<path d="M26.5 ${hy + 5} l-5 3 l5 2.5 l-4 3.5 l5.5 0.5" fill="${sp.fur}" stroke="${INK}" stroke-width="2" stroke-linejoin="round"/>`);
    p.push(`<path d="M101.5 ${hy + 5} l5 3 l-5 2.5 l4 3.5 l-5.5 0.5" fill="${sp.fur}" stroke="${INK}" stroke-width="2" stroke-linejoin="round"/>`);
  } else if (!o.helmet) {
    // A little tuft on top.
    p.push(`<path d="M60 ${hy - 34} q2 -7 7 -6 q-4 1 -3 6" fill="${sp.fur}" stroke="${INK}" stroke-width="2" stroke-linejoin="round"/>`);
  }
  const ex = 46 + f, ex2 = 82 + f, ey = hy + 3;
  if (sp.muzzle) p.push(ellipse(64 + f, hy + 16, 15, 10, sp.muzzle));
  if (sp.patches) {
    p.push(ellipse(ex, ey + 1, 11.5, 14, sp.limb, `transform="rotate(30 ${ex} ${ey + 1})"`));
    p.push(ellipse(ex2, ey + 1, 11.5, 14, sp.limb, `transform="rotate(-30 ${ex2} ${ey + 1})"`));
  }
  const look = o.look ?? [f * 0.25, 0];
  const eyes = o.eyes ?? "open";
  const eyeLook = eyes === "down" ? [look[0], 2.4] : look;
  p.push(eye(sp, ex + 1, ey, eyes, eyeLook), eye(sp, ex2 - 1, ey, eyes, eyeLook));
  if (o.glasses) {
    p.push(`<g fill="none" stroke="#7a5534" stroke-width="2"><circle cx="${ex + 1}" cy="${ey}" r="9.5"/><circle cx="${ex2 - 1}" cy="${ey}" r="9.5"/><path d="M${ex + 10.5} ${ey - 1} q${(ex2 - ex - 21) / 2} -4 ${ex2 - ex - 21} 0"/></g>`);
  }
  if (o.blush !== false) {
    const g = o.blush === "hot" ? "url(#hot)" : "url(#blush)";
    const k = o.blush === "hot" ? 1.35 : 1;
    p.push(ellipse(35 + f, hy + 16, 9 * k, 6 * k, g), ellipse(93 + f, hy + 16, 9 * k, 6 * k, g));
  }
  const nx = 64 + f, ny = hy + 13;
  if (sp.patches) {
    p.push(`<path d="M${nx - 4.8} ${ny - 2.4} q4.8 -2 9.6 0 q-1 4.4 -4.8 4.9 q-3.8 -0.5 -4.8 -4.9z" fill="${sp.nose}"/>` + ellipse(nx - 1.5, ny - 1.7, 1.3, 0.8, "#fff", `opacity=".75"`));
  } else {
    p.push(`<path d="M${nx - 3.8} ${ny - 2.4} h7.6 l-3.8 4.3 z" fill="${sp.nose}" stroke="${INK}" stroke-width="1.2" stroke-linejoin="round"/>`);
    for (const s of [-1, 1]) {
      for (const d of [-2, 3]) p.push(`<path d="M${nx + s * 13} ${ny + 3 + d} l${s * 14} ${d * 1.3 - 1}" stroke="${INK}" stroke-width="1.2" stroke-linecap="round"/>`);
    }
  }
  p.push(mouth(nx, ny, o.mouth ?? "w"));
  if (o.helmet) {
    p.push(`<path d="M${23 + f * 0.3} ${hy - 4} q41 -64 82 0 z" fill="${o.helmet}" stroke="${INK}" stroke-width="2.5" stroke-linejoin="round"/>`);
    p.push(`<path d="M30 ${hy - 10} q34 -14 68 0" fill="none" stroke="#fff" stroke-width="3" opacity=".55"/>`, ellipse(64, hy - 29, 4, 2.2, "#fff", `opacity=".8"`));
  }
  // Tilt around the neck, then offset.
  const tilt = o.tilt ?? 0;
  return `<g transform="translate(${r1(o.headDx ?? 0)} 0) rotate(${r1(tilt)} 64 ${hy + 28})">${p.join("")}</g>`;
}

function pet(sp, o) {
  const sitting = !!o.sitting;
  const by = (sitting ? 102 : 98) + (o.bodyDy ?? 0);
  const g = [];
  if (o.behind) g.push(o.behind);
  if (sp.tail && o.tail !== false) {
    const sw = Math.min(14, o.tail ?? 0);
    const d = `M84 ${by + 10} q24 -2 22 -24 q-1 -10 7 -15`;
    g.push(`<g transform="rotate(${r1(sw)} 84 ${by + 10})"><path d="${d}" fill="none" stroke="${INK}" stroke-width="12" stroke-linecap="round"/><path d="${d}" fill="none" stroke="${sp.tail}" stroke-width="7.6" stroke-linecap="round"/><path d="M104 ${by - 12} l4 -3 M106 ${by - 4} l5 -1" stroke="${sp.stripes}" stroke-width="3" stroke-linecap="round"/></g>`);
  }
  const legSpec = o.legs === "none" ? [] : Array.isArray(o.legs) ? o.legs : sitting ? [[41, 115, -18], [87, 115, 18]] : [[50, 118, 0], [78, 118, 0]];
  let legsSvg = feet(sp, legSpec, sitting && !Array.isArray(o.legs));
  if (o.hang) {
    legsSvg = o.hang
      .map((a, i) => {
        const hx = i ? 76 : 52, hy = by + 8;
        const rad = (a * Math.PI) / 180;
        const fx = hx + Math.sin(rad) * 19, fy = hy + Math.cos(rad) * 19;
        return capsule(hx, hy, fx, fy, 11.5, sp.limb, sp.limbLine) + ellipse(fx, fy + 2, 8.5, 7, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`) + ellipse(fx, fy + 4, 4, 2.5, "#ffb3c6");
      })
      .join("");
  }
  if (!sitting || o.hang) g.push(legsSvg);
  // Body: small and round, with a soft belly.
  g.push(ellipse(64, by, sitting ? 28 : 25, sitting ? 19.5 : 21, "url(#fur)", `stroke="${INK}" stroke-width="2.5"`));
  g.push(ellipse(64, by + 4, 14.5, 12, sp.belly));
  if (sp.band) g.push(`<path d="M39 ${by - 12} q25 -12 50 0 q-2 8 -6 10 q-19 -8 -38 0 q-4 -2 -6 -10z" fill="${sp.limb}"/>`);
  if (sitting && !o.hang) g.push(legsSvg);
  const sh = [[46, by - 9], [82, by - 9]];
  const arms = o.arms ?? [[38, by + 7], [90, by + 7]];
  const used = arms.map((a, i) => [a, i]).filter(([a]) => a && a[0] != null);
  // Raised arms go behind the head (so they never cover the face) with the paw in front.
  const behind = used.map(([[, y]]) => o.armsBehind || y < by - 22);
  const arm = ([[x, y], i]) => capsule(sh[i][0], sh[i][1], x, y, 11.5, sp.limb, sp.limbLine);
  const paw = ([[x, y]]) => ellipse(x, y, 7.2, 6.8, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.3"`);
  used.forEach((u, k) => behind[k] && g.push(arm(u)));
  g.push(head(sp, o));
  if (o.held) g.push(o.held);
  used.forEach((u, k) => g.push(behind[k] ? paw(u) : arm(u)));
  if (o.front) g.push(o.front);

  const [sx, sy] = o.sq ?? [1, 1];
  const ground = sitting ? 121 : 124;
  const squashed = `<g transform="translate(64 ${ground}) scale(${r1(sx * 1000) / 1000} ${r1(sy * 1000) / 1000}) translate(-64 -${ground})">${g.join("")}</g>`;
  const tx = o.dx ?? 0, ty = o.dy ?? 0, s = o.scale ?? 1, rot = o.rot ?? 0;
  const out = [];
  if (o.shadow !== 0) {
    const k = o.shadow ?? 1;
    out.push(ellipse(64 + tx, 124, 32 * k * s * sx, 5 * k, "url(#shadow)"));
  }
  out.push(`<g transform="translate(${r1(tx)} ${r1(ty)}) scale(${s}) rotate(${r1(rot)} 64 ${ground})">${squashed}</g>`);
  return out.join("");
}

/** Seen from behind, for climbing. `u` in [0,1): which paw is up. */
function petBack(sp, t, o = {}) {
  const up = bump(t); // 0..1: left paw high → right paw high
  const sway = sin(t) * 2;
  const lift = -Math.abs(cos(t)) * 1.5;
  const g = [];
  const by = 94;
  const lfy = 114 - (1 - up) * 12, rfy = 114 - up * 12;
  g.push(capsule(52, by + 12, 46, lfy, 12.5, sp.limb, sp.limbLine), capsule(76, by + 12, 82, rfy, 12.5, sp.limb, sp.limbLine));
  g.push(ellipse(45, lfy + 3, 9, 6.5, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`), ellipse(83, rfy + 3, 9, 6.5, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`));
  if (sp.tail) {
    const d = `M64 ${by + 18} q-2 14 10 18 q8 2 8 -6`;
    g.push(`<path d="${d}" fill="none" stroke="${INK}" stroke-width="11.5" stroke-linecap="round"/><path d="${d}" fill="none" stroke="${sp.tail}" stroke-width="7.2" stroke-linecap="round"/>`);
  }
  g.push(ellipse(64, by, 25, 23, "url(#fur)", `stroke="${INK}" stroke-width="2.5"`));
  if (!sp.tail) g.push(ellipse(64, by + 19, 6, 5.5, sp.fur, `stroke="${INK}" stroke-width="2"`)); // panda tail
  if (sp.band) g.push(`<path d="M39 ${by - 14} q25 -14 50 0 q0 10 -4 12 q-21 -9 -42 0 q-4 -2 -4 -12z" fill="${sp.limb}"/>`);
  if (sp.stripes) {
    for (const yy of [by - 6, by + 3, by + 12]) g.push(`<path d="M52 ${yy} q12 6 24 0" fill="none" stroke="${sp.stripes}" stroke-width="3.4" stroke-linecap="round"/>`);
  }
  const lpy = 46 - up * 36, rpy = 46 - (1 - up) * 36;
  g.push(capsule(42, by - 14, 20, lpy + 2, 11.5, sp.limb, sp.limbLine), capsule(86, by - 14, 108, rpy + 2, 11.5, sp.limb, sp.limbLine));
  if (sp.ears === "round") {
    g.push(ellipse(32, 25, 13.5, 13, sp.earColor), ellipse(96, 25, 13.5, 13, sp.earColor));
  } else {
    g.push(`<path d="M23 36 Q26 14 30 6 Q40 16 50 26 Z M105 36 Q102 14 98 6 Q88 16 78 26 Z" fill="${sp.earColor}" stroke="${INK}" stroke-width="2.5" stroke-linejoin="round"/>`);
  }
  g.push(ellipse(64, 50, 38, 33, "url(#fur)", `stroke="${INK}" stroke-width="2.5"`));
  if (sp.stripes) {
    for (const [x, len] of [[54, 11], [64, 14], [74, 11]]) g.push(`<path d="M${x} 21 v${len}" stroke="${sp.stripes}" stroke-width="4" stroke-linecap="round"/>`);
  } else {
    g.push(`<path d="M60 18 q2 -7 7 -6 q-4 1 -3 6" fill="${sp.fur}" stroke="${INK}" stroke-width="2" stroke-linejoin="round"/>`);
  }
  // Paws gripping, with claws.
  const claw = sp.limbLine === INK && sp.limb !== sp.limbLine ? INK : "#8a7f90";
  g.push(ellipse(20, lpy, 8.5, 7, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`), ellipse(108, rpy, 8.5, 7, sp.limb, `stroke="${sp.limbLine}" stroke-width="2.2"`));
  g.push(`<path d="M16 ${r1(lpy - 3)} v4 M20 ${r1(lpy - 4)} v4 M24 ${r1(lpy - 3)} v4 M104 ${r1(rpy - 3)} v4 M108 ${r1(rpy - 4)} v4 M112 ${r1(rpy - 3)} v4" stroke="${claw}" stroke-width="1.2" stroke-linecap="round"/>`);
  if (o.sweat) g.push(drop(104, 16, 1));
  return `<g transform="translate(${r1(sway)} ${r1(lift)})">${g.join("")}</g>`;
}

// ----------------------------------------------------------------- props --

const heart = (x, y, s = 1, op = 1) => `<path transform="translate(${r1(x)} ${r1(y)}) scale(${r1(s * 100) / 100})" d="M0 -3 c-4 -6 -12 -2 -8 4 l8 8 l8 -8 c4 -6 -4 -10 -8 -4z" fill="#ff5c7a" opacity="${r1(op)}"/>`;
const drop = (x, y, s = 1, op = 1) => `<path transform="translate(${r1(x)} ${r1(y)}) scale(${r1(s * 100) / 100})" d="M0 0 q-4 6 0 9 q4 -3 0 -9z" fill="#7ec8ff" opacity="${r1(op)}"/>`;
const note = (x, y, ch, rot = 0, op = 1) => `<g transform="rotate(${r1(rot)} ${r1(x)} ${r1(y)})">${text(x, y, 15, ch, "#6a4cc2", 700, `opacity="${r1(op)}"`)}</g>`;
const sparkle = (x, y, s = 1) => `<path transform="translate(${r1(x)} ${r1(y)}) scale(${r1(s * 100) / 100})" d="M0 -6 L1.5 -1.5 L6 0 L1.5 1.5 L0 6 L-1.5 1.5 L-6 0 L-1.5 -1.5 Z" fill="#ffd84d"/>`;
const zz = (x, y, s, op) => text(x, y, s, "z", "#5b6b8c", 700, `opacity="${r1(op)}"`);

function food(sp, bite) {
  if (sp.food === "bamboo") {
    const top = 62 + bite * 10;
    return `<g transform="rotate(-24 64 94)"><rect x="60" y="${r1(top)}" width="8.5" height="${r1(110 - top)}" rx="3" fill="#8bc34a" stroke="#33691e" stroke-width="1.6"/><path d="M60 86 h8.5 M60 99 h8.5" stroke="#33691e" stroke-width="1.6"/><path d="M68.5 90 q13 -7 18 2 q-11 3 -18 -2z" fill="#7cb342" stroke="#33691e" stroke-width="1.2"/></g>`;
  }
  const len = 28 - bite * 9;
  return `<g transform="rotate(-18 64 86)"><ellipse cx="${r1(64 + (28 - len) / 2)}" cy="86" rx="${r1(len / 2)}" ry="6.5" fill="#86bfe3" stroke="${INK}" stroke-width="1.6"/><path d="M${r1(78)} 86 l8 -6 v12 z" fill="#86bfe3" stroke="${INK}" stroke-width="1.6" stroke-linejoin="round"/><circle cx="${r1(57 + (28 - len) / 2)}" cy="84" r="1.4" fill="${INK}"/></g>`;
}

const ball = (x, y, rot = 0) => `<g transform="rotate(${r1(rot)} ${r1(x)} ${r1(y)})">${ellipse(x, y, 9.5, 9.5, "#ff7043", `stroke="${INK}" stroke-width="2"`)}<path d="M${r1(x - 9)} ${r1(y)} q9 6 18 0" fill="none" stroke="#fff" stroke-width="2"/></g>`;

function book(flip) {
  const f = flip > 0 ? `<path d="M0 -4 Q${r1(8 * flip)} ${r1(-4 - 14 * flip)} ${r1(16 * flip)} ${r1(-10 + 2 * (1 - flip))} L0 11 Z" fill="#fff" stroke="#bbb" stroke-width="1"/>` : "";
  return `<g transform="translate(64 93)"><path d="M-22 -10 L0 -6 L22 -10 L22 10 L0 14 L-22 10 Z" fill="#7a5cc9" stroke="${INK}" stroke-width="2" stroke-linejoin="round"/><path d="M-19 -8 L0 -4 L0 11 L-19 7 Z M19 -8 L0 -4 L0 11 L19 7 Z" fill="#fffaf0" stroke="#bbb" stroke-width="1"/><path d="M-15 -3 l11 2 M-15 1 l11 2 M15 -3 l-11 2 M15 1 l-11 2" stroke="#bbb" stroke-width="1.2"/>${f}</g>`;
}

function ladderFlat(y) {
  const rungs = [];
  for (let x = 16; x <= 112; x += 12) rungs.push(`M${x} ${r1(y - 6)} v12`);
  return `<path d="M8 ${r1(y - 6)} h112 M8 ${r1(y + 6)} h112" stroke="#7a4f24" stroke-width="5" stroke-linecap="round"/><path d="M8 ${r1(y - 6)} h112 M8 ${r1(y + 6)} h112" stroke="#c58b4e" stroke-width="3" stroke-linecap="round"/><path d="${rungs.join(" ")}" stroke="#a8743e" stroke-width="3"/>`;
}

function umbrella(sway) {
  const scallop = [];
  for (let i = 0; i < 6; i++) scallop.push(`q-8.33 -7 -16.67 0`);
  return `<g transform="rotate(${r1(sway)} 64 76)"><path d="M14 28 Q64 -18 114 28 ${scallop.join(" ")} Z" fill="#ff6b6b" stroke="${INK}" stroke-width="2.4" stroke-linejoin="round"/><path d="M64 5 Q52 14 47.3 28 M64 5 Q76 14 80.7 28" fill="none" stroke="#fff" stroke-width="4" opacity=".75"/><path d="M64 28 v8" stroke="${INK}" stroke-width="3.2" stroke-linecap="round"/><circle cx="64" cy="4" r="3" fill="${INK}"/></g>`;
}

function cloud(k, fade) {
  const pf = [[64, 70, 26], [40, 78, 18], [88, 78, 18], [52, 56, 17], [78, 56, 17], [64, 92, 18]];
  return (
    pf.map(([x, y, r]) => ellipse(64 + (x - 64) * k, 70 + (y - 70) * k, r * k, r * k, "#fff", `stroke="#cfd6e0" stroke-width="2.5" opacity="${r1(fade)}"`)).join("") +
    sparkle(24 + 6 * k, 30, k) + sparkle(104 - 6 * k, 40, k * 0.8) + sparkle(32, 110 - 8 * k, k * 0.7)
  );
}

function wheel(cx, cy, r, angle, tire = INK, hub = "#bbb") {
  const spokes = [0, 1, 2, 3].map((i) => {
    const a = angle + (i * Math.PI) / 4;
    return `M${r1(cx - Math.cos(a) * (r - 3))} ${r1(cy - Math.sin(a) * (r - 3))} L${r1(cx + Math.cos(a) * (r - 3))} ${r1(cy + Math.sin(a) * (r - 3))}`;
  });
  return `${ellipse(cx, cy, r, r, "none", `stroke="${tire}" stroke-width="4"`)}<path d="${spokes.join(" ")}" stroke="${hub}" stroke-width="1.6"/>${ellipse(cx, cy, 3, 3, hub, `stroke="${INK}" stroke-width="1"`)}`;
}

function puffs(t, x = 8, y = 100) {
  return [0, 1, 2]
    .map((i) => {
      const k = (t + i / 3) % 1;
      return ellipse(x - k * 8, y - k * 6, 3 + k * 5, 3 + k * 5, "#d6d9de", `opacity="${r1(0.85 - k * 0.8)}"`);
    })
    .join("");
}

const speedLines = (t) => [0, 1, 2].map((i) => `<path d="M${r1(4 + ((t * 20 + i * 9) % 18))} ${78 + i * 12} h14" stroke="#b9c3d1" stroke-width="2.5" stroke-linecap="round"/>`).join("");

// ------------------------------------------------------------ animations --
//
// Each animation is n frames of a function of t in [0,1).

const anim = (fps, n, f) => ({ fps, frames: Array.from({ length: n }, (_, i) => f(i / n, i)) });
const SIT_ARMS = [[36, 106], [92, 106]];

/** Breathing: a slow squash/stretch around the feet. */
const breathe = (t, k = 1) => [1 - 0.012 * k * sin(t), 1 + 0.024 * k * sin(t)];

// Feet swing ±STRIDE. A planted foot travels 2·STRIDE·FIT canvas px per half
// cycle (0.5 s at 12 fps), i.e. 40 px/s for a 128 px pet: the app's default
// walking speed (WALK_SPEED_OF_ART in src-tauri/src/state.rs), so the feet
// don't slide.
const STRIDE = 40 / (4 * FIT);

function walkPose(t, extra = {}) {
  const ph = sin(t), pc = cos(t);
  const contact = ph * ph; // 1 at the far points of the stride
  return {
    face: 3,
    look: [1.2, 0],
    dy: -2.4 * pc * pc,
    sq: [1 + 0.022 * contact, 1 - 0.032 * contact],
    legs: [
      [50 + STRIDE * ph, 118 - 5.5 * Math.max(0, pc), -8 * Math.max(0, pc)],
      [78 - STRIDE * ph, 118 - 5.5 * Math.max(0, -pc), 8 * Math.max(0, -pc)],
    ],
    arms: [[38 - 6 * ph, 105 - 1.5 * contact], [90 - 6 * ph, 105 - 1.5 * contact]],
    headDy: 1.1 * cos(2 * t - 0.12),
    tilt: 2.4 * ph,
    ears: [-7 * sin(2 * t - 0.2), 7 * sin(2 * t - 0.2)],
    tail: 10 * ph,
    ...extra,
  };
}

const FRAMES = {
  idle: anim(8, 16, (t, i) => ({
    sq: breathe(t),
    headDy: 1.1 * sin(t - 0.1),
    ears: [-5 * sin(t - 0.18), 5 * sin(t - 0.18)],
    look: [0.7 * sin(t), 0],
    tail: 9 * sin(t),
    eyes: i === 11 || i === 13 ? "halfblink" : i === 12 ? "closed" : "open",
  })),
  walk: anim(12, 12, (t) => walkPose(t)),
  sit: anim(6, 12, (t, i) => ({
    sitting: true,
    sq: breathe(t),
    headDy: 1 * sin(t - 0.1),
    arms: SIT_ARMS,
    ears: [-4 * sin(t - 0.2), 4 * sin(t - 0.2)],
    tail: 14 * sin(t),
    eyes: i === 7 || i === 9 ? "halfblink" : i === 8 ? "closed" : "open",
  })),
  sleep: anim(4, 12, (t) => {
    const b = 3 + 4.5 * bump(t);
    return {
      sitting: true,
      sq: breathe(t, 1.8),
      headDy: 3.5 + 1.2 * sin(t - 0.1),
      tilt: -6,
      arms: SIT_ARMS,
      eyes: "closed",
      mouth: "small",
      ears: [-8, 6],
      front:
        ellipse(70, 72, b, b, "#c7ebff", `opacity=".75" stroke="#8fc9ee" stroke-width="1.2"`) +
        ellipse(70 + b * 0.35, 72 - b * 0.35, b * 0.25, b * 0.25, "#fff", `opacity=".9"`) +
        zz(98 + 6 * t, 32 - 20 * t, 10 + 5 * t, Math.min(1, 1.6 - 1.6 * t)) +
        zz(98 + 6 * ((t + 0.5) % 1), 32 - 20 * ((t + 0.5) % 1), 10 + 5 * ((t + 0.5) % 1), Math.min(1, 1.6 - 1.6 * ((t + 0.5) % 1))),
    };
  }),
  happy: anim(14, 16, (t, i) => {
    // crouch, launch, airborne arc, land, settle. `a` is how far the arms are raised.
    const keys = [
      { dy: 0, sq: [1.03, 0.97], a: 0.12 },
      { dy: 0, sq: [1.06, 0.93], a: 0.05 },
      { dy: 0, sq: [1.08, 0.9], a: 0 },
      { dy: -1.5, sq: [0.97, 1.04], a: 0.3 },
      { dy: -5, sq: [0.94, 1.07], a: 0.6 },
      { dy: -9, sq: [0.97, 1.04], a: 0.85 },
      { dy: -11.5, sq: [1, 1], a: 1 },
      { dy: -12.5, sq: [1, 1], a: 1 },
      { dy: -11.5, sq: [1, 1], a: 1 },
      { dy: -8.5, sq: [0.98, 1.03], a: 0.9 },
      { dy: -4, sq: [0.96, 1.05], a: 0.7 },
      { dy: 0, sq: [1.07, 0.91], a: 0.5 },
      { dy: 0, sq: [1.1, 0.88], a: 0.35 },
      { dy: 0, sq: [1.03, 0.97], a: 0.25 },
      { dy: 0, sq: [0.99, 1.01], a: 0.18 },
      { dy: 0, sq: [1, 1], a: 0.14 },
    ];
    const k = keys[i];
    const air = k.dy < -2;
    const lerp = (x, y) => x + (y - x) * k.a;
    return {
      dy: k.dy,
      sq: k.sq,
      eyes: i >= 6 && i <= 8 ? "heart" : "happy",
      mouth: "open",
      arms: [[lerp(38, 24), lerp(106, 60)], [lerp(90, 104), lerp(106, 60)]],
      legs: air ? [[52, 116, -10], [76, 116, 10]] : undefined,
      ears: [8 * k.a - 6 * (1 - k.a), -8 * k.a + 6 * (1 - k.a)],
      headDy: (i >= 1 && i <= 2) || (i >= 11 && i <= 12) ? 2 : air ? -1 : 0,
      tail: air ? -18 : 6,
      shadow: Math.max(0.5, 1 + k.dy / 25),
      front: heart(106, 30 - 18 * t, 0.7 + 0.4 * t, Math.min(1, 2 - 2 * t)),
    };
  }),
  sad: anim(4, 8, (t) => ({
    sq: breathe(t, 0.8),
    eyes: "sad",
    mouth: "frown",
    ears: [-16 - 2 * sin(t), 16 + 2 * sin(t)],
    look: [0, 1.6],
    arms: [[42, 108], [86, 108]],
    headDy: 3 + 0.8 * sin(t - 0.15),
    tilt: -3,
    tail: 28,
    front: drop(40, 66 + 16 * t, 1, Math.min(1, 1.8 - 1.8 * t)),
  })),
  bored: anim(4, 12, (t, i) => {
    const yawn = i >= 6 && i <= 8;
    const paw = [0, 0, 0, 0, 0, 0.5, 1, 1, 1, 0.5, 0, 0][i]; // paw to the mouth and back
    return {
      sitting: true,
      sq: breathe(t, yawn ? 1.6 : 0.8),
      eyes: yawn ? "closed" : i === 9 || i === 5 ? "halfblink" : "half",
      mouth: yawn ? "yawn" : i === 5 || i === 9 ? "o" : "flat",
      arms: [[36 + 8 * paw, 106 - 32 * paw], SIT_ARMS[1]],
      headDy: 1.5 - 3.5 * paw,
      tilt: -4 + 8 * paw,
      tail: 10 * sin(t),
      front: yawn ? "" : text(100, 20, 16, "…", "#777", 700, `opacity="${r1(1 - paw)}"`),
    };
  }),
  surprised: anim(10, 6, (t, i) => {
    const keys = [
      { dy: 0, sq: [1.08, 0.9] },
      { dy: -6, sq: [0.93, 1.07] },
      { dy: -5, sq: [0.96, 1.04] },
      { dy: 0, sq: [1.08, 0.92] },
      { dy: 0, sq: [0.98, 1.02] },
      { dy: 0, sq: [1, 1] },
    ];
    return {
      ...keys[i],
      eyes: "wide",
      mouth: "o",
      arms: i === 0 ? [[34, 92], [94, 92]] : i === 1 ? [[26, 70], [102, 70]] : [[24, 60], [104, 60]],
      ears: i === 1 || i === 2 ? [-14, 14] : [-6, 6],
      tail: -32,
      shadow: Math.max(0.6, 1 + keys[i].dy / 30),
      front: drop(106, 30, 1.1) + text(14, 30 + (i % 2), 19, "!", "#ff6b6b"),
    };
  }),
  drag: anim(8, 8, (t) => {
    const s = sin(t);
    return {
      eyes: "squint",
      mouth: "o",
      rot: 5 * sin(t - 0.15),
      arms: [[24, 40], [104, 40]],
      legs: [[50 + 4 * s, 121, 14 * s], [78 + 4 * s, 121, 14 * s]],
      ears: [-10 * s, -10 * s],
      shadow: 0,
      tail: 30 * s,
    };
  }),
  alert: anim(10, 10, (t) => ({
    eyes: "wide",
    mouth: "open",
    dy: -2.5 * bump(2 * t % 1),
    sq: [1 + 0.02 * cos(2 * t), 1 - 0.03 * cos(2 * t)],
    arms: [[28 + 6 * cos(t), 58 + 12 * sin(t)], [90, 104]],
    ears: [-8 * sin(t), 8 * sin(t)],
    tail: 16 * sin(t),
    front: `<g transform="translate(108 ${r1(22 - 3 * bump(2 * t % 1))}) rotate(${r1(10 * sin(t))})">${ellipse(0, 0, 12, 12, "#ffcc33", `stroke="${INK}" stroke-width="2"`)}${text(-3.5, 7, 18, "!")}</g>`,
  })),
  eat: anim(8, 8, (t, i) => ({
    sitting: true,
    sq: breathe(2 * t % 1, 0.8),
    eyes: i % 4 === 3 ? "closed" : "happy",
    mouth: i % 2 ? "chew" : "o",
    arms: [[56, 92], [72, 92]],
    headDy: i % 2 ? 1.5 : 0,
    tilt: 3 * sin(t),
    tail: 12 * sin(t),
    bite: Math.min(1, t * 1.6),
    crumbs: t > 0.4,
  })),
  play: anim(12, 12, (t) => {
    // Toss the ball from the paw, watch it fly, reach up and catch it.
    const bx = 104 - 8 * sin(t / 2);
    const by = 92 - 72 * Math.sin(Math.PI * t);
    const reach = Math.sin(Math.PI * t) ** 0.8; // 0 at toss/catch, 1 at the top
    const hop = t > 0.25 && t < 0.75 ? -2.8 * Math.sin(Math.PI * (t - 0.25) / 0.5) : 0;
    return {
      dy: hop,
      sq: [1 - 0.01 * hop, 1 + 0.012 * -hop],
      eyes: by < 40 ? "wide" : "happy",
      mouth: "open",
      look: [2, by < 60 ? -2 : 1],
      arms: [[36 - 6 * reach, 100 - 34 * reach], [bx - 4 + 2 * reach, Math.max(by + 6, 92 - 30 * reach)]],
      tail: 16 * sin(t),
      shadow: 1 + hop / 25,
      ball: [bx, by, 360 * t],
    };
  }),
  sweat: anim(8, 8, (t) => ({
    sq: breathe(2 * t % 1, 1.6),
    eyes: "half",
    mouth: "wavy",
    blush: "hot",
    arms: [[38, 104], [100 + 5 * sin(2 * t), 46 + 4 * cos(2 * t)]],
    headDy: 1,
    tail: 8 * sin(t),
    front:
      drop(104, 34 + 14 * t, 1, Math.min(1, 1.6 - 1.6 * t)) +
      drop(24, 44 + 14 * ((t + 0.5) % 1), 0.9, Math.min(1, 1.6 - 1.6 * ((t + 0.5) % 1))) +
      `<path d="M${r1(42 + 2 * sin(t))} ${r1(14 - 4 * t)} q-4 -4 0 -8 M${r1(86 - 2 * sin(t))} ${r1(14 - 4 * ((t + 0.5) % 1))} q4 -4 0 -8" fill="none" stroke="#c9c9c9" stroke-width="2.5" stroke-linecap="round" opacity=".9"/>`,
  })),
  hungry: anim(4, 8, (t, i) => ({
    sq: breathe(t),
    eyes: i === 5 ? "closed" : "half",
    mouth: i % 2 ? "o" : "small",
    arms: [[56, 106], [72, 106]],
    headDy: 2 + sin(t - 0.1),
    tilt: 3 * sin(t),
    tail: 10 * sin(t),
    front:
      `<path d="M${r1(50 + 2 * sin(2 * t))} 101 q3 -3 6 0 q3 3 6 0 M${r1(66 - 2 * sin(2 * t))} 107 q3 -3 6 0 q3 3 6 0" fill="none" stroke="${INK}" stroke-width="1.6" opacity="${r1(0.35 + 0.35 * bump(2 * t % 1))}"/>` +
      `<g transform="translate(0 ${r1(2 * sin(t))})">${ellipse(96, 25, 2, 2, "#ddd")}${ellipse(102, 18, 3, 3, "#ddd")}${ellipse(111, 10, 11, 7.5, "#f6f6f6", `stroke="#ccc"`)}${ellipse(111, 10, 6, 3, "#9ccc65")}</g>` +
      drop(70, 76 + 3 * t, 0.6, 1 - t),
  })),
  jump: anim(8, 4, (t, i) => ({
    eyes: i % 2 ? "happy" : "closed",
    mouth: "open",
    sq: i % 2 ? [0.94, 1.07] : [1.02, 0.98],
    arms: [[24 + 2 * i, 58], [104 - 2 * i, 58]],
    legs: [[52, 113 - i, -18], [76, 113 - i, 18]],
    ears: [10, -10],
    shadow: 0.45,
    tail: -18 + 12 * i,
  })),
  perch: anim(6, 12, (t, i) => ({
    sitting: true,
    bodyDy: -12,
    headDy: -12 + 0.8 * sin(t - 0.1),
    hang: [20 * sin(t), -20 * sin(t + 0.28)],
    arms: [[34, 98], [94, 98]],
    look: [2.2 * sin(t / 1), 0],
    tilt: 4 * sin(t),
    eyes: i === 8 || i === 10 ? "halfblink" : i === 9 ? "closed" : "open",
    shadow: 0,
    tail: 14 * sin(t),
  })),
  carry: anim(12, 12, (t) => {
    const p = walkPose(t, { arms: [[34, 12], [94, 12]], armsBehind: true, mouth: "grit", tilt: 0 });
    // The ladder bounces a moment after the body.
    p.held = ladderFlat(10 - 2.2 * cos(t - 0.08) ** 2);
    p.arms = [[34, 12 - 2.2 * cos(t - 0.08) ** 2], [94, 12 - 2.2 * cos(t - 0.08) ** 2]];
    return p;
  }),
  float: anim(5, 10, (t) => ({
    dy: 22 + 2 * sin(t),
    scale: 0.8,
    dx: 12.8,
    rot: 4 * sin(t - 0.15),
    eyes: "happy",
    mouth: "w",
    arms: [[56, 10], [72, 10]],
    armsBehind: true,
    legs: [[50 + 3 * sin(t - 0.3), 121, 14 * sin(t - 0.3)], [78 + 3 * sin(t - 0.3), 121, 14 * sin(t - 0.3)]],
    ears: [6 * sin(t - 0.3), 6 * sin(t - 0.3)],
    shadow: 0,
    tail: 20 * sin(t - 0.3),
    umbrella: 4 * sin(t),
  })),
  poof: anim(12, 6, (t) => ({ poof: [0.45 + 0.6 * Math.sin(Math.PI * Math.min(1, t * 1.3)), Math.min(1, 1.9 - 1.9 * t)] })),
  rideBike: anim(12, 8, (t) => ({ vehicle: "bike", t })),
  rideMoto: anim(14, 8, (t) => ({ vehicle: "moto", t })),
  rideCar: anim(12, 8, (t) => ({ vehicle: "car", t })),
  rideSkate: anim(10, 8, (t) => ({ vehicle: "skate", t })),
  dance: anim(10, 16, (t, i) => {
    const beat = (4 * t) % 1;
    const hop = -4 * Math.max(0, Math.sin(Math.PI * beat * 1.4));
    const up = sin(t) > 0;
    return {
      rot: 7 * sin(t),
      dy: hop,
      sq: hop < -1 ? [0.96, 1.05] : beat > 0.75 ? [1.06, 0.93] : [1, 1],
      eyes: i % 8 === 6 ? "star" : "happy",
      mouth: "open",
      arms: up ? [[24 + 4 * sin(2 * t), 50], [92, 102]] : [[36, 102], [104 - 4 * sin(2 * t), 50]],
      legs: [[48 - 3 * sin(t), 118 - 4 * Math.max(0, sin(t)), -10 * sin(t)], [80 - 3 * sin(t), 118 - 4 * Math.max(0, -sin(t)), -10 * sin(t)]],
      tilt: -5 * sin(t),
      ears: [-10 * sin(2 * t), 10 * sin(2 * t)],
      tail: 24 * sin(t),
      shadow: 1 + hop / 30,
      front: note(12 + 4 * t, 40 - 26 * t, "♪", -12, Math.min(1, 2 - 2 * t)) + note(104 - 4 * ((t + 0.5) % 1), 36 - 24 * ((t + 0.5) % 1), "♫", 12, Math.min(1, 2 - 2 * ((t + 0.5) % 1))),
    };
  }),
  read: anim(5, 12, (t, i) => {
    const flipping = i >= 8 && i <= 10;
    return {
      sitting: true,
      sq: breathe(t, 0.8),
      eyes: i === 5 ? "closed" : "down",
      look: [flipping ? 2.2 : -2.2 + 4.4 * ((i % 4) / 3), 2.4],
      mouth: i === 11 ? "small" : "w",
      glasses: true,
      arms: [[48, 99], [80, 99]],
      headDy: 2,
      tilt: 2 * sin(t),
      tail: 8 * sin(t),
      book: flipping ? (i - 7) / 3 : 0,
    };
  }),
  climb: anim(8, 8, (t, i) => ({ back: t, sweat: i >= 6 })),
};

function vehicle(sp, kind, t) {
  const ang = -TAU * t * 2; // two wheel turns per loop
  const p = [];
  const bounce = Math.abs(sin(2 * t));
  if (kind === "bike") {
    const bb = [60, 104];
    const ped = [0, Math.PI].map((o) => [bb[0] + 9 * Math.cos(TAU * t + o), bb[1] + 9 * Math.sin(TAU * t + o)]);
    const s = 0.66, tx = 17, ty = -1 - 1.2 * bounce;
    const toPet = ([x, y]) => [(x - tx) / s, (y - ty) / s];
    const hips = [[tx + 52 * s, ty + 108 * s], [tx + 72 * s, ty + 108 * s]];
    p.push(speedLines(t));
    p.push(wheel(30, 104, 18, ang), wheel(100, 104, 18, ang));
    p.push(`<path d="M30 104 L60 104 L54 76 Z M54 78 L90 78 L60 104 M90 78 L100 104 M88 70 L90 78" fill="none" stroke="#3f9ad8" stroke-width="4" stroke-linejoin="round" stroke-linecap="round"/>`);
    p.push(`<path d="M84 68 h12" stroke="${INK}" stroke-width="4" stroke-linecap="round"/>`, ellipse(52, 73, 9, 3.5, INK));
    p.push(capsule(hips[1][0], hips[1][1], ped[1][0], ped[1][1], 8.5, sp.limb, sp.limbLine));
    p.push(pet(sp, { scale: s, dx: tx, dy: ty, face: 5, look: [2, 0], eyes: "happy", mouth: "open", legs: "none", arms: [toPet([90, 68]), toPet([95, 68])], armsBehind: true, shadow: 0, tail: 15, ears: [8, 14], tilt: 2 * sin(2 * t) }));
    p.push(capsule(hips[0][0], hips[0][1], ped[0][0], ped[0][1], 8.5, sp.limb, sp.limbLine));
    p.push(ellipse(ped[0][0], ped[0][1], 6, 3, "#555"), ellipse(ped[1][0], ped[1][1], 6, 3, "#555"));
  } else if (kind === "moto") {
    const s = 0.64, tx = 17, ty = -3 - 0.8 * bounce;
    const toPet = ([x, y]) => [(x - tx) / s, (y - ty) / s];
    p.push(puffs(t, 16, 100));
    p.push(wheel(32, 106, 16, ang, INK, "#999"), wheel(100, 106, 16, ang, INK, "#999"));
    p.push(`<path d="M20 96 h20" stroke="#888" stroke-width="5" stroke-linecap="round"/>`);
    p.push(`<path d="M30 92 Q34 78 52 80 L86 76 Q100 76 104 90 L96 94 L64 98 Q46 100 30 92 Z" fill="#ef5350" stroke="${INK}" stroke-width="2.4" stroke-linejoin="round"/>`);
    p.push(ellipse(70, 79, 16, 7, "#e53935", `stroke="${INK}" stroke-width="2"`), ellipse(48, 80, 13, 4, INK));
    p.push(`<path d="M96 76 L102 66 M96 66 h10" stroke="${INK}" stroke-width="4" stroke-linecap="round"/>`, ellipse(106, 82, 5, 5, "#ffe066", `stroke="${INK}" stroke-width="1.5"`));
    p.push(pet(sp, { scale: s, dx: tx, dy: ty, face: 6, look: [2, 0], eyes: "open", mouth: "open", legs: "none", arms: [toPet([98, 68]), toPet([102, 67])], armsBehind: true, shadow: 0, helmet: "#ffcc33", tail: 25 + 5 * sin(2 * t) }));
    p.push(capsule(tx + 70 * s, ty + 108 * s, 70, 98, 8.5, sp.limb, sp.limbLine), ellipse(72, 99, 7, 4, sp.limb, `stroke="${sp.limbLine}" stroke-width="2"`));
  } else if (kind === "car") {
    const s = 0.6, tx = 15, ty = -5 - 1.2 * bounce;
    const toPet = ([x, y]) => [(x - tx) / s, (y - ty) / s];
    const wave = t < 0.5;
    const wy = 26 + 6 * sin(2 * t);
    p.push(puffs(t, 10, 104));
    p.push(pet(sp, { scale: s, dx: tx, dy: ty, face: 5, look: [2, 0], eyes: wave ? "happy" : "open", mouth: "open", legs: "none", arms: [wave ? toPet([26, wy]) : toPet([60, 70]), toPet([78, 66])], armsBehind: true, shadow: 0, tail: false, ears: [6 * sin(2 * t), -6 * sin(2 * t)] }));
    const cy = -1.2 * bounce;
    p.push(`<g transform="translate(0 ${r1(cy)})"><path d="M10 100 Q8 74 24 70 L96 68 Q117 70 120 92 Q121 104 110 106 L18 106 Q10 106 10 100 Z" fill="#5ab0ff" stroke="${INK}" stroke-width="2.5" stroke-linejoin="round"/><path d="M14 92 L118 90" stroke="#3d8fd9" stroke-width="5"/>${ellipse(114, 82, 4, 3.5, "#ffe066", `stroke="${INK}" stroke-width="1.4"`)}<path d="M78 68 L89 53 L95 53 L93 68" fill="#d8f0ff" stroke="${INK}" stroke-width="2" stroke-linejoin="round" opacity=".9"/>${ellipse(80, 66, 6, 2.5, INK)}</g>`);
    p.push(wheel(35, 106, 12, ang, INK, "#ccc"), wheel(96, 106, 12, ang, INK, "#ccc"));
  } else if (kind === "skate") {
    const tilt = 4 * sin(t);
    p.push(speedLines(t));
    p.push(`<g transform="rotate(${r1(tilt)} 64 114)">`);
    p.push(pet(sp, { scale: 0.86, dx: 9, dy: -5, face: 3, eyes: "happy", mouth: "w", legs: [[46, 118, -10], [84, 118, 10]], arms: [[16, 84 + 6 * sin(t)], [112, 84 - 6 * sin(t)]], shadow: 0, tail: 10 * sin(t), tilt: -tilt, ears: [8 * sin(t), 8 * sin(t)] }));
    p.push(`<rect x="20" y="108" width="88" height="7" rx="3.5" fill="#ff8a3d" stroke="${INK}" stroke-width="2.2"/>`);
    p.push(wheel(35, 119, 4.5, ang, INK, "#999"), wheel(93, 119, 4.5, ang, INK, "#999"), `</g>`);
  }
  return p.join("");
}

function frame(sp, o) {
  const body = [];
  if (o.poof) {
    body.push(cloud(o.poof[0], o.poof[1]));
  } else if (o.vehicle) {
    body.push(vehicle(sp, o.vehicle, o.t));
  } else if (o.back != null) {
    body.push(petBack(sp, o.back, o));
  } else {
    const extra = { ...o };
    if (o.bite != null) extra.held = food(sp, o.bite);
    if (o.crumbs) extra.front = `${ellipse(84, 70, 1.6, 1.6, "#33691e")}${ellipse(88, 76, 1.3, 1.3, "#33691e")}`;
    if (o.ball) extra.front = (extra.front ?? "") + ball(...o.ball);
    if (o.book != null) extra.front = (extra.front ?? "") + book(o.book);
    if (o.umbrella != null) body.push(umbrella(o.umbrella));
    body.push(pet(sp, extra));
  }
  // Shrink a little around the floor point so nothing touches the edges.
  return `${svgOpen(sp)}\n  <g transform="translate(64 125) scale(${FIT}) translate(-64 -125)">\n  ${body.join("\n  ")}\n  </g>\n</svg>\n`;
}

const preview = {};
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
  preview[id] = {};
  let n = 0;
  for (const [name, { fps, frames }] of Object.entries(FRAMES)) {
    manifest.animations[name] = { fps, frames: [] };
    preview[id][name] = { fps, frames: [] };
    frames.forEach((opts, i) => {
      const file = `${name}-${i}.svg`;
      const svg = frame(sp, opts);
      writeFileSync(join(out, file), svg);
      manifest.animations[name].frames.push(file);
      preview[id][name].frames.push(`data:image/svg+xml;base64,${Buffer.from(svg).toString("base64")}`);
      n++;
    });
  }
  writeFileSync(join(out, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  console.log(`${id}: wrote ${n} frames`);
}

// A page that plays every animation at its real speed.
const pdir = join(ROOT, "tools", "preview");
mkdirSync(pdir, { recursive: true });
writeFileSync(
  join(pdir, "index.html"),
  `<!doctype html><meta charset="utf-8"><title>DesktopPet animations</title>
<style>body{font:13px Segoe UI,sans-serif;background:#eef1f6;margin:16px}h2{margin:18px 0 6px}.grid{display:flex;flex-wrap:wrap;gap:8px}.cell{background:#fff;border-radius:10px;padding:6px;text-align:center;box-shadow:0 1px 3px #0002}img{width:128px;height:128px;display:block}label{margin-left:12px}</style>
<label>Size <input id="size" type="range" min="64" max="256" value="128"></label>
<div id="root"></div>
<script>
const DATA = ${JSON.stringify(preview)};
const root = document.getElementById("root");
for (const [pet, anims] of Object.entries(DATA)) {
  const h = document.createElement("h2"); h.textContent = pet; root.append(h);
  const grid = document.createElement("div"); grid.className = "grid"; root.append(grid);
  for (const [name, a] of Object.entries(anims)) {
    const cell = document.createElement("div"); cell.className = "cell";
    const img = document.createElement("img"); cell.append(img, name + " (" + a.frames.length + " @ " + a.fps + "fps)"); grid.append(cell);
    const start = performance.now();
    (function tick(now) { img.src = a.frames[Math.floor(((now - start) / 1000) * a.fps) % a.frames.length]; requestAnimationFrame(tick); })(start);
  }
}
document.getElementById("size").oninput = (e) => document.querySelectorAll("img").forEach((i) => (i.style.width = i.style.height = e.target.value + "px"));
</script>
`,
);
console.log(`preview: ${join("tools", "preview", "index.html")}`);
