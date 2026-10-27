// Pet window: draws the sprite and speech bubble. All decisions (where to
// walk, which animation, what to say) come from Rust as events.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const stage = document.getElementById("stage");
const sprite = document.getElementById("sprite");
const bubble = document.getElementById("bubble");
const bubbleTitle = document.getElementById("bubble-title");
const bubbleText = document.getElementById("bubble-text");
const bubbleActions = document.getElementById("bubble-actions");

let pack = null;
let frame = null;
let animName = null;
let frameIndex = 0;
let frameTimer = null;
let reminderId = null;

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
  sprite.src = anim.frames[frameIndex % anim.frames.length];
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
  sprite.style.width = `${f.spriteWidth}px`;
  sprite.style.height = `${f.spriteHeight}px`;
  const mirror = pack && pack.facing !== "none" && f.facing !== pack.facing;
  sprite.classList.toggle("flip", mirror);
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
  bubble.style.bottom = `${frame.spriteHeight + 4}px`;
}

function showBubble(b) {
  if (!b) {
    bubble.hidden = true;
    reminderId = null;
    return;
  }
  const isReminder = b.kind === "reminder";
  reminderId = isReminder ? b.reminderId : null;
  bubble.classList.toggle("reminder", isReminder);
  bubbleTitle.textContent = isReminder ? `⏰ ${b.title}` : "";
  bubbleText.textContent = b.text;
  bubbleActions.hidden = !isReminder;
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
  invoke("open_dashboard");
});
document.addEventListener("contextmenu", (e) => e.preventDefault());

document.getElementById("done").addEventListener("click", () => {
  if (reminderId != null) invoke("reminder_done", { id: reminderId });
});
document.getElementById("snooze").addEventListener("click", () => {
  if (reminderId != null) invoke("reminder_snooze", { id: reminderId, minutes: null });
});

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
