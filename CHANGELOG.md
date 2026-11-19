# Changelog

All notable changes to Decky Macro are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.0] - 2026-11-19

### Added
- `pnpm run dist`, which builds the frontend and the static daemon, then
  packages the plugin.

### Changed
- Status and error toasts are now pushed from the backend with `decky.emit` and
  received through `addEventListener`, instead of the frontend polling
  `get_notifications` every second. This removes the constant background load,
  and toasts now show up as soon as the event happens.
- The Python backend now imports the `decky` module instead of the legacy
  `decky_plugin` alias.
- `pnpm run package` now runs `scripts/package.sh`. Besides the plugin folder,
  it creates `target/decky-macro-v<version>.zip` and includes LICENSE, README
  and CHANGELOG.

### Fixed
- The LICENSE file now keeps the original template copyright notice.

### Removed
- The `get_notifications` backend method.

## [1.1.2] - 2026-10-09

### Changed
- The build system now uses `backend/Dockerfile` (holo-toolchain-rust), so the
  plugin store CI can build the Rust daemon.
- The frontend build (`pnpm run build`) and the backend build
  (`pnpm run build:backend` / `build:backend:static`) are now separate scripts.
- Updated the documentation.

## [1.1.1] - 2026-10-09

### Changed
- Updated the plugin store image.

## [1.1.0] - 2026-10-09

### Added
- A configurable trigger hold time (0–2 s in 100 ms steps, default 0.5 s).
  Setting it to 0 makes the trigger fire as soon as the button is pressed.
- A Settings dialog for the record/replay trigger buttons and the hold time.
  It is only available while the daemon is stopped.
- Toasts for recording and playback events, plus a "Show info toasts" setting.
  Error toasts are always shown.
- Error toasts for a recording with no inputs (the previous macro is kept) and
  for a replay on an empty slot.
- An optional static musl build of the daemon for running on SteamOS.

### Fixed
- With a 0 ms hold, a duplicate press event could fire the trigger twice.
- Updated the LICENSE file.

### Removed
- Unused `defaults/` and `py_modules/` template folders.

## [1.0.0] - 2026-09-27

### Added
- Initial release: a Rust `evdev` daemon that records and replays digital
  gamepad input macros, with multiple named slots, per-slot loop counts and
  cancel-on-input.

[Unreleased]: https://github.com/olivervbk/decky-macro/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/olivervbk/decky-macro/compare/v1.1.2...v1.2.0
[1.1.2]: https://github.com/olivervbk/decky-macro/compare/v1.1.1...v1.1.2
[1.1.1]: https://github.com/olivervbk/decky-macro/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/olivervbk/decky-macro/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/olivervbk/decky-macro/releases/tag/v1.0.0
