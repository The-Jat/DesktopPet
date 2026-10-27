// Dashboard: manage reminders, notes, the pet and its behaviour.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];

let overview = null;
let editingId = null;

// ---------- helpers ----------

function showError(err) {
  const box = $("#error");
  box.textContent = err ? String(err) : "";
  box.hidden = !err;
}

async function call(cmd, args) {
  try {
    const result = await invoke(cmd, args);
    showError(null);
    return result;
  } catch (err) {
    showError(err);
    throw err;
  }
}

function el(tag, props = {}, ...children) {
  const node = document.createElement(tag);
  Object.assign(node, props);
  node.append(...children.filter((c) => c != null));
  return node;
}

const pad = (n) => String(n).padStart(2, "0");
const toDateInput = (d) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
const toTimeInput = (d) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;
/** Rust sends local times as "YYYY-MM-DDTHH:MM:SS" with no zone. */
const parseLocal = (s) => new Date(s);

function formatWhen(s) {
  const d = parseLocal(s);
  const today = new Date();
  const tomorrow = new Date(today);
  tomorrow.setDate(today.getDate() + 1);
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (d.toDateString() === today.toDateString()) return `Today ${time}`;
  if (d.toDateString() === tomorrow.toDateString()) return `Tomorrow ${time}`;
  return `${d.toLocaleDateString([], { weekday: "short", day: "numeric", month: "short", year: "numeric" })} ${time}`;
}

const DAY_NAMES = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

function describeRepeat(r) {
  switch (r.kind) {
    case "once": return "Once";
    case "daily": return "Every day";
    case "weekdays": return "Weekdays";
    case "weekly": return r.days.length ? `Every ${r.days.map((d) => DAY_NAMES[d]).join(", ")}` : "Weekly";
    case "everyMinutes": return `Every ${r.minutes} min`;
    default: return r.kind;
  }
}

// ---------- tabs ----------

for (const tab of $$(".tab")) {
  tab.addEventListener("click", () => {
    for (const t of $$(".tab")) t.classList.toggle("active", t === tab);
    for (const s of $$("main > section")) s.hidden = s.id !== `tab-${tab.dataset.tab}`;
  });
}

// ---------- reminders ----------

const form = $("#reminder-form");
const F = form.elements;

function updateRepeatFields() {
  const kind = F.repeat.value;
  $("#weekly-days").hidden = kind !== "weekly";
  $("#every-minutes").hidden = kind !== "everyMinutes";
}
F.repeat.addEventListener("change", updateRepeatFields);

function setFormTime(d) {
  F.date.value = toDateInput(d);
  F.time.value = toTimeInput(d);
}

for (const b of $$("[data-preset]")) {
  b.addEventListener("click", () => {
    const d = new Date();
    d.setSeconds(0, 0);
    if (b.dataset.preset === "10m") d.setMinutes(d.getMinutes() + 10);
    if (b.dataset.preset === "1h") d.setHours(d.getHours() + 1);
    if (b.dataset.preset === "tomorrow") {
      d.setDate(d.getDate() + 1);
      d.setHours(9, 0);
    }
    setFormTime(d);
  });
}

function resetForm() {
  editingId = null;
  form.reset();
  const soon = new Date();
  soon.setMinutes(soon.getMinutes() + 30, 0, 0);
  setFormTime(soon);
  $("#reminder-form-title").textContent = "New reminder";
  $("#reminder-submit").textContent = "Add reminder";
  $("#reminder-cancel").hidden = true;
  updateRepeatFields();
}

function readDraft() {
  const kind = F.repeat.value;
  let repeat = { kind };
  if (kind === "weekly") repeat.days = $$("#weekly-days input:checked").map((c) => Number(c.value));
  if (kind === "everyMinutes") repeat.minutes = Number(F.minutes.value);
  return {
    title: F.title.value,
    details: F.details.value,
    due: `${F.date.value}T${F.time.value}:00`,
    repeat,
    enabled: F.enabled.checked,
  };
}

function editReminder(r) {
  editingId = r.id;
  F.title.value = r.title;
  F.details.value = r.details;
  setFormTime(parseLocal(r.due));
  F.repeat.value = r.repeat.kind;
  for (const c of $$("#weekly-days input")) c.checked = (r.repeat.days || []).includes(Number(c.value));
  if (r.repeat.minutes) F.minutes.value = r.repeat.minutes;
  F.enabled.checked = r.enabled;
  $("#reminder-form-title").textContent = "Edit reminder";
  $("#reminder-submit").textContent = "Save changes";
  $("#reminder-cancel").hidden = false;
  updateRepeatFields();
  F.title.focus();
}

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const draft = readDraft();
  if (editingId == null) await call("add_reminder", { draft });
  else await call("update_reminder", { id: editingId, draft });
  resetForm();
  await refresh();
});
$("#reminder-cancel").addEventListener("click", resetForm);

function reminderStatus(r, now) {
  if (r.completedAt) return { text: "Done", muted: true };
  if (!r.enabled) return { text: "Off", muted: true };
  const fire = parseLocal(r.snoozedUntil || r.due);
  if (fire <= now) return { text: "Due now", due: true };
  if (r.snoozedUntil) return { text: `Snoozed until ${formatWhen(r.snoozedUntil)}` };
  return null;
}

function renderReminders() {
  const list = $("#reminder-list");
  list.replaceChildren();
  const now = new Date();
  const sorted = [...overview.reminders].sort((a, b) => {
    const rank = (r) => (r.completedAt ? 2 : r.enabled ? 0 : 1);
    return rank(a) - rank(b) || parseLocal(a.snoozedUntil || a.due) - parseLocal(b.snoozedUntil || b.due);
  });
  if (!sorted.length) list.append(el("li", { className: "empty" }, "No reminders yet. Add one above."));

  for (const r of sorted) {
    const status = reminderStatus(r, now);
    const badge = status && el("span", { className: `badge${status.due ? " due" : ""}`, textContent: status.text });
    const meta = `${formatWhen(r.due)} · ${describeRepeat(r.repeat)}${r.details ? " · " + r.details : ""}`;
    const toggle = el("button", { type: "button", textContent: r.enabled ? "Turn off" : "Turn on" });
    toggle.addEventListener("click", async () => {
      await call("set_reminder_enabled", { id: r.id, enabled: !r.enabled });
      await refresh();
    });
    const edit = el("button", { type: "button", textContent: "Edit" });
    edit.addEventListener("click", () => editReminder(r));
    const del = el("button", { type: "button", className: "danger", textContent: "Delete" });
    del.addEventListener("click", async () => {
      if (!confirm(`Delete "${r.title}"?`)) return;
      await call("delete_reminder", { id: r.id });
      if (editingId === r.id) resetForm();
      await refresh();
    });
    list.append(
      el(
        "li",
        { className: status && status.muted ? "muted" : "" },
        el("div", { className: "main" }, el("div", { className: "title" }, r.title, badge), el("div", { className: "meta", textContent: meta })),
        edit,
        toggle,
        del,
      ),
    );
  }
}

// ---------- notes ----------

$("#note-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const input = e.target.elements.text;
  await call("add_note", { text: input.value });
  input.value = "";
  await refresh();
});

function renderNotes() {
  const list = $("#note-list");
  list.replaceChildren();
  if (!overview.notes.length) list.append(el("li", { className: "empty" }, "No notes yet."));
  for (const n of [...overview.notes].reverse()) {
    const del = el("button", { type: "button", className: "danger", textContent: "Delete" });
    del.addEventListener("click", async () => {
      await call("delete_note", { id: n.id });
      await refresh();
    });
    list.append(
      el("li", {}, el("div", { className: "main" }, el("div", { className: "title", textContent: n.text }), el("div", { className: "meta", textContent: `Added ${formatWhen(n.created)}` })), del),
    );
  }
}

// ---------- pet & behaviour settings ----------

const settingsForms = [$("#pet-form"), $("#behaviour-form")];

function fillSettings(s) {
  for (const f of settingsForms) {
    for (const input of f.elements) {
      if (!input.name || !(input.name in s)) continue;
      if (input.type === "checkbox") input.checked = s[input.name];
      else input.value = s[input.name];
    }
  }
  updateOutputs();
}

function updateOutputs() {
  for (const out of $$("output[data-for]")) {
    const input = $(`[name="${out.dataset.for}"]`);
    out.textContent = input.name === "scale" ? `${Number(input.value).toFixed(1)}×` : `${input.value} px/s`;
  }
}

function readSettings() {
  const s = { ...overview.settings };
  for (const f of settingsForms) {
    for (const input of f.elements) {
      if (!input.name || !(input.name in s)) continue;
      if (input.type === "checkbox") s[input.name] = input.checked;
      else if (typeof s[input.name] === "number") s[input.name] = Number(input.value);
      else s[input.name] = input.value;
    }
  }
  return s;
}

async function saveSettings(patch = {}) {
  overview.settings = await call("save_settings", { settings: { ...readSettings(), ...patch } });
  fillSettings(overview.settings);
  renderPacks();
}

for (const f of settingsForms) {
  f.addEventListener("input", updateOutputs);
  f.addEventListener("change", () => saveSettings().catch(() => {}));
  f.addEventListener("submit", (e) => e.preventDefault());
}

function renderPacks() {
  const grid = $("#pack-grid");
  grid.replaceChildren();
  for (const p of overview.packs) {
    const card = el(
      "button",
      { type: "button", className: `pack${p.id === overview.settings.petId ? " selected" : ""}`, title: p.author ? `By ${p.author}` : "" },
      el("img", { src: p.preview, alt: "" }),
      p.name,
    );
    card.addEventListener("click", () => saveSettings({ petId: p.id }).catch(() => {}));
    grid.append(card);
  }
  const errors = $("#pack-errors");
  errors.textContent = overview.packErrors.length ? `Some pets could not be loaded:\n${overview.packErrors.join("\n")}` : "";
  errors.hidden = !overview.packErrors.length;
}

$("#reload-packs").addEventListener("click", async () => {
  await call("reload_packs");
  await refresh();
});

$("#say-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  await call("pet_say", { text: e.target.elements.text.value });
  e.target.elements.text.value = "";
});

const FEELINGS = { happy: "Happy", neutral: "Content", bored: "Bored", sad: "Sad", sleepy: "Sleepy" };

async function refreshMood() {
  if ($("#tab-pet").hidden) return;
  try {
    const st = await invoke("get_status");
    const rows = [
      ["Happiness", st.mood.happiness],
      ["Energy", st.mood.energy],
      ["Boredom", st.mood.boredom],
    ];
    const box = $("#mood");
    box.replaceChildren(
      el("div", {}, el("strong", { textContent: "Feeling" })),
      el("div", { textContent: FEELINGS[st.feeling] || st.feeling }),
      el("div"),
    );
    for (const [label, v] of rows) {
      const fill = el("span");
      fill.style.width = `${v}%`;
      box.append(el("div", { textContent: label }), el("div", { className: "bar" }, fill), el("div", { textContent: Math.round(v) }));
    }
  } catch {
    // Status is cosmetic; ignore transient failures.
  }
}
setInterval(refreshMood, 2000);

// ---------- load ----------

async function refresh() {
  overview = await call("get_overview");
  renderReminders();
  renderNotes();
  renderPacks();
  fillSettings(overview.settings);
  $("#user-pets-dir").textContent = overview.userPetsDir;
  $("#data-path").textContent = overview.dataPath;
  refreshMood();
}

listen("data-changed", () => refresh().catch(() => {}));
// Keep "Due now" badges current while the window is open.
setInterval(() => overview && renderReminders(), 30_000);

resetForm();
refresh().catch(() => {});
