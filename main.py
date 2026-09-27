import sys
import os
import subprocess
import json
import threading
import decky_plugin

class Plugin:
    process = None

    def __init__(self):
        self.config_dir = decky_plugin.DECKY_PLUGIN_SETTINGS_DIR
        self.config_path = os.path.join(self.config_dir, "config.json")
        self.data_path = os.path.join(self.config_dir, "data.json")
        self.bin_path = os.path.join(decky_plugin.DECKY_PLUGIN_DIR, "bin", "decky-macro-daemon")
        self.notifications = []

        if not os.path.exists(self.config_dir):
            os.makedirs(self.config_dir)

        if os.path.exists(self.bin_path):
            os.chmod(self.bin_path, 0o755)

        # Ensure default config
        if not os.path.exists(self.config_path):
            with open(self.config_path, "w") as f:
                json.dump({"trigger_record_code": 317, "trigger_replay_code": 318}, f)

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
            decky_plugin.logger.error(f"Cannot start daemon: binary not found at {self.bin_path}")
            return False
            
        try:
            data = self._get_data()
            active_slot = data.get("active_slot", "0")
            slot_config = data.get("slots", {}).get(active_slot, {})
            
            loops = str(slot_config.get("loops", 0))
            cancel = "true" if slot_config.get("cancel_on_input", True) else "false"
            macro_file = os.path.join(self.config_dir, f"macro_{active_slot}.json")

            decky_plugin.logger.info(f"Starting decky-macro-daemon daemon for slot {active_slot}...")
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
            
            def log_output(pipe):
                with pipe:
                    for line in iter(pipe.readline, b''):
                        line_str = line.decode('utf-8', errors='replace').strip()
                        decky_plugin.logger.info(f"[Rust] {line_str}")
                        
                        if "Started recording to" in line_str:
                            self.notifications.append("Recording Started")
                        elif "Stopped recording. Saved" in line_str:
                            self.notifications.append("Recording Saved")
                        elif "Started playback from" in line_str:
                            self.notifications.append("Playback Started")
                        elif "Playback finished." in line_str:
                            self.notifications.append("Playback Finished")
                        elif "Playback cancelled by physical input" in line_str:
                            self.notifications.append("Playback Cancelled")
                        
            threading.Thread(target=log_output, args=(Plugin.process.stdout,), daemon=True).start()
            return True
        except Exception as e:
            decky_plugin.logger.error(f"Failed to start daemon: {e}")
            return False

    def stop_daemon(self):
        if Plugin.process:
            decky_plugin.logger.info("Stopping decky-macro-daemon daemon...")
            Plugin.process.terminate()
            Plugin.process.wait()
            Plugin.process = None

    async def get_notifications(self) -> list:
        res = self.notifications.copy()
        self.notifications.clear()
        return res

    async def get_config(self) -> dict:
        if os.path.exists(self.config_path):
            with open(self.config_path, "r") as f:
                return json.load(f)
        return {"trigger_record_code": 317, "trigger_replay_code": 318}

    async def save_config(self, config: dict) -> bool:
        try:
            with open(self.config_path, "w") as f:
                json.dump(config, f, indent=4)
            if await self.get_daemon_state():
                import signal
                Plugin.process.send_signal(signal.SIGHUP)
            return True
        except: return False

    async def get_slots_data(self) -> dict:
        return self._get_data()

    async def save_slots_data(self, data: dict) -> bool:
        try:
            with open(self.data_path, "w") as f:
                json.dump(data, f, indent=4)
            if await self.get_daemon_state():
                self.start_daemon()
            return True
        except: return False

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

