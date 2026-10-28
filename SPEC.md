# DesktopPet: Spec

A small animated pet that lives on the Windows desktop. It walks around, has
moods and needs, keeps an eye on your computer, and reminds you of things
through a speech bubble.

## Decisions

| Topic | Decision |
|---|---|
| Platform | Windows 10/11. Other OSes build for development, without drag, idle detection, the window perch and most system readings. |
| Stack | Rust + Tauri v2. Rust owns all logic; the windows are plain HTML/CSS/JS (no bundler). |
| Storage | One JSON file in `%APPDATA%\com.thejat.desktoppet\data.json`, written atomically. |
| Sound | None for now. |
| Network | None. Everything is local. |
| Default pet | Panda ("Bamboo"). Pets are swappable packs; a ginger cat is built in too. |

## Windows

1. **Pet window**: transparent, frameless, always on top, not in the taskbar.
   Exactly the size of the sprite; it grows upward only while a speech bubble
   is showing. (OS-level click-through is not used: on Windows it makes the
   window `WS_EX_LAYERED`, and the webview then draws nothing.)
2. **Quick-add box**: a small frameless box in the middle of the screen,
   opened by a global shortcut or menus. Hides when it loses focus.
3. **Dashboard**: a normal window. Closing it only hides it.
4. **Tray icon**: Open dashboard / Quick add / Show-hide pet / Quit. Left-click
   opens the dashboard. Its tooltip shows the focus timer.

Only one copy runs: starting it again opens the running pet's dashboard.

## The pet

**Controls**: left-click = pet it; drag = carry it (it falls when dropped);
right-click = menu (Pet, Feed, Play, Climb a window, Do something fun, Quick
add, Save clipboard as note, Start/stop focus timer, Open dashboard, Hide
pet).

**Trying things out**: `DESKTOPPET_DEMO=climb` or `=fun` triggers that every
30 seconds; `DESKTOPPET_DEBUG=1` logs which windows are climbable and why
others are not.

**Where it goes** (setting):
- *Along the taskbar* (default): walks on the bottom of the work area.
- *Anywhere on the screen*: wanders in 2-D, no gravity; stays where dropped.
- *On top of the window I'm using*: walks on the active window's title bar.
  Falls back to the taskbar for the desktop, minimised or full-height windows.
  Focusing one of our own windows keeps the last perch.
- *Monitor*: stay on a chosen monitor, or wherever it currently is.

**Climbing windows** (taskbar mode, on by default, 1–20 trips an hour,
default 3; the first one within about 2 minutes of starting):
- Candidates are the windows you focused in the last 10 minutes (up to 6),
  plus the one it is standing on. Skipped: minimised, maximised (no room on
  top), cloaked (other virtual desktops), tool/overlay windows, the shell and
  our own.
- It may stand only on the part of a window's top edge that no window in
  front covers, and keeps 150 px clear of the title-bar buttons.
- A trip: walk to a window's side → jump to grab it if its bottom is above
  the taskbar (up to 2.5 sprite heights) → climb hand over hand (seen from
  behind, moving in pulls) → pull up on top.
- **Out of reach** (or now and then, for fun): it walks to the screen edge,
  pulls out a ladder (poof), carries it back overhead, stands it against the
  window from the taskbar up, climbs it, and pulls it up after itself. If the
  window moves meanwhile, the ladder topples and the pet falls.
- Up there it walks, sits, perches on an end with its legs over the edge,
  and sometimes jumps to another window within 4 sprite widths (not more
  than 2 heights higher). After 1–3 minutes it comes down: a jump from low
  windows, an **umbrella** float from high ones.
- It rides along when the window is moved, and falls when the window is
  closed, minimised or covered, landing on whatever is below. Dropped from
  a drag, it can land on a window too.
- Right-click → **Climb a window** does it now, or says why it can't.

**Fun** (on by default, 1–20 an hour, default 4): pops a bicycle, motorbike
(with a helmet), car or skateboard out of a puff of smoke and rides along
the taskbar, turning at the ends; or dances; or reads a book with little
glasses. Right-click → **Do something fun** does one now.

**Reactions**: now and then walks over to the mouse pointer; a fast
left-right mouse shake startles it; says good morning/afternoon/evening at
start and "Welcome back!" after you were away; naps when you are idle for
*N* minutes.

**Animations**: idle, walk, sit, sleep, happy, sad, bored, surprised, drag,
alert, eat, play, sweat (computer under load), hungry, climb, jump, perch,
carry, float, poof, rideBike, rideMoto, rideCar, rideSkate, dance, read.
On top of the frames the pet window adds small motions: breathing when
still, a bob when walking, a rumble on engines, a sway when dancing.

## Emotions and care

Four stats, 0–100: **happiness**, **energy**, **boredom**, **fullness**. The
mood is saved every minute and on quit.

| Changes | Effect |
|---|---|
| Time passing (awake) | energy −0.5/min (−1.5/min 23:00–06:00), boredom +1/min |
| Sleeping | energy +6/min, boredom −2/min |
| Always | happiness drifts toward 60 |
| A reminder ignored > 5 min | happiness −2/min |
| Care mode, awake / asleep | fullness −0.25/min / −0.1/min; below 20, happiness −1/min |
| Poked | happiness +8, boredom −25 |
| Fed | fullness +35, happiness +5 |
| Played | boredom −40, happiness +10, energy −5 |
| Reminder done / snoozed | happiness +10 / −2 |

Feeling: Sleepy (energy < 15, or battery low) → Hungry (care mode, fullness <
20) → Sad (happiness < 30) → Bored (boredom > 70) → Happy (happiness > 75) →
Neutral. The feeling picks the idle animation and what it does next. When
hungry it asks to be fed, at most every 30 minutes.

## Computer health

Readings every 2 s: CPU (`GetSystemTimes`, averaged over ~20 s), memory load,
battery (`GetSystemPowerStatus`), free space on the system drive.

| Limit (default) | Pet |
|---|---|
| CPU ≥ 90 % sustained | sweats, walks slower, says so |
| Memory ≥ 90 % | sweats, says so |
| Battery ≤ 20 % and not charging | sleepy, says so |
| Free disk ≤ 10 % | says so |

Limits have hysteresis (CPU clears 10 points below, etc.) and each message
has a cooldown (15 min to 2 h). Speech can be turned off, leaving only the
animations. Live readings show in the dashboard.

## Reminders

- Title, details, date + time, repeat, category, enabled flag.
- Repeat: once, every day, weekdays, chosen weekdays, every *N* minutes.
- When due: the pet stops, plays its alert animation and shows a bubble with
  **Done** / **Snooze**; it stays until answered. Several due → oldest first.
- If the pet is hidden or a full-screen app/presentation is running, a
  Windows notification is sent too (setting).
- Done on a repeating reminder moves it to the next occurrence after now.
- Missed while closed → fire once at the next start. Local wall-clock times.

**Typing a reminder** (quick-add box and the dashboard's "Type it"): a small
English parser understands "in 20 min", "in an hour", "at 5pm", "17:30",
"noon", "today/tonight/tomorrow", "tomorrow morning", weekday names, "next
friday", "dec 25", "5 nov", "2026-11-30", and repeats like "every day",
"daily", "every weekday", "every mon and wed", "every 45 minutes", "every 2h",
"hourly". "at 5" means the next 5 o'clock. Words that are not a time become
the title; "remind me to" is dropped. With no time, the quick box saves a
note instead.

**Healthy-habit presets** (one click, category "Health"): drink water
(hourly), check posture (30 min), stretch (hourly), rest your eyes (20 min).

## Notes, chatter and summaries

- Notes are text without a time, with an optional category.
- **Clipboard to note**: a global shortcut (default `Ctrl+Alt+N`) or the
  pet's menu saves the clipboard's text as a note.
- **Chatter**: every ~N minutes (±25 %) a random note or a mood line. Never
  while sleeping, focusing, during quiet hours, or over another bubble.
- **Daily summary**: once a day at a set time (default 09:00, or at the
  first start after it) the pet lists today's reminders.

## Breaks and focus

- **Screen breaks**: continuous activity (input within the last minute) is
  counted; after *N* minutes (default 50) the pet suggests a break with
  "Taking one" / "In 15 min" buttons, and asks again 10 minutes later if
  ignored. Being away for 5 minutes counts as a break and resets the count.
- **Focus timer** (pomodoro): focus 25 min → short break 5 min, long break
  15 min after every 4 sessions. The pet keeps quiet while focusing. At the
  end it offers the break, and after the break another session. Completed
  sessions go to the history.

## Speech bubbles

Three kinds, in priority order: **reminder** (stays until answered) >
**suggestion** (break, focus, summary; with buttons, queued behind
reminders) > **chatter** (disappears after a few seconds, never replaces the
others). Clicking the pet dismisses chatter.

## History

Every done/snoozed reminder (with how long it waited), focus session and
taken break is logged (last 5000 entries). The dashboard shows: done today,
done this week, share answered within 5 minutes, current and best streak of
days with something done, focus sessions and breaks today, and a filterable
log.

## Categories and search

Categories have a name and colour. Reminders and notes can have one. The
dashboard filters by category and searches titles, details and notes.
Deleting a category keeps its items.

## Dashboard

Tabs: Reminders, Notes, Focus & breaks, History, Pet, Behaviour, General.
Settings save as soon as they change. The sidebar shows due reminders and the
running timer.

**General**: start with Windows (registers the program being run), global
shortcuts (quick add `Ctrl+Alt+R`, clipboard note `Ctrl+Alt+N`; empty turns
one off), notifications, **export** to a JSON file, **import** to add to or
replace everything (a backup of the current data is written first; files
that are not DesktopPet data are refused).

## Pet packs

A folder with `manifest.json` and image frames (`.png`, `.gif`, `.webp`,
`.jpg`, `.svg`). Only `idle` is required; missing animations fall back. Built-in
packs ship in `pets/`; user packs go in
`%APPDATA%\com.thejat.desktoppet\pets` and override a built-in with the same
id. Format: [`pets/README.md`](pets/README.md).

## Known limits / to verify on real Windows

The code is type-checked for Windows and the logic is unit-tested, but these
need a real Windows desktop to confirm:

- The transparent corners of the sprite's rectangle still catch clicks.
- Drag and the window perch assume the process is per-monitor DPI aware, so
  `GetCursorPos`, window frames and window positions are all physical
  pixels. I believe Tauri sets this, but it is not confirmed on mixed DPI.
- Staying above full-screen apps is not guaranteed (notifications cover it).
- Global shortcuts can fail if another app already uses the combination;
  the dashboard shows the error.
- "Start with Windows" from a debug build registers the debug executable.
