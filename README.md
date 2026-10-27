# DesktopPet

A desktop pet for Windows, written in Rust with [Tauri v2](https://v2.tauri.app).
A panda (or any pet you add) walks along your taskbar, has moods, and pops up
a speech bubble to remind you of things.

![states](docs/panda-states.png)

- **Reminders** with repeats (daily, weekdays, chosen days, every N minutes), Done / Snooze from the bubble
- **Notes** the pet brings up now and then
- **Emotions**: happy, bored, sad, sleepy; it naps when you're away
- **Dashboard** to configure everything; **tray icon** to show/hide or quit
- **Swappable pets**: drop a folder with a `manifest.json` in your pets folder

See [SPEC.md](SPEC.md) for the full behaviour.

## Controls

| On the pet | Does |
|---|---|
| Left-click | Pet it |
| Drag | Move it (it falls back to the taskbar) |
| Right-click | Open the dashboard |

## Build and run (Windows)

You need:

- [Rust](https://rustup.rs) (the default MSVC toolchain)
- Microsoft C++ Build Tools ("Desktop development with C++")
- WebView2: already installed on Windows 10/11

Tauri's own prerequisites page is the source of truth if anything here is out
of date: <https://v2.tauri.app/start/prerequisites/>.

```powershell
cargo run -p desktop-pet          # run in debug
cargo test --workspace            # tests
```

To build an installer, install the Tauri CLI once and bundle:

```powershell
cargo install tauri-cli --version "^2" --locked
cd src-tauri
cargo tauri build                 # installer ends up in target\release\bundle\
```

## Layout

```
crates/pet-core/   platform-independent logic + tests: reminders, mood, behaviour, storage, pet packs
src-tauri/         the app: windows, tray, engine loop, Win32 calls (platform.rs), commands
ui/                pet window and dashboard (plain HTML/CSS/JS)
pets/              built-in pet packs (format: pets/README.md)
tools/             gen-panda.mjs: regenerates the panda art
```

Your data lives in `%APPDATA%\com.thejat.desktoppet\` (`data.json`, `pets\`).

## Status

Early version. The logic is unit-tested and the app compiles for Windows, but
it has not yet been run on a real Windows desktop. See *Known limits* in
SPEC.md for what to check first.

The original C++/Win32 version is on the `main` branch.
