# Decky Macro

A Decky Loader plugin for the Steam Deck that allows you to easily record and replay precise gamepad input macros. 

Built with a blazing-fast, low-level Rust daemon that interfaces directly with the Linux `evdev` subsystem, ensuring 1:1 input reproduction without lag or dropped inputs.

## Features

- **Live Macro Recording:** Instantly record gamepad sequences directly during gameplay.
- **Perfect Playback:** Replays macros at the exact speed and duration they were recorded.
- **Native Decky UI:** Configure your trigger buttons, slots, and playback loops natively from the SteamOS Quick Access Menu.
- **Configurable Trigger Hold:** Choose how long the Record/Replay buttons must be held before they activate — anywhere from **0 to 2 seconds** in 100 ms steps (default **0.5s**). Set it to **0** to trigger instantly on press.
- **Echo Cancellation:** Deep `evdev` injection ensures macros seamlessly integrate with Gamescope without confusing Steam Input.
- **Manual Slot System:** Create, name, and manage multiple macro slots. Your active slot seamlessly switches in the background.
- **Auto-Cancellation:** Immediately abort a running macro loop by pressing any physical D-Pad or Face button (if enabled in the slot config).
- **Background Daemon:** Operates independently of the Quick Access Menu, meaning you can record and replay with zero performance impact.

## Limitations & Best Practices

- **Digital Inputs Only:** For extreme precision and to save memory, this plugin explicitly ignores analog stick movement (`ABS_X`/`ABS_Y`) and analog triggers (`ABS_Z`). **You must use the D-Pad for directional macros.**
- **Hardware Triggers:** Make sure your physical buttons (like the back paddles `L4`/`R4`) are mapped to the configured trigger buttons (e.g., `L3`/`R3`) via Steam Input.

## Macro Data Format & Limits

- **Maximum Size:** Each macro can record a maximum of **5,000 events**. Once this limit is hit, recording stops automatically and saves.
- **Format:** Macros are saved in `~/homebrew/settings/decky-macro/macro_<slot>.json`. They are simple JSON arrays containing the delay (in milliseconds since the last event), and the raw `evdev` event type, code, and value.
  ```json
  [
    { "delay_ms": 150, "event_type": 1, "code": 304, "value": 1 },
    { "delay_ms": 80, "event_type": 1, "code": 304, "value": 0 }
  ]
  ```

## Installation / Building

Because this plugin contains a compiled Rust backend, you must build it using Node.js and Cargo. 

1. Clone this repository into your development environment.
2. Ensure you have Node.js, `pnpm`, and the Rust toolchain installed.
3. Install dependencies:
   ```bash
   pnpm install
   ```
4. Build and assemble a deployable plugin folder:
   ```bash
   pnpm run build             # frontend bundle -> dist/
   pnpm run build:backend     # Rust daemon -> bin/   (or build:backend:static)
   pnpm run package           # -> target/decky-macro/
   ```
5. Move the resulting `target/decky-macro` folder to your Steam Deck's `~/homebrew/plugins/` directory and restart your Deck.

### How the backend is built

`pnpm run build` only builds the **frontend** (`rollup -c`). The daemon is built by `pnpm run build:backend` locally, or by the **store CI** through `backend/Dockerfile` (image `ghcr.io/steamdeckhomebrew/holo-toolchain-rust`), which mounts `backend/` → `/backend` and the output → `/backend/out`, then runs `entrypoint.sh` → `build.sh`. That container is **SteamOS-based**, so the store ships a binary linked against the Deck's glibc.

Building locally on a newer-libc host (WSL/Ubuntu) and running on the Deck can fail with `GLIBC_x.xx not found`. Use the static musl build instead — it runs anywhere:

```bash
pnpm run build:backend         # native (host libc)
pnpm run build:backend:static  # static musl
```

`build:backend:static` installs the musl Rust std if needed, and links via Rust's bundled `rust-lld` when no C compiler is present. You only need it for local testing — the store CI builds a SteamOS-glibc binary.

*(Note: If building for official release, remember to use the official `decky-cli` docker builder).*

## Usage

1. Open the Quick Access Menu (QAM) on your Steam Deck and navigate to the Decky Macro plugin.
2. With the daemon **stopped**, click **Settings** to choose your **Record Trigger** and **Replay Trigger** buttons (defaults to L3 / R3) and the **Trigger hold time**. The Settings button is disabled while the daemon is running.
3. Turn on the **Enable Decky Macro** toggle to start the daemon — the saved trigger settings are applied on start.
4. In a game, hold your **Record** button for the configured hold time to start capturing inputs. Hold it again to stop.
5. Hold the **Replay** button for the configured hold time to play back the exact sequence.

### Trigger Settings
The **Record Trigger**, **Replay Trigger** and **Trigger hold time** are configured from the **Settings** dialog (the button above *Active Macro Slot*). Because the daemon reads these values at startup, the **Settings button is disabled while the daemon is running** — stop the daemon, change the settings, then start it again.

**Trigger hold time** controls how long the Record/Replay buttons must be held before they activate:

- **Range:** `0` to `2` seconds, adjustable in `0.1s` (100 ms) steps.
- **Default:** `0.5` seconds.
- **`0` seconds:** the trigger fires **immediately** on press (no hold required).

The values are saved to the global config (`config.json` → `trigger_record_code`, `trigger_replay_code`, `hold_duration_ms`).

### Notifications
In the **Notifications** section of the plugin:

- **Show info toasts** — when enabled (the default), status toasts are shown for *Recording Started/Saved*, *Playback Started/Finished* and *Playback Cancelled*.
- **Error toasts are always shown**, regardless of this setting, for example:
  - *No inputs recorded — nothing saved*: a recording finished without capturing any button presses. The previous macro (if any) is kept.
  - *No macro recorded for this slot*: replay was triggered but the active slot has no recorded inputs.

### Loop Mode
In the slot settings (click "Edit Selected Slot"), you can configure how many times the macro should run:
- `1-50`: Run exactly N times.
- `0`: Infinite Loop (Will run until you manually interrupt it by pressing another button).
