// Pet window: draws the sprite and speech bubble. All decisions (where to
// walk, which animation, what to say) come from Rust as events.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const stage = document.getElementById("stage");
const sprite = document.getElementById("sprite");
const spriteImg = document.getElementById("sprite-img");
const ladder = document.getElementById("ladder");
const bubble = document.getElementById("bubble");
const bubbleTitle = document.getElementById("bubble-title");
const bubbleText = document.getElementById("bubble-text");
const bubbleActions = document.getElementById("bubble-actions");

let pack = null;
let frame = null;
let animName = null;
let frameIndex = 0;
let frameTimer = null;

// Extra motion layered over the frames (see pet.css), so even a short frame
// cycle breathes, bobs and wobbles.
const MOTION = {
  idle: "breathe",
  sit: "breathe",
  sad: "breathe",
  bored: "breathe",
  hungry: "breathe",
  perch: "breathe",
  read: "breathe",
  sleep: "sleep",
  walk: "bob",
  carry: "bob",
  climb: "wobble",
  jump: "stretch",
  float: "swing",
  dance: "groove",
  rideBike: "bob",
  rideMoto: "rumble",
  rideCar: "rumble",
  rideSkate: "rock",
  sweat: "shiver",
};

function playAnimation(name) {
  if (!pack || name === animName) return;
  animName = name;
  frameIndex = 0;
  clearTimeout(frameTimer);
  step();
}

function step() {
  const anim = pack.animations[animName];
  if (!anim) return;
  spriteImg.src = anim.frames[frameIndex % anim.frames.length];
  frameIndex += 1;
  if (anim.frames.length > 1) {
    frameTimer = setTimeout(step, 1000 / anim.fps);
  }
}

function applyFrame(f) {
  if (!f) return;
  frame = f;
  stage.style.width = `${f.windowWidth}px`;
  stage.style.height = `${f.windowHeight}px`;
  sprite.style.left = `${f.spriteLeft}px`;
  sprite.style.top = `${f.spriteTop}px`;
  sprite.style.width = `${f.spriteWidth}px`;
  sprite.style.height = `${f.spriteHeight}px`;
  const mirror = pack && pack.facing !== "none" && f.facing !== pack.facing;
  spriteImg.classList.toggle("flip", mirror);
  sprite.dataset.motion = MOTION[f.anim] || "";
  if (f.ladder) {
    Object.assign(ladder.style, {
      left: `${f.ladder.left}px`,
      top: `${f.ladder.top}px`,
      width: `${f.ladder.width}px`,
      height: `${f.ladder.height}px`,
    });
  }
  ladder.hidden = !f.ladder;
  playAnimation(f.anim);
  placeBubble();
}

function setPack(p) {
  pack = p;
  animName = null;
  if (frame) applyFrame(frame);
}

function placeBubble() {
  if (!frame || bubble.hidden) return;
  const width = bubble.offsetWidth;
  const center = frame.spriteLeft + frame.spriteWidth / 2;
  const left = Math.max(4, Math.min(center - width / 2, frame.windowWidth - width - 4));
  bubble.style.left = `${left}px`;
  bubble.style.bottom = `${frame.windowHeight - frame.spriteTop + 4}px`;
}

function showBubble(b) {
  if (!b) {
    bubble.hidden = true;
    return;
  }
  bubble.classList.toggle("reminder", b.kind === "reminder");
  bubble.classList.toggle("nudge", b.kind === "nudge");
  bubbleTitle.textContent = b.kind === "reminder" ? `⏰ ${b.title}` : b.title || "";
  bubbleText.textContent = b.text;
  const actions = b.actions || [];
  bubbleActions.replaceChildren(
    ...actions.map((a, i) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.textContent = a.label;
      if (i === 0) btn.classList.add("primary");
      btn.addEventListener("click", () => invoke("bubble_action", { id: a.id }).catch(report));
      return btn;
    }),
  );
  bubbleActions.hidden = actions.length === 0;
  bubble.hidden = false;
  placeBubble();
}

// Pressing the sprite: Rust tracks the global cursor and decides between a
// click (poke) and a drag, so the pet can be dragged faster than this small
// window could follow mouse events.
sprite.addEventListener("pointerdown", (e) => {
  if (e.button !== 0) return;
  e.preventDefault();
  invoke("pet_pointer_down");
});
window.addEventListener("pointerup", (e) => {
  if (e.button === 0) invoke("pet_pointer_up");
});
sprite.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  invoke("pet_context_menu").catch(report);
});
document.addEventListener("contextmenu", (e) => e.preventDefault());

// Report script errors to the terminal; the window may be too small to show them.
const report = (msg) => invoke("log_from_page", { msg: String(msg) }).catch(() => {});
window.addEventListener("error", (e) => report(`error: ${e.message} (${e.filename}:${e.lineno})`));
window.addEventListener("unhandledrejection", (e) => report(`error: ${e.reason}`));

listen("pet://frame", (e) => applyFrame(e.payload));
listen("pet://bubble", (e) => showBubble(e.payload));
listen("pet://pack", (e) => setPack(e.payload));

// Retry briefly in case the page loads before the app finished starting.
async function init(attempt = 0) {
  let data;
  try {
    data = await invoke("pet_init");
  } catch (err) {
    if (attempt < 20) return setTimeout(() => init(attempt + 1), 250);
    return report(`pet_init failed: ${err}`);
  }
  if (data.debug) {
    // Tint the page so it is obvious whether the webview draws at all.
    document.body.style.background = "rgba(255, 0, 0, 0.35)";
    report(`loaded; page ${innerWidth}x${innerHeight} @${devicePixelRatio}x, frame ${JSON.stringify(data.frame)}`);
  }
  setPack(data.pack);
  applyFrame(data.frame);
  showBubble(data.bubble);
}
init();
