# DesktopPet: Spec

A small animated pet that lives on the Windows desktop, walks along the
taskbar, has moods, and reminds you of things through a speech bubble.

## Decisions

| Topic | Decision |
|---|---|
| Platform | Windows 10/11. Other OSes build for development, without drag, click-through or idle detection. |
| Stack | Rust + Tauri v2. Rust owns all logic; the two windows are plain HTML/CSS/JS (no bundler). |
| Storage | One JSON file in `%APPDATA%\com.thejat.desktoppet\data.json`, written atomically. |
| Sound | None for now. |
| Network | None. Everything is local. |
| Default pet | Panda ("Bamboo"). Pets are swappable packs. |

## Windows

1. **Pet window**: transparent, frameless, always on top, not in the taskbar.
   Exactly the size of the sprite; it grows upward only while a speech bubble
   is showing, so it never blocks clicks on more of the screen than needed.
   (OS-level click-through is not used: on Windows it makes the window
   `WS_EX_LAYERED`, and the webview then draws nothing.)
2. **Dashboard**: a normal window. Closing it only hides it.
3. **Tray icon**: Open dashboard / Show-hide pet / Quit. Left-click opens the dashboard.

## Pet behaviour

- Walks along the bottom of the work area (just above the taskbar) of the
  monitor it is on. Turns at the edges.
- Activities: idle, walk, sit, sleep, react (happy hop, surprised), alert.
- **Left-click** = pet it (happy reaction). **Drag** = pick it up; on release
  it falls to the floor. **Right-click** = open the dashboard.
- Naps when you've been away from keyboard/mouse for *N* minutes
  (`GetLastInputInfo`), wakes when you're back.
- Drops in from the top of the screen at startup.

## Emotions

Three stats, 0–100: **happiness**, **energy**, **boredom**.

| Changes | Effect |
|---|---|
| Time passing (awake) | energy −0.5/min (−1.5/min 23:00–06:00), boredom +1/min |
| Sleeping | energy +6/min, boredom −2/min |
| Always | happiness drifts toward 60 |
| A reminder ignored > 5 min | happiness −2/min |
| Poked | happiness +8, boredom −25 |
| Dragged | boredom −10 |
| Reminder done / snoozed | happiness +10 / −2 |

Dominant feeling: Sleepy (energy < 15) → Sad (happiness < 30) → Bored
(boredom > 70) → Happy (happiness > 75) → Neutral. The feeling picks the idle
animation and how often the pet walks, sits or hops. Can be turned off.

## Reminders

- Title, optional details, date + time, enabled flag.
- Repeat: once, every day, weekdays, chosen weekdays, every *N* minutes.
- When due: the pet stops, plays its alert animation, and shows a bubble with
  **Done** / **Snooze**. The bubble stays until answered. Several due → oldest
  first.
- Done on a repeating reminder moves it to the next occurrence after now
  (missed occurrences are not replayed one by one).
- Reminders missed while the app was closed fire once at the next start.
- Times are local wall-clock times.

## Notes and chatter

- Notes are text without a time.
- With chatter on, every ~N minutes (±25%) the pet says something: a random
  note ("Don't forget: …") or a line matching its mood. Never while sleeping,
  during quiet hours, or over a reminder.

## Dashboard

- **Reminders**: add / edit / turn off / delete, quick presets (+10 min, +1 h, tomorrow 9:00).
- **Notes**: add / delete.
- **Pet**: choose a pack, name, size (0.5–3×), walking speed, live mood bars,
  "say something" to preview the bubble, reload packs.
- **Behaviour**: emotions on/off, chatter on/off + interval, bubble duration,
  snooze length, nap delay, quiet hours.
- **About**: controls and data location.

Settings save as soon as they change.

## Pet packs

A folder with `manifest.json` and image frames (`.png`, `.gif`, `.webp`,
`.jpg`, `.svg`). Only `idle` is required; missing animations fall back
(e.g. `sleep` → `sit` → `idle`). Built-in packs ship in `pets/`; user packs go
in `%APPDATA%\com.thejat.desktoppet\pets` and override a built-in with the
same id. Format: [`pets/README.md`](pets/README.md).

## Not in this version (candidates for later)

- Show CPU / RAM usage through the pet's animation (from the original README plan).
- Start with Windows, global hotkey to add a reminder.
- Natural-language times ("in 20 minutes", "tomorrow at 5").
- Roaming the whole screen / chosen monitor; multi-monitor preferences.
- Sound, Windows toast notifications as a fallback.
- Single-instance guard.
- Calendar, weather, other integrations.

## Known limits / to verify on real Windows

These were type-checked for Windows but not yet run on a Windows machine:

- The transparent corners of the sprite's rectangle still catch clicks.
- Drag/hit-testing assume the process is per-monitor DPI aware so that
  `GetCursorPos` and window positions are both physical pixels. I believe
  Tauri sets this, but it has not been confirmed on a mixed-DPI setup.
- Staying above full-screen apps is not guaranteed.
