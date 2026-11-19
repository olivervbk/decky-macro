import sys
import os
import subprocess
import json
import threading
import traceback
import asyncio
import decky

# Default global config. `hold_duration_ms` is how long the record/replay
# trigger buttons must be held before activating (0 = trigger on press).
# `show_info_toasts` controls whether informational status toasts are shown;
# error toasts are always shown regardless of this setting.
DEFAULT_CONFIG = {
    "trigger_record_code": 317,
    "trigger_replay_code": 318,
    "hold_duration_ms": 500,
    "show_info_toasts": True,
}

def _log_emit_error(future):
    # Futures from run_coroutine_threadsafe are never awaited, so surface errors here.
    try:
        exc = future.exception()
    except Exception as e:  # cancelled
        exc = e
    if exc is not None:
        decky.logger.error(f"Failed to emit notification: {exc}")


class Plugin:
    process = None

    def __init__(self):
        self.config_dir = decky.DECKY_PLUGIN_SETTINGS_DIR
        self.config_path = os.path.join(self.config_dir, "config.json")
        self.data_path = os.path.join(self.config_dir, "data.json")
        self.bin_path = os.path.join(decky.DECKY_PLUGIN_DIR, "bin", "decky-macro-daemon")

        if not os.path.exists(self.config_dir):
            os.makedirs(self.config_dir)

        if os.path.exists(self.bin_path):
            os.chmod(self.bin_path, 0o755)

        # Ensure default config
        if not os.path.exists(self.config_path):
            with open(self.config_path, "w") as f:
                json.dump(dict(DEFAULT_CONFIG), f, indent=4)

        # Ensure default data.json
        if not os.path.exists(self.data_path):
            with open(self.data_path, "w") as f:
                json.dump({
                    "active_slot": "0",
                    "slots": {
                        "0": { "name": "Default", "loops": 0, "cancel_on_input": True }
                    }
                }, f)

    def _get_data(self):
        if not os.path.exists(self.data_path): return {}
        with open(self.data_path, "r") as f:
            return json.load(f)

    def start_daemon(self) -> bool:
        self.stop_daemon()
        if not os.path.exists(self.bin_path):
            decky.logger.error(f"Cannot start daemon: binary not found at {self.bin_path}")
            return False
            
        try:
            data = self._get_data()
            active_slot = data.get("active_slot", "0")
            slot_config = data.get("slots", {}).get(active_slot, {})
            
            loops = str(slot_config.get("loops", 0))
            cancel = "true" if slot_config.get("cancel_on_input", True) else "false"
            macro_file = os.path.join(self.config_dir, f"macro_{active_slot}.json")

            decky.logger.info(f"Starting decky-macro-daemon daemon for slot {active_slot}...")
            Plugin.process = subprocess.Popen(
                [
                    self.bin_path, 
                    "--config", self.config_path, 
                    "--macro-file", macro_file,
                    "--loops", loops,
                    "--cancel-on-input", cancel
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT
            )
            
            # start_daemon is only called from async callables, so a loop is
            # running here. The reader thread below uses it to push toasts to
            # the frontend via decky.emit (instead of the frontend polling).
            try:
                loop = asyncio.get_running_loop()
            except RuntimeError:
                loop = None
                decky.logger.warning("No running event loop; daemon notifications will not be sent.")

            def log_output(pipe):
                def notify(message, error=False):
                    if loop is None or loop.is_closed():
                        return
                    try:
                        future = asyncio.run_coroutine_threadsafe(
                            decky.emit("notification", message, error), loop
                        )
                        future.add_done_callback(_log_emit_error)
                    except RuntimeError as e:
                        # Loop shut down between the check and the call (plugin unload).
                        decky.logger.debug(f"Dropped notification '{message}': {e}")

                with pipe:
                    for line in iter(pipe.readline, b''):
                        line_str = line.decode('utf-8', errors='replace').strip()
                        decky.logger.info(f"[Rust] {line_str}")

                        if "Started recording to" in line_str:
                            notify("Recording Started")
                        elif "Stopped recording. Saved" in line_str:
                            notify("Recording Saved")
                        elif "Recording stopped with no inputs" in line_str:
                            notify("No inputs recorded - nothing saved", error=True)
                        elif "Started playback from" in line_str:
                            notify("Playback Started")
                        elif "Playback finished." in line_str:
                            notify("Playback Finished")
                        elif "Playback cancelled by physical input" in line_str:
                            notify("Playback Cancelled")
                        elif "Playback failed: no recorded inputs" in line_str:
                            notify("No macro recorded for this slot", error=True)
                        
            threading.Thread(target=log_output, args=(Plugin.process.stdout,), daemon=True).start()
            return True
        except Exception as e:
            decky.logger.error(f"Failed to start daemon: {e}\n{traceback.format_exc()}")
            return False

    def stop_daemon(self):
        if Plugin.process:
            decky.logger.info("Stopping decky-macro-daemon daemon...")
            Plugin.process.terminate()
            Plugin.process.wait()
            Plugin.process = None

    async def get_config(self) -> dict:
        config = dict(DEFAULT_CONFIG)
        if os.path.exists(self.config_path):
            try:
                with open(self.config_path, "r") as f:
                    config.update(json.load(f))
            except Exception as e:
                decky.logger.error(
                    f"Failed to read config from {self.config_path}: {e}\n{traceback.format_exc()}"
                )
        return config

    async def save_config(self, config: dict) -> bool:
        # Trigger/replay/hold settings are only read by the daemon at startup, so
        # the UI restricts editing them to when the daemon is stopped. We no longer
        # signal a running daemon here to reload (that behaviour was removed).
        try:
            with open(self.config_path, "w") as f:
                json.dump(config, f, indent=4)
            decky.logger.info("Config saved. Trigger/hold changes apply on next daemon start.")
            return True
        except Exception as e:
            decky.logger.error(f"Failed to save config to {self.config_path}: {e}\n{traceback.format_exc()}")
            return False

    async def get_slots_data(self) -> dict:
        return self._get_data()

    async def save_slots_data(self, data: dict) -> bool:
        try:
            with open(self.data_path, "w") as f:
                json.dump(data, f, indent=4)
            if await self.get_daemon_state():
                self.start_daemon()
            return True
        except Exception as e:
            decky.logger.error(f"Failed to save slots data to {self.data_path}: {e}\n{traceback.format_exc()}")
            return False

    async def get_daemon_state(self) -> bool:
        if Plugin.process is None: return False
        return Plugin.process.poll() is None

    async def set_daemon_state(self, is_running: bool) -> bool:
        if is_running:
            if not self.start_daemon():
                raise Exception("Failed to start Decky Macro daemon. Check logs for details.")
        else:
            self.stop_daemon()
        return True

    async def _main(self):
        # Daemon DOES NOT start by default anymore
        pass

    async def _unload(self):
        self.stop_daemon()

