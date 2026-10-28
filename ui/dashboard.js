// Dashboard: reminders, notes, focus, history, the pet and every setting.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;

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

let noticeTimer = null;
function showNotice(text) {
  const box = $("#notice");
  box.textContent = text;
  box.hidden = false;
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (box.hidden = true), 6000);
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

function button(text, onClick, className = "") {
  const b = el("button", { type: "button", textContent: text, className });
  b.addEventListener("click", onClick);
  return b;
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
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  if (d.toDateString() === today.toDateString()) return `Today ${time}`;
  if (d.toDateString() === tomorrow.toDateString()) return `Tomorrow ${time}`;
  if (d.toDateString() === yesterday.toDateString()) return `Yesterday ${time}`;
  return `${d.toLocaleDateString([], { weekday: "short", day: "numeric", month: "short", year: "numeric" })} ${time}`;
}

const DAY_NAMES = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

function describeRepeat(r) {
  switch (r.kind) {
    case "once": return "Once";
    case "daily": return "Every day";
    case "weekdays": return "Weekdays";
    case "weekly": return r.days.length ? `Every ${r.days.map((d) => DAY_NAMES[d]).join(", ")}` : "Weekly";
    case "everyMinutes": return r.minutes % 60 === 0 ? `Every ${r.minutes / 60} h` : `Every ${r.minutes} min`;
    default: return r.kind;
  }
}

const category = (id) => overview.categories.find((c) => c.id === id);

function categoryChip(id) {
  const c = id != null && category(id);
  if (!c) return null;
  const chip = el("span", { className: "chip", textContent: c.name });
  chip.style.setProperty("--chip", c.color);
  return chip;
}

const matches = (text, query) => text.toLowerCase().includes(query.trim().toLowerCase());

// ---------- tabs ----------

function openTab(name) {
  for (const t of $$(".tab")) t.classList.toggle("active", t.dataset.tab === name);
  for (const s of $$("main > section")) s.hidden = s.id !== `tab-${name}`;
  if (name === "history") refreshHistory();
  refreshStatus();
}
for (const tab of $$(".tab")) tab.addEventListener("click", () => openTab(tab.dataset.tab));

// ---------- categories (selects, filters, manager) ----------

function renderCategoryControls() {
  for (const sel of $$(".category-select")) {
    const keep = sel.value;
    sel.replaceChildren(el("option", { value: "", textContent: "No category" }), ...overview.categories.map((c) => el("option", { value: String(c.id), textContent: c.name })));
    sel.value = overview.categories.some((c) => String(c.id) === keep) ? keep : "";
  }
  for (const sel of $$(".category-filter")) {
    const keep = sel.value;
    sel.replaceChildren(
      el("option", { value: "", textContent: "All categories" }),
      el("option", { value: "none", textContent: "No category" }),
      ...overview.categories.map((c) => el("option", { value: String(c.id), textContent: c.name })),
    );
    sel.value = [...sel.options].some((o) => o.value === keep) ? keep : "";
  }

  const list = $("#category-list");
  list.replaceChildren();
  if (!overview.categories.length) list.append(el("li", { className: "empty" }, "No categories yet."));
  for (const c of overview.categories) {
    const name = el("input", { value: c.name, maxLength: 40, className: "grow" });
    const color = el("input", { type: "color", value: c.color });
    const save = () => call("update_category", { id: c.id, name: name.value, color: color.value }).then(refresh).catch(() => {});
    name.addEventListener("change", save);
    color.addEventListener("change", save);
    const del = button("Delete", async () => {
      if (!confirm(`Delete category "${c.name}"? Its reminders and notes are kept, without a category.`)) return;
      await call("delete_category", { id: c.id });
      await refresh();
    }, "danger");
    list.append(el("li", {}, color, name, del));
  }
}

$("#category-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const f = e.target.elements;
  await call("add_category", { name: f.name.value, color: f.color.value });
  f.name.value = "";
  await refresh();
});

const filterCategory = (value, id) => value === "" || (value === "none" ? id == null : String(id) === value);
const selectedCategory = (sel) => (sel.value === "" ? null : Number(sel.value));

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

function setRepeat(r) {
  F.repeat.value = r.kind;
  for (const c of $$("#weekly-days input")) c.checked = (r.days || []).includes(Number(c.value));
  if (r.minutes) F.minutes.value = r.minutes;
  updateRepeatFields();
}

// Typing a sentence fills in the form.
let quickSeq = 0;
$("#quick").addEventListener("input", async (e) => {
  const text = e.target.value;
  const preview = $("#quick-preview");
  if (!text.trim()) {
    preview.textContent = "Fills in the form below as you type.";
    preview.className = "hint small";
    return;
  }
  const mine = ++quickSeq;
  const p = await invoke("parse_quick", { text });
  if (mine !== quickSeq) return;
  F.title.value = p.title;
  setRepeat(p.repeat);
  if (p.due) {
    setFormTime(parseLocal(p.due));
    preview.textContent = `Understood: ${formatWhen(p.due)} · ${describeRepeat(p.repeat)}`;
    preview.className = "hint small ok";
  } else {
    preview.textContent = "No time found yet. Try “in 20 min”, “at 5pm” or “tomorrow”.";
    preview.className = "hint small warn";
  }
});

function resetForm() {
  editingId = null;
  const keepCategory = F.category.value;
  form.reset();
  F.category.value = keepCategory;
  const soon = new Date();
  soon.setMinutes(soon.getMinutes() + 30, 0, 0);
  setFormTime(soon);
  $("#quick").value = "";
  $("#quick-preview").textContent = "Fills in the form below as you type.";
  $("#quick-preview").className = "hint small";
  $("#reminder-form-title").textContent = "New reminder";
  $("#reminder-submit").textContent = "Add reminder";
  $("#reminder-cancel").hidden = true;
  updateRepeatFields();
}

function readDraft() {
  const kind = F.repeat.value;
  const repeat = { kind };
  if (kind === "weekly") repeat.days = $$("#weekly-days input:checked").map((c) => Number(c.value));
  if (kind === "everyMinutes") repeat.minutes = Number(F.minutes.value);
  return {
    title: F.title.value,
    details: F.details.value,
    due: `${F.date.value}T${F.time.value}:00`,
    repeat,
    enabled: F.enabled.checked,
    categoryId: selectedCategory(F.category),
  };
}

function editReminder(r) {
  editingId = r.id;
  F.title.value = r.title;
  F.details.value = r.details;
  setFormTime(parseLocal(r.due));
  setRepeat(r.repeat);
  F.enabled.checked = r.enabled;
  F.category.value = r.categoryId == null ? "" : String(r.categoryId);
  $("#reminder-form-title").textContent = "Edit reminder";
  $("#reminder-submit").textContent = "Save changes";
  $("#reminder-cancel").hidden = false;
  form.scrollIntoView({ behavior: "smooth" });
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

for (const b of $$("[data-habit]")) {
  b.addEventListener("click", async () => {
    await call("add_preset", { kind: b.dataset.habit });
    showNotice(`Added: ${b.textContent.replace(/^\S+\s/, "")}`);
    await refresh();
  });
}

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
  const query = $("#reminder-search").value;
  const cat = $("#reminder-filter").value;
  const shown = overview.reminders
    .filter((r) => filterCategory(cat, r.categoryId))
    .filter((r) => !query.trim() || matches(`${r.title} ${r.details} ${category(r.categoryId)?.name || ""}`, query))
    .sort((a, b) => {
      const rank = (r) => (r.completedAt ? 2 : r.enabled ? 0 : 1);
      return rank(a) - rank(b) || parseLocal(a.snoozedUntil || a.due) - parseLocal(b.snoozedUntil || b.due);
    });
  if (!overview.reminders.length) list.append(el("li", { className: "empty" }, "No reminders yet. Add one above."));
  else if (!shown.length) list.append(el("li", { className: "empty" }, "Nothing matches."));

  for (const r of shown) {
    const status = reminderStatus(r, now);
    const badge = status && el("span", { className: `badge${status.due ? " due" : ""}`, textContent: status.text });
    const meta = `${formatWhen(r.due)} · ${describeRepeat(r.repeat)}${r.details ? " · " + r.details : ""}`;
    const item = el(
      "li",
      { className: status && status.muted ? "muted" : "" },
      el("div", { className: "main" }, el("div", { className: "title" }, r.title, categoryChip(r.categoryId), badge), el("div", { className: "meta", textContent: meta })),
      button("Edit", () => editReminder(r)),
      button(r.enabled ? "Turn off" : "Turn on", async () => {
        await call("set_reminder_enabled", { id: r.id, enabled: !r.enabled });
        await refresh();
      }),
      button("Delete", async () => {
        if (!confirm(`Delete "${r.title}"?`)) return;
        await call("delete_reminder", { id: r.id });
        if (editingId === r.id) resetForm();
        await refresh();
      }, "danger"),
    );
    const c = category(r.categoryId);
    if (c) item.style.setProperty("--accent-line", c.color);
    list.append(item);
  }
}
$("#reminder-search").addEventListener("input", renderReminders);
$("#reminder-filter").addEventListener("change", renderReminders);

// ---------- notes ----------

$("#note-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const f = e.target.elements;
  await call("add_note", { text: f.text.value, categoryId: selectedCategory(f.category) });
  f.text.value = "";
  await refresh();
});

function renderNotes() {
  const list = $("#note-list");
  list.replaceChildren();
  const query = $("#note-search").value;
  const cat = $("#note-filter").value;
  const shown = [...overview.notes]
    .reverse()
    .filter((n) => filterCategory(cat, n.categoryId))
    .filter((n) => !query.trim() || matches(`${n.text} ${category(n.categoryId)?.name || ""}`, query));
  if (!overview.notes.length) list.append(el("li", { className: "empty" }, "No notes yet."));
  else if (!shown.length) list.append(el("li", { className: "empty" }, "Nothing matches."));
  for (const n of shown) {
    const item = el(
      "li",
      {},
      el("div", { className: "main" }, el("div", { className: "title" }, n.text, categoryChip(n.categoryId)), el("div", { className: "meta", textContent: `Added ${formatWhen(n.created)}` })),
      button("Copy", () => navigator.clipboard?.writeText(n.text).then(() => showNotice("Copied."))),
      button("Delete", async () => {
        await call("delete_note", { id: n.id });
        await refresh();
      }, "danger"),
    );
    const c = category(n.categoryId);
    if (c) item.style.setProperty("--accent-line", c.color);
    list.append(item);
  }
}
$("#note-search").addEventListener("input", renderNotes);
$("#note-filter").addEventListener("change", renderNotes);

// ---------- settings (every form.settings binds by input name) ----------

const settingsForms = $$("form.settings");

function fillSettings(s) {
  for (const f of settingsForms) {
    for (const input of f.elements) {
      if (!input.name || !(input.name in s)) continue;
      if (input.type === "checkbox") input.checked = s[input.name];
      else if (input.name === "monitorIndex") input.value = s.monitorIndex == null ? "" : String(s.monitorIndex);
      else input.value = s[input.name];
    }
  }
  $("#summary-time").value = `${pad(s.dailySummaryHour)}:${pad(s.dailySummaryMinute)}`;
  // Climbing only happens in the taskbar mode.
  const taskbar = s.roamMode === "bottomEdge";
  $('[name="climbWindows"]').disabled = !taskbar;
  $('[name="climbsPerHour"]').disabled = !taskbar || !s.climbWindows;
  $('[name="funPerHour"]').disabled = !s.funEnabled;
  $("#climb-hint").textContent = taskbar ? "Picks from the windows you used in the last 10 minutes." : "Only while it lives along the taskbar (Where it goes).";
  $("#clip-key").textContent = s.hotkeyClipboardNote || "(no shortcut set)";
  updateOutputs();
}

function updateOutputs() {
  for (const out of $$("output[data-for]")) {
    const input = $(`[name="${out.dataset.for}"]`);
    out.textContent =
      input.name === "scale"
        ? `${Number(input.value).toFixed(1)}×`
        : input.name === "climbsPerHour" || input.name === "funPerHour"
          ? `${input.value} ${input.value === "1" ? "time" : "times"} an hour`
          : `${input.value} px/s`;
  }
}

function readSettings() {
  const s = { ...overview.settings };
  for (const f of settingsForms) {
    for (const input of f.elements) {
      if (!input.name || !(input.name in s)) continue;
      if (input.type === "checkbox") s[input.name] = input.checked;
      else if (input.name === "monitorIndex") s.monitorIndex = input.value === "" ? null : Number(input.value);
      else if (typeof s[input.name] === "number") s[input.name] = Number(input.value);
      else s[input.name] = input.value;
    }
  }
  const [h, m] = ($("#summary-time").value || "09:00").split(":").map(Number);
  s.dailySummaryHour = h;
  s.dailySummaryMinute = m;
  return s;
}

async function saveSettings(patch = {}) {
  try {
    overview.settings = await call("save_settings", { settings: { ...readSettings(), ...patch } });
  } finally {
    // Hotkey errors arrive after saving; reload to show what is stored.
    overview = await invoke("get_overview");
    fillSettings(overview.settings);
    renderPacks();
  }
}

for (const f of settingsForms) {
  f.addEventListener("input", updateOutputs);
  f.addEventListener("change", () => saveSettings().catch(() => {}));
  f.addEventListener("submit", (e) => e.preventDefault());
}

// ---------- pet ----------

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

async function renderMonitors() {
  const sel = $("#monitor-select");
  const monitors = await invoke("list_monitors").catch(() => []);
  sel.replaceChildren(
    el("option", { value: "", textContent: "Wherever it is" }),
    ...monitors.map((m) => el("option", { value: String(m.index), textContent: `${m.name} (${m.width}×${m.height})` })),
  );
  if (overview) sel.value = overview.settings.monitorIndex == null ? "" : String(overview.settings.monitorIndex);
}

$("#reload-packs").addEventListener("click", async () => {
  await call("reload_packs");
  await refresh();
});
$("#feed").addEventListener("click", () => call("pet_feed").then(refreshStatus));
$("#play").addEventListener("click", () => call("pet_play").then(refreshStatus));
$("#say-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  await call("pet_say", { text: e.target.elements.text.value });
  e.target.elements.text.value = "";
});

// ---------- live status (mood, system, focus) ----------

const FEELINGS = { happy: "Happy", neutral: "Content", bored: "Bored", sad: "Sad", sleepy: "Sleepy", hungry: "Hungry" };

function bars(box, rows) {
  box.replaceChildren();
  for (const [label, v, text] of rows) {
    const fill = el("span");
    fill.style.width = `${Math.max(0, Math.min(100, v ?? 0))}%`;
    // A row with only text (like "Feeling") gets no bar.
    const bar = v == null && text != null && label === "Feeling" ? el("div", { className: "strong", textContent: text }) : el("div", { className: "bar" }, fill);
    const value = bar.className === "strong" ? "" : text ?? (v == null ? "–" : Math.round(v));
    box.append(el("div", { textContent: label }), bar, el("div", { textContent: value }));
  }
}

const fmtClock = (secs) => `${pad(Math.floor(secs / 60))}:${pad(secs % 60)}`;

async function refreshStatus() {
  let st;
  try {
    st = await invoke("get_status");
  } catch {
    return; // cosmetic
  }
  const care = overview?.settings.careEnabled;
  bars($("#mood"), [
    ["Feeling", null, FEELINGS[st.feeling] || st.feeling],
    ["Happiness", st.mood.happiness],
    ["Energy", st.mood.energy],
    ["Boredom", st.mood.boredom],
    ...(care ? [["Fullness", st.mood.fullness]] : []),
  ]);
  $("#feed").hidden = $("#play").hidden = !care;

  const sys = st.system;
  const pct = (v) => (v == null ? "n/a" : `${Math.round(v)}%`);
  bars($("#system"), [
    ["CPU", st.cpuAverage, `${pct(st.cpuAverage)}${st.condition.hot ? " 🔥" : ""}`],
    ["Memory", sys.ramPercent, `${pct(sys.ramPercent)}${st.condition.ramFull ? " ⚠" : ""}`],
    ["Battery", sys.battery?.percent, sys.battery ? `${pct(sys.battery.percent)}${sys.battery.charging ? " ⚡" : ""}` : "n/a"],
    ["Free disk", sys.diskFreePercent, `${pct(sys.diskFreePercent)}${st.condition.lowDisk ? " ⚠" : ""}`],
  ]);

  const f = st.focus;
  const running = f.phase !== "off";
  $("#focus-timer").textContent = running ? fmtClock(f.remainingSecs) : fmtClock((overview?.settings.focusMinutes ?? 25) * 60);
  $("#focus-phase").textContent = f.phase === "focus" ? "Focusing" : f.phase === "break" ? (f.longBreak ? "Long break" : "Break") : "Not running";
  $("#focus-progress").style.width = running && f.totalSecs ? `${100 - (f.remainingSecs / f.totalSecs) * 100}%` : "0%";
  $("#focus-start").hidden = running;
  $("#focus-stop").hidden = !running;
  $("#focus-today").textContent = `Sessions since the last long break: ${f.streak}`;
  $("#screen-time").textContent = `Screen time since your last break: ${st.screenMinutes} min`;

  const nav = [];
  if (st.dueCount) nav.push(`⏰ ${st.dueCount} due`);
  if (running) nav.push(`${f.phase === "focus" ? "🎯" : "☕"} ${fmtClock(f.remainingSecs)}`);
  $("#nav-status").textContent = nav.join("  ");
}
setInterval(refreshStatus, 1000);

$("#focus-start").addEventListener("click", () => call("focus_start").then(refreshStatus));
$("#focus-stop").addEventListener("click", () => call("focus_stop").then(refreshStatus));

// ---------- history ----------

const KIND_LABEL = { done: "✅ Done", snoozed: "💤 Snoozed", focus: "🎯 Focus", break: "☕ Break" };

async function refreshHistory() {
  const h = await call("get_history", { limit: 500 }).catch(() => null);
  if (!h) return;
  const s = h.stats;
  const tile = (value, label) => el("div", { className: "tile" }, el("strong", { textContent: value }), el("span", { textContent: label }));
  $("#stats").replaceChildren(
    tile(s.doneToday, "done today"),
    tile(s.doneThisWeek, "done this week"),
    tile(`${s.onTimePercent}%`, "answered within 5 min"),
    tile(`${s.currentStreak} 🔥`, `day streak (best ${s.bestStreak})`),
    tile(s.focusToday, "focus sessions today"),
    tile(s.breaksToday, "breaks today"),
  );
  const filter = $("#history-filter").value;
  const list = $("#history-list");
  list.replaceChildren();
  const entries = h.entries.filter((e) => !filter || e.kind === filter);
  if (!entries.length) list.append(el("li", { className: "empty" }, "Nothing here yet."));
  for (const e of entries) {
    const late = e.kind === "done" && e.waitedMinutes > 5 ? ` · answered after ${e.waitedMinutes} min` : "";
    list.append(
      el("li", {}, el("span", { className: "kind", textContent: KIND_LABEL[e.kind] || e.kind }), el("div", { className: "main" }, el("div", { className: "title", textContent: e.title }), el("div", { className: "meta", textContent: `${formatWhen(e.at)}${late}` }))),
    );
  }
}
$("#history-filter").addEventListener("change", refreshHistory);

// ---------- general ----------

$("#autostart").addEventListener("change", async (e) => {
  try {
    e.target.checked = await call("set_autostart", { enabled: e.target.checked });
  } catch {
    e.target.checked = !e.target.checked;
  }
});

const FILTERS = [{ name: "DesktopPet data", extensions: ["json"] }];

$("#export").addEventListener("click", async () => {
  const path = await dialog.save({ defaultPath: `desktoppet-${toDateInput(new Date())}.json`, filters: FILTERS });
  if (!path) return;
  await call("export_data", { path });
  showNotice(`Exported to ${path}`);
});

async function importFrom(mode) {
  const path = await dialog.open({ multiple: false, directory: false, filters: FILTERS });
  if (!path) return;
  if (mode === "replace" && !(await dialog.confirm("Replace ALL your reminders, notes, history and settings with this file?", { title: "Replace everything", kind: "warning" }))) return;
  const msg = await call("import_data", { path, mode });
  showNotice(msg);
  await refresh();
}
$("#import-merge").addEventListener("click", () => importFrom("merge").catch(() => {}));
$("#import-replace").addEventListener("click", () => importFrom("replace").catch(() => {}));

// ---------- load ----------

async function refresh() {
  overview = await call("get_overview");
  renderCategoryControls();
  renderReminders();
  renderNotes();
  renderPacks();
  fillSettings(overview.settings);
  $("#user-pets-dir").textContent = overview.userPetsDir;
  $("#data-path").textContent = overview.dataPath;
  refreshStatus();
  if (!$("#tab-history").hidden) refreshHistory();
}

listen("data-changed", () => refresh().catch(() => {}));
// Keep "Due now" badges current while the window is open.
setInterval(() => overview && renderReminders(), 30_000);

resetForm();
refresh()
  .then(renderMonitors)
  .catch(() => {});
invoke("get_autostart")
  .then((on) => ($("#autostart").checked = on))
  .catch(() => ($("#autostart").disabled = true));
