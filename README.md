# Decky Macro

A Decky Loader plugin for the Steam Deck that allows you to easily record and replay precise gamepad input macros. 

Built with a blazing-fast, low-level Rust daemon that interfaces directly with the Linux `evdev` subsystem, ensuring 1:1 input reproduction without lag or dropped inputs.

## Features

- **Live Macro Recording:** Instantly record gamepad sequences directly during gameplay.
- **Perfect Playback:** Replays macros at the exact speed and duration they were recorded.
- **Native Decky UI:** Configure your trigger buttons, slots, and playback loops natively from the SteamOS Quick Access Menu.
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
3. Run the build script:
   ```bash
   pnpm install
   pnpm run build
   ```
4. Move the resulting `decky-macro` folder to your Steam Deck's `~/homebrew/plugins/` directory and restart your Deck.

*(Note: If building for official release, remember to use the official `decky-cli` docker builder).*

## Usage

1. Open the Quick Access Menu (QAM) on your Steam Deck and navigate to the Decky Macro plugin.
2. Turn on the "Enable Decky Macro" toggle.
3. Select which buttons you want to use for **Record** and **Replay** (Defaults to L3 / R3).
4. In a game, hold your **Record** button for 0.5s to start capturing inputs. Hold it again for 0.5s to stop.
5. Hold the **Replay** button for 0.5s to play back the exact sequence.

### Loop Mode
In the slot settings (click "Edit Selected Slot"), you can configure how many times the macro should run:
- `1-50`: Run exactly N times.
- `0`: Infinite Loop (Will run until you manually interrupt it by pressing another button).
