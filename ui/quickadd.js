// Quick-add box: type a sentence, see how it is understood, press Enter.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const input = document.getElementById("text");
const preview = document.getElementById("preview");
const HINT = "Type a reminder. No time? It's saved as a note.";
const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

function describeRepeat(r) {
  switch (r.kind) {
    case "daily": return ", every day";
    case "weekdays": return ", every weekday";
    case "weekly": return r.days.length ? `, every ${r.days.map((d) => DAYS[d]).join(", ")}` : ", weekly";
    case "everyMinutes": return r.minutes % 60 === 0 ? `, every ${r.minutes / 60} h` : `, every ${r.minutes} min`;
    default: return "";
  }
}

function formatDue(s) {
  const d = new Date(s);
  const today = new Date();
  const tomorrow = new Date(today);
  tomorrow.setDate(today.getDate() + 1);
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (d.toDateString() === today.toDateString()) return `today ${time}`;
  if (d.toDateString() === tomorrow.toDateString()) return `tomorrow ${time}`;
  return `${d.toLocaleDateString([], { weekday: "short", day: "numeric", month: "short" })} ${time}`;
}

function setPreview(text, cls = "hint") {
  preview.textContent = text;
  preview.className = cls;
}

let seq = 0;
async function updatePreview() {
  const text = input.value;
  if (!text.trim()) return setPreview(HINT);
  const mine = ++seq;
  const p = await invoke("parse_quick", { text });
  if (mine !== seq) return; // a newer keystroke won
  if (p.due) setPreview(`⏰ “${p.title}” — ${formatDue(p.due)}${describeRepeat(p.repeat)}`, "reminder");
  else setPreview(`📝 No time found: will be saved as a note`, "note");
}

input.addEventListener("input", () => updatePreview().catch(() => {}));

document.getElementById("form").addEventListener("submit", async (e) => {
  e.preventDefault();
  try {
    await invoke("quick_add", { text: input.value });
    close();
  } catch (err) {
    setPreview(String(err), "error");
  }
});

function close() {
  input.value = "";
  setPreview(HINT);
  invoke("hide_quick_add");
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") close();
});

listen("quickadd://open", () => {
  input.value = "";
  setPreview(HINT);
  input.focus();
});
input.focus();
