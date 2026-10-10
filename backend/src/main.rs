use anyhow::Result;
use evdev::{Device, InputEvent, InputEventKind};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn default_hold_ms() -> u64 {
    500
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Config {
    trigger_record_code: u16,
    trigger_replay_code: u16,
    // How long the trigger button must be held before activating, in milliseconds.
    // 0 means "activate immediately on press". Defaults to 500 (0.5s) so that
    // config files written before this field existed keep their old behaviour.
    #[serde(default = "default_hold_ms")]
    hold_duration_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MacroEvent {
    delay_ms: u64,
    event_type: u16,
    code: u16,
    value: i32,
}

impl MacroEvent {
    fn from_evdev(delay: Duration, event: InputEvent) -> Self {
        Self {
            delay_ms: delay.as_millis() as u64,
            event_type: event.event_type().0,
            code: event.code(),
            value: event.value(),
        }
    }
}

// Convert compressed string back to full struct for logic
fn parse_compressed_events(strings: Vec<String>) -> Vec<MacroEvent> {
    let mut events = Vec::new();
    for s in strings {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 4 {
            if let (Ok(d), Ok(t), Ok(c), Ok(v)) = (
                parts[0].parse::<u64>(),
                parts[1].parse::<u16>(),
                parts[2].parse::<u16>(),
                parts[3].parse::<i32>(),
            ) {
                events.push(MacroEvent {
                    delay_ms: d,
                    event_type: t,
                    code: c,
                    value: v,
                });
            }
        }
    }
    events
}

fn load_or_create_config(path: &str) -> Config {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(config) = serde_json::from_str(&data) {
            return config;
        }
    }
    Config {
        trigger_record_code: 317,
        trigger_replay_code: 318,
        hold_duration_ms: default_hold_ms(),
    }
}

fn load_macro(path: &Path) -> Vec<MacroEvent> {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(strings) = serde_json::from_str::<Vec<String>>(&data) {
            return parse_compressed_events(strings);
        }
    }
    Vec::new()
}

fn save_macro(path: &Path, events: &Vec<MacroEvent>) {
    let mut compressed = Vec::new();
    for e in events {
        compressed.push(format!("{}:{}:{}:{}", e.delay_ms, e.event_type, e.code, e.value));
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(&compressed) {
        let _ = fs::write(path, json);
    }
}

#[derive(PartialEq)]
enum State {
    Idle,
    Recording,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting Decky Macro v{}...", env!("CARGO_PKG_VERSION"));

    let args: Vec<String> = std::env::args().collect();
    let mut config_path = String::new();
    let mut macro_file = String::new();
    let mut loops: i32 = 0;
    let mut cancel_on_input = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => { config_path = args[i + 1].clone(); i += 2; }
            "--macro-file" => { macro_file = args[i + 1].clone(); i += 2; }
            "--loops" => { loops = args[i + 1].parse().unwrap_or(0); i += 2; }
            "--cancel-on-input" => { cancel_on_input = args[i + 1] == "true"; i += 2; }
            _ => i += 1,
        }
    }

    let mut config = load_or_create_config(&config_path);
    let active_macro_path = PathBuf::from(&macro_file);
    let mut recorded_events = load_macro(&active_macro_path);

    let reload_config = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGHUP, Arc::clone(&reload_config))?;

    info!("Loaded active slot: {:?}", active_macro_path);
    info!("Loops: {}, Cancel: {}", loops, cancel_on_input);
    info!("Trigger hold duration: {} ms", config.hold_duration_ms);

    let (tx, rx) = mpsc::channel();
    let tx_discovery = tx.clone();

    thread::spawn(move || {
        let mut opened_paths = HashSet::new();
        loop {
            if let Ok(input_dir) = std::fs::read_dir("/dev/input") {
                for entry in input_dir.flatten() {
                    let path = entry.path();
                    if path.to_string_lossy().contains("event") {
                        if !opened_paths.contains(&path) {
                            if let Ok(mut device) = Device::open(&path) {
                                opened_paths.insert(path.clone());
                                let tx_inner = tx_discovery.clone();
                                let path_clone = path.clone();
                                thread::spawn(move || {
                                    loop {
                                        match device.fetch_events() {
                                            Ok(events) => {
                                                for event in events {
                                                    let kind = event.kind();
                                                    let is_key = matches!(kind, InputEventKind::Key(_));
                                                    let is_dpad = matches!(
                                                        kind,
                                                        InputEventKind::AbsAxis(evdev::AbsoluteAxisType::ABS_HAT0X) |
                                                        InputEventKind::AbsAxis(evdev::AbsoluteAxisType::ABS_HAT0Y)
                                                    );
                                                    if is_key || is_dpad {
                                                        let _ = tx_inner.send((path_clone.clone(), event));
                                                    }
                                                }
                                            }
                                            Err(_) => break,
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });

    let mut state = State::Idle;
    let mut record_start_time = Instant::now();
    let mut is_replaying = false;
    let mut replay_index = 0;
    let mut replay_start_time = Instant::now();
    let mut current_iteration = 0;

    let mut record_held_since: Option<Instant> = None;
    let mut replay_held_since: Option<Instant> = None;
    let mut record_triggered_this_press = false;
    let mut replay_triggered_this_press = false;

    let mut write_devices: HashMap<PathBuf, Device> = HashMap::new();
    let mut ignore_injected: HashMap<(PathBuf, u16, u16, i32), usize> = HashMap::new();
    let mut last_active_device: Option<PathBuf> = None;

    loop {
        if reload_config.swap(false, Ordering::Relaxed) {
            info!("SIGHUP received, reloading global config from {:?}", config_path);
            config = load_or_create_config(&config_path);
            info!("Reloaded config: record={}, replay={}, hold={}ms",
                config.trigger_record_code, config.trigger_replay_code, config.hold_duration_ms);
        }

        let timeout = if is_replaying && replay_index < recorded_events.len() {
            let next_event = &recorded_events[replay_index];
            let target_time = replay_start_time + Duration::from_millis(next_event.delay_ms);
            let now = Instant::now();
            if target_time > now { Some(target_time - now) } else { Some(Duration::from_secs(0)) }
        } else {
            Some(Duration::from_millis(50))
        };

        let msg = if let Some(t) = timeout { rx.recv_timeout(t).ok() } else { rx.recv().ok() };
        let now = Instant::now();

        if let Some((device_path, event)) = msg {
            let kind = event.kind();
            let value = event.value();
            
            let key_tuple = (device_path.clone(), event.event_type().0, event.code(), value);
            if let Some(count) = ignore_injected.get_mut(&key_tuple) {
                if *count > 0 {
                    *count -= 1;
                    continue;
                }
            }

            last_active_device = Some(device_path.clone());

            let is_record_btn = matches!(kind, InputEventKind::Key(k) if k.code() == config.trigger_record_code);
            let is_replay_btn = matches!(kind, InputEventKind::Key(k) if k.code() == config.trigger_replay_code);

            if is_record_btn {
                if value == 1 {
                    // Only act on a fresh press edge; ignore duplicate/echoed
                    // value==1 events so a 0ms hold cannot trigger twice.
                    if record_held_since.is_none() {
                        record_held_since = Some(now);
                        record_triggered_this_press = false;
                    }
                } else if value == 0 {
                    record_held_since = None;
                }
            }
            if is_replay_btn {
                if value == 1 {
                    if replay_held_since.is_none() {
                        replay_held_since = Some(now);
                        replay_triggered_this_press = false;
                    }
                } else if value == 0 {
                    replay_held_since = None;
                }
            }

            if is_replaying && value != 0 && !is_record_btn && !is_replay_btn && cancel_on_input {
                warn!("Playback cancelled by physical input: {:?}", kind);
                is_replaying = false;
            }

            if state == State::Recording && !is_record_btn && !is_replay_btn {
                if recorded_events.len() >= 5000 {
                    state = State::Idle;
                    info!("Macro size limit reached. Stopped recording.");
                    save_macro(&active_macro_path, &recorded_events);
                } else {
                    recorded_events.push(MacroEvent::from_evdev(record_start_time.elapsed(), event));
                }
            }
        }

        if let Some(held) = record_held_since {
            if !record_triggered_this_press && held.elapsed() >= Duration::from_millis(config.hold_duration_ms) {
                if replay_held_since.is_none() {
                    record_triggered_this_press = true;
                    if state == State::Idle {
                        state = State::Recording;
                        recorded_events.clear();
                        record_start_time = Instant::now();
                        is_replaying = false;
                        info!("Started recording to {:?} (hold {}ms)", active_macro_path, config.hold_duration_ms);
                    } else {
                        state = State::Idle;
                        if recorded_events.is_empty() {
                            error!("Recording stopped with no inputs; previous macro kept.");
                            // Keep in-memory state consistent with the (unchanged)
                            // file so a later replay uses the previous macro, if any.
                            recorded_events = load_macro(&active_macro_path);
                        } else {
                            save_macro(&active_macro_path, &recorded_events);
                            info!("Stopped recording. Saved {} events.", recorded_events.len());
                        }
                    }
                }
            }
        }

        if let Some(held) = replay_held_since {
            if !replay_triggered_this_press && held.elapsed() >= Duration::from_millis(config.hold_duration_ms) {
                if record_held_since.is_none() {
                    replay_triggered_this_press = true;
                    if state == State::Idle {
                        if recorded_events.is_empty() {
                            error!("Playback failed: no recorded inputs for {:?}", active_macro_path);
                        } else {
                            is_replaying = true;
                            replay_index = 0;
                            current_iteration = 0;
                            replay_start_time = Instant::now();
                            info!("Started playback from {:?} (hold {}ms)", active_macro_path, config.hold_duration_ms);
                        }
                    }
                }
            }
        }

        if is_replaying && replay_index < recorded_events.len() {
            let next_event = &recorded_events[replay_index];
            if Instant::now() >= replay_start_time + Duration::from_millis(next_event.delay_ms) {
                if let Some(device_path) = &last_active_device {
                    let dev = write_devices.entry(device_path.clone()).or_insert_with(|| {
                        Device::open(device_path).unwrap()
                    });

                    let t = next_event.event_type;
                    let c = next_event.code;
                    let v = next_event.value;
                    let injection = evdev::InputEvent::new(evdev::EventType(t), c, v);
                    let sync = evdev::InputEvent::new(evdev::EventType::SYNCHRONIZATION, 0, 0);

                    if let Ok(_) = dev.send_events(&[injection, sync]) {
                        *ignore_injected.entry((device_path.clone(), t, c, v)).or_insert(0) += 1;
                    }
                }
                replay_index += 1;
            }
        } else if is_replaying && replay_index >= recorded_events.len() {
            if loops <= 0 || current_iteration < loops - 1 {
                current_iteration += 1;
                replay_index = 0;
                replay_start_time = Instant::now();
                info!("Looping macro (Iteration {})", current_iteration);
            } else {
                is_replaying = false;
                info!("Playback finished.");
            }
        }
    }
}
