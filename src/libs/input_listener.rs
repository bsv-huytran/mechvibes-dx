// NOTE: We provide two implementations:
// - macOS: use CoreGraphics Event Tap (avoids rdev crash on key typing)
// - Others: keep the current rdev-based listener

use std::collections::HashSet;
use std::sync::{ mpsc::Sender, Arc, Mutex };
use std::thread;
use std::time::{ Duration, Instant };

// ================================
// Shared helpers (key/button maps)
// ================================

// Maps a keyboard key to its standardized code (for the rdev path)
#[cfg(not(target_os = "macos"))]
use rdev::Key;

#[cfg(not(target_os = "macos"))]
fn map_key_to_code(key: Key) -> &'static str {
    match key {
        // Common keys across all platforms
        Key::Space => "Space",
        Key::Backspace => "Backspace",
        Key::CapsLock => "CapsLock",
        Key::Tab => "Tab",
        Key::Return => "Enter",
        Key::Escape => "Escape",
        Key::Delete => "Delete",

        // Modifier keys with left/right variants
        Key::Alt => "AltLeft",
        Key::AltGr => "AltRight",
        Key::ShiftLeft => "ShiftLeft",
        Key::ShiftRight => "ShiftRight",
        Key::ControlLeft => "ControlLeft",
        Key::ControlRight => "ControlRight",
        Key::MetaLeft => "MetaLeft",
        Key::MetaRight => "MetaRight",

        // Arrow keys
        Key::UpArrow => "ArrowUp",
        Key::DownArrow => "ArrowDown",
        Key::LeftArrow => "ArrowLeft",
        Key::RightArrow => "ArrowRight",

        // Navigation keys
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Insert => "Insert", // Function keys F1-F12 (rdev 0.5.3 only supports F1-F12)
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",

        // Alpha keys A-Z
        Key::KeyA => "KeyA",
        Key::KeyB => "KeyB",
        Key::KeyC => "KeyC",
        Key::KeyD => "KeyD",
        Key::KeyE => "KeyE",
        Key::KeyF => "KeyF",
        Key::KeyG => "KeyG",
        Key::KeyH => "KeyH",
        Key::KeyI => "KeyI",
        Key::KeyJ => "KeyJ",
        Key::KeyK => "KeyK",
        Key::KeyL => "KeyL",
        Key::KeyM => "KeyM",
        Key::KeyN => "KeyN",
        Key::KeyO => "KeyO",
        Key::KeyP => "KeyP",
        Key::KeyQ => "KeyQ",
        Key::KeyR => "KeyR",
        Key::KeyS => "KeyS",
        Key::KeyT => "KeyT",
        Key::KeyU => "KeyU",
        Key::KeyV => "KeyV",
        Key::KeyW => "KeyW",
        Key::KeyX => "KeyX",
        Key::KeyY => "KeyY",
        Key::KeyZ => "KeyZ",

        // Number keys 0-9
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",

        // Punctuation and symbols
        Key::Minus => "Minus", // -
        Key::Equal => "Equal", // =
        Key::Comma => "Comma", // ,
        Key::Dot => "Period", // .
        Key::Quote => "Quote", // '
        Key::BackQuote => "Backquote", // `
        Key::Slash => "Slash", // /
        Key::LeftBracket => "BracketLeft", // [
        Key::RightBracket => "BracketRight", // ]
        Key::BackSlash => "Backslash", // \
        Key::SemiColon => "Semicolon", // ;
        Key::IntlBackslash => "IntlBackslash", // Additional backslash key on some keyboards

        // Numpad keys
        Key::KpReturn => "NumpadEnter",
        Key::KpMinus => "NumpadSubtract",
        Key::KpPlus => "NumpadAdd",
        Key::KpMultiply => "NumpadMultiply",
        Key::KpDivide => "NumpadDivide",
        Key::Kp0 => "Numpad0",
        Key::Kp1 => "Numpad1",
        Key::Kp2 => "Numpad2",
        Key::Kp3 => "Numpad3",
        Key::Kp4 => "Numpad4",
        Key::Kp5 => "Numpad5",
        Key::Kp6 => "Numpad6",
        Key::Kp7 => "Numpad7",
        Key::Kp8 => "Numpad8",
        Key::Kp9 => "Numpad9",
        Key::KpDelete => "NumpadDecimal",

        // Additional system keys
        Key::NumLock => "NumLock",
        Key::ScrollLock => "ScrollLock",
        Key::PrintScreen => "PrintScreen",
        Key::Pause => "Pause",
        Key::Function => "Fn", // Special function key on some keyboards

        // Unknown or unmapped keys
        Key::Unknown(_) => "", // Handle unknown keys gracefully
    }
}

// ==========================================================
// Non-macOS implementation (keep your original rdev approach)
// ==========================================================
#[cfg(not(target_os = "macos"))]
mod non_macos_impl {
    use super::*;
    use rdev::{listen, Button, Event, EventType};
    // Maps a mouse button to its standardized code
    fn map_button_to_code(button: Button) -> &'static str {
        match button {
            Button::Left => "MouseLeft",
            Button::Right => "MouseRight",
            Button::Middle => "MouseMiddle",
            Button::Unknown(code) => {
                // Handle additional mouse buttons (side buttons, etc.)
                match code {
                    4 => "Mouse4", // Back/Previous
                    5 => "Mouse5", // Forward/Next
                    6 => "Mouse6", // Extra button 1
                    7 => "Mouse7", // Extra button 2
                    8 => "Mouse8", // Extra button 3
                    _ => "MouseUnknown",
                }
            }
        }
    }

    /// Start a unified input listener that handles both keyboard and mouse events
    /// This solves the issue where rdev can only have one global listener at a time
    pub fn start_unified_input_listener(
        keyboard_tx: Sender<String>,
        mouse_tx: Sender<String>,
        hotkey_tx: Sender<String>
    ) {
        println!("🎮 Starting unified input listener (keyboard + mouse + hotkeys)...");

        thread::spawn(move || {
            println!("🎮 Unified input listener thread started");

            // Separate state tracking for keyboard and mouse
            let keyboard_last_press = Arc::new(Mutex::new(Instant::now()));
            let mouse_last_press = Arc::new(Mutex::new(Instant::now()));
            let pressed_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
            let pressed_buttons = Arc::new(Mutex::new(HashSet::<String>::new()));

            // Track pressed modifier keys for hotkey detection
            let mut ctrl_pressed = false;
            let mut alt_pressed = false;
            let result = listen(move |event: Event| {
                match event.event_type {
                    // ===== KEYBOARD EVENTS =====
                    EventType::KeyPress(key) => {
                        let key_code = map_key_to_code(key);
                        if !key_code.is_empty() {
                            // println!("⌨️ Key Pressed: {}", key_code);
                            // println!("🔍 DEBUG: Key event detected: {}", key_code);

                            // Track modifier keys for hotkey detection
                            match key_code {
                                "ControlLeft" | "ControlRight" => {
                                    ctrl_pressed = true;
                                }
                                "AltLeft" | "AltRight" => {
                                    alt_pressed = true;
                                }
                                "KeyM" => {
                                    // Check for Ctrl+Alt+M hotkey combination
                                    if ctrl_pressed && alt_pressed {
                                        println!(
                                            "🔥 Hotkey detected: Ctrl+Alt+M - Toggling global sound"
                                        );
                                        let _ = hotkey_tx.send("TOGGLE_SOUND".to_string());
                                        return; // Don't process this as a regular key event
                                    }
                                }
                                _ => {}
                            }

                            // Check if key is already pressed
                            let mut pressed = pressed_keys.lock().unwrap();
                            if pressed.contains(&key_code.to_string()) {
                                return; // Key already pressed, ignore
                            }
                            pressed.insert(key_code.to_string());
                            drop(pressed); // Apply debounce and detect rapid key events
                            let now = Instant::now();
                            let mut last = keyboard_last_press.lock().unwrap();
                            let time_since_last = now.duration_since(*last);

                            // Special handling for Backspace key - skip if too rapid (< 10ms)
                            if key_code == "Backspace" && time_since_last < Duration::from_millis(10) {
                                return; // Skip this Backspace event entirely
                            }

                            if time_since_last > Duration::from_millis(1) {
                                *last = now;
                                let _ = keyboard_tx.send(key_code.to_string());
                            }
                        }
                    }
                    EventType::KeyRelease(key) => {
                        let key_code = map_key_to_code(key);
                        if !key_code.is_empty() {
                            // println!("⌨️ Key Released: {}", key_code);

                            // Track modifier key releases for hotkey detection
                            match key_code {
                                "ControlLeft" | "ControlRight" => {
                                    ctrl_pressed = false;
                                }
                                "AltLeft" | "AltRight" => {
                                    alt_pressed = false;
                                }
                                _ => {}
                            }

                            // Remove key from pressed set
                            let mut pressed = pressed_keys.lock().unwrap();
                            pressed.remove(&key_code.to_string());
                            drop(pressed);

                            let _ = keyboard_tx.send(format!("UP:{}", key_code));
                        }
                    }

                    // ===== MOUSE EVENTS =====
                    EventType::ButtonPress(button) => {
                        let button_code = map_button_to_code(button);
                        if !button_code.is_empty() && button_code != "MouseUnknown" {
                            // println!("🖱️ Mouse Button Pressed: {}", button_code);
                            // println!("🔍 DEBUG: Mouse event detected: {}", button_code);

                            // Check if button is already pressed
                            let mut pressed = pressed_buttons.lock().unwrap();
                            if pressed.contains(&button_code.to_string()) {
                                return; // Button already pressed, ignore
                            }
                            pressed.insert(button_code.to_string());
                            drop(pressed); // Apply debounce and detect rapid mouse events
                            let now = Instant::now();
                            let mut last = mouse_last_press.lock().unwrap();
                            let time_since_last = now.duration_since(*last);

                            // General rapid event detection (< 60ms) - log but still process
                            if
                                time_since_last < Duration::from_millis(60) &&
                                time_since_last > Duration::from_millis(1)
                            {
                                println!(
                                    "⚡ RAPID MOUSE EVENT detected: '{}' fired {:.1}ms after previous mouse event",
                                    button_code,
                                    time_since_last.as_millis()
                                );
                            }

                            if time_since_last > Duration::from_millis(1) {
                                *last = now;
                                let _ = mouse_tx.send(button_code.to_string());
                            }
                        }
                    }
                    EventType::ButtonRelease(button) => {
                        let button_code = map_button_to_code(button);
                        if !button_code.is_empty() && button_code != "MouseUnknown" {
                            // println!("🖱️ Mouse Button Released: {}", button_code);

                            // Remove button from pressed set
                            let mut pressed = pressed_buttons.lock().unwrap();
                            pressed.remove(&button_code.to_string());
                            drop(pressed);

                            let _ = mouse_tx.send(format!("UP:{}", button_code));
                        }
                    }
                    // Skip mouse wheel events for now
                    EventType::Wheel { delta_x: _, delta_y: _ } => {
                        // let wheel_event = if delta_y > 0 {
                        //     "MouseWheelUp"
                        // } else if delta_y < 0 {
                        //     "MouseWheelDown"
                        // } else {
                        //     return; // No vertical scroll, ignore
                        // };

                        // println!("🖱️ Mouse Wheel: {}", wheel_event);

                        // // Apply longer debounce for wheel events
                        // let now = Instant::now();
                        // let mut last = mouse_last_press.lock().unwrap();
                        // if now.duration_since(*last) > Duration::from_millis(50) {
                        //     *last = now;
                        //     let _ = mouse_tx.send(wheel_event.to_string());
                        // }
                    }
                    EventType::MouseMove { x: _, y: _ } => {
                        // Mouse move events are too noisy, ignore them
                        // println!("🖱️ Mouse Move: ({}, {})", x, y);
                    }
                }
            });

            if let Err(error) = result {
                eprintln!("❌ Unified input listener error: {:?}", error);
            }
        });
    }
}

#[cfg(not(target_os = "macos"))]
pub use non_macos_impl::start_unified_input_listener;

// ==============================
// macOS implementation (CGEventTap) – core-graphics 0.25
// ==============================
#[cfg(target_os = "macos")]
mod macos_impl {

    use core_foundation::base::{kCFAllocatorDefault, TCFType};
    use core_foundation::mach_port::{CFMachPort, CFMachPortCreateRunLoopSource};
    use core_foundation::runloop::{
        kCFRunLoopCommonModes, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRun,
    };

    use core_graphics::event::{
        CallbackResult, CGEvent, CGEventField, CGEventTap, CGEventTapLocation,
        CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType, CGEventFlags
    };

    use std::collections::HashSet;
    use std::sync::{mpsc::Sender, Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use std::sync::OnceLock; // NEW: for storing tap port
    use std::sync::atomic::{AtomicU64, Ordering};
    use objc::{class, msg_send, sel, sel_impl};
    use objc::runtime::Object;

    // --- NEW: FFI to re-enable CGEventTap ---
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventTapEnable(tap: core_foundation::mach_port::CFMachPortRef, enable: bool);
        fn CGEventTapIsEnabled(tap: core_foundation::mach_port::CFMachPortRef) -> bool;
    }

    // Global slot to hold the pointer value (as usize) — usize is Sync.
    static TAP_PORT_RAW: OnceLock<usize> = OnceLock::new();
    static LAST_EVENT_NS: AtomicU64 = AtomicU64::new(0);
    static LAST_REPAIR_NS: AtomicU64 = AtomicU64::new(0);

    // Quartz CGEventField numeric constants (stable across crate versions)
    const KCG_KEYBOARD_EVENT_KEYCODE: CGEventField = 9;
    const KCG_MOUSE_EVENT_BUTTON_NUMBER: CGEventField = 11;
    // Long idle will consider state as “suspicious” and clean it up to avoid key jamming
    const STALE_CLEAR_MS: u64 = 1200;

    // Keep the activity token so the OS knows we are still "busy" (no App Nap)
    static ACTIVITY_TOKEN: OnceLock<usize> = OnceLock::new();

    #[inline]
    fn begin_no_nap_activity() {
        // Keep process "awake" while listening for input, no background noise needed
        // Use NSProcessInfo beginActivityWithOptions:reason:
        // Use only lightweight flag: NSActivityIdleSystemSleepDisabled = 1 << 20
        unsafe {
            let pi: *mut Object = msg_send![class!(NSProcessInfo), processInfo];
            let opts: u64 = 1u64 << 20; // NSActivityIdleSystemSleepDisabled
            let reason: *mut Object = core::ptr::null_mut(); // nil
            let token: *mut Object = msg_send![pi, beginActivityWithOptions: opts reason: reason];
            let _ = ACTIVITY_TOKEN.set(token as usize);
        }
    }

    fn map_macos_keycode_to_code(kc: u16) -> &'static str {
        match kc {
            0 => "KeyA",   1 => "KeyS",   2 => "KeyD",    3 => "KeyF",
            4 => "KeyH",   5 => "KeyG",   6 => "KeyZ",    7 => "KeyX",
            8 => "KeyC",   9 => "KeyV",  11 => "KeyB",   12 => "KeyQ",
            13 => "KeyW", 14 => "KeyE",  15 => "KeyR",   16 => "KeyY",
            17 => "KeyT", 31 => "KeyO",  32 => "KeyU",   34 => "KeyI",
            35 => "KeyP", 37 => "KeyL",  38 => "KeyJ",   40 => "KeyK",
            45 => "KeyN", 46 => "KeyM",
            18 => "Digit1", 19 => "Digit2", 20 => "Digit3", 21 => "Digit4",
            22 => "Digit6", 23 => "Digit5", 25 => "Digit9", 26 => "Digit7",
            28 => "Digit8", 29 => "Digit0",
            24 => "Equal", 27 => "Minus", 30 => "BracketRight", 33 => "BracketLeft",
            39 => "Quote", 41 => "Semicolon", 42 => "Backslash",
            43 => "Comma", 44 => "Slash", 47 => "Period", 50 => "Backquote",
            36 => "Enter", 48 => "Tab", 49 => "Space", 51 => "Backspace", 53 => "Escape",
            55 => "MetaLeft", 54 => "MetaRight",
            56 => "ShiftLeft", 60 => "ShiftRight",
            57 => "CapsLock",
            58 => "AltLeft", 61 => "AltRight",
            59 => "ControlLeft", 62 => "ControlRight",
            115 => "Home", 116 => "PageUp", 117 => "Delete", 119 => "End", 121 => "PageDown",
            123 => "ArrowLeft", 124 => "ArrowRight", 125 => "ArrowDown", 126 => "ArrowUp",
            122 => "F1", 120 => "F2", 99 => "F3", 118 => "F4", 96 => "F5",
            97 => "F6", 98 => "F7", 100 => "F8", 101 => "F9", 109 => "F10",
            103 => "F11", 111 => "F12",
            76 => "NumpadEnter", 78 => "NumpadSubtract", 69 => "NumpadAdd",
            67 => "NumpadMultiply", 75 => "NumpadDivide",
            65 => "NumpadDecimal", 81 => "NumpadEqual",
            82 => "Numpad0", 83 => "Numpad1", 84 => "Numpad2", 85 => "Numpad3",
            86 => "Numpad4", 87 => "Numpad5", 88 => "Numpad6", 89 => "Numpad7",
            91 => "Numpad8", 92 => "Numpad9",
            _ => "",
        }
    }

    fn map_macos_mouse_button(btn_number: i64) -> &'static str {
        match btn_number {
            0 => "MouseLeft",
            1 => "MouseRight",
            2 => "MouseMiddle",
            3 => "Mouse4",
            4 => "Mouse5",
            5 => "Mouse6",
            6 => "Mouse7",
            7 => "Mouse8",
            _ => "MouseUnknown",
        }
    }

    struct CallbackData {
        keyboard_tx: Sender<String>,
        mouse_tx: Sender<String>,
        hotkey_tx: Sender<String>,
        keyboard_last_press: Arc<Mutex<Instant>>,
        mouse_last_press: Arc<Mutex<Instant>>,
        pressed_keys: Arc<Mutex<HashSet<String>>>,
        pressed_buttons: Arc<Mutex<HashSet<String>>>,
        ctrl_pressed: Arc<Mutex<bool>>,
        alt_pressed: Arc<Mutex<bool>>,
    }

    pub fn start_unified_input_listener(
        keyboard_tx: Sender<String>,
        mouse_tx: Sender<String>,
        hotkey_tx: Sender<String>,
    ) {
        println!("🎮 (macOS) Starting unified input listener via CGEventTap");

        thread::spawn(move || {
            let keyboard_last_press = Arc::new(Mutex::new(Instant::now()));
            let mouse_last_press = Arc::new(Mutex::new(Instant::now()));
            let pressed_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
            let pressed_buttons = Arc::new(Mutex::new(HashSet::<String>::new()));
            let ctrl_pressed = Arc::new(Mutex::new(false));
            let alt_pressed = Arc::new(Mutex::new(false));

            let data = std::sync::Arc::new(CallbackData {
                keyboard_tx,
                mouse_tx,
                hotkey_tx,
                keyboard_last_press,
                mouse_last_press,
                pressed_keys,
                pressed_buttons,
                ctrl_pressed,
                alt_pressed,
            }); 

            // In 0.25, CGEventTap::new takes a Vec<CGEventType>
            let events = vec![
                CGEventType::KeyDown,
                CGEventType::KeyUp,
                CGEventType::FlagsChanged,
                CGEventType::LeftMouseDown,
                CGEventType::LeftMouseUp,
                CGEventType::RightMouseDown,
                CGEventType::RightMouseUp,
                CGEventType::OtherMouseDown,
                CGEventType::OtherMouseUp,
                // NEW: watch for tap being disabled by system
                CGEventType::TapDisabledByTimeout,
                CGEventType::TapDisabledByUserInput,
            ];

            // FIX: capture an Arc and use it immutably; mutate through Mutex inside fields
            let data_for_cb = std::sync::Arc::clone(&data);

            let cb = move |_proxy: CGEventTapProxy, etype: CGEventType, event: &CGEvent| -> CallbackResult {
                // NEW: if the event tap got disabled, immediately re-enable and skip further handling
                if matches!(etype, CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput) {
                    if let Some(raw) = TAP_PORT_RAW.get() {
                        let p = *raw as core_foundation::mach_port::CFMachPortRef;
                        unsafe { CGEventTapEnable(p, true); }
                    }
                    LAST_EVENT_NS.store(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64, Ordering::Relaxed);
                    return CallbackResult::Keep;
                }

                // We do NOT mutate the captured variable itself, so the closure is Fn
                let d: &CallbackData = &data_for_cb;

                // ===== Keyboard =====
                if matches!(etype, CGEventType::KeyDown | CGEventType::KeyUp) {
                    // CLEAN STATE AFTER IDLE: update milestone & clear if idle exceeds threshold
                    let now = Instant::now();
                    let idle_and_update = {
                        let mut last = d.keyboard_last_press.lock().unwrap();
                        let idle = now.duration_since(*last);
                        // Clear stuck state if idle exceeds threshold
                        if idle > Duration::from_millis(STALE_CLEAR_MS) {
                            d.pressed_keys.lock().unwrap().clear();
                            d.pressed_buttons.lock().unwrap().clear();
                        }
                        // ONLY update after idle calculation
                        *last = now;
                        idle
                    };

                    let kc = event.get_integer_value_field(KCG_KEYBOARD_EVENT_KEYCODE) as u16;
                    let code = map_macos_keycode_to_code(kc);
                    if !code.is_empty() {
                        // Hotkey: Ctrl+Alt+M
                        {
                            let mut ctrl = d.ctrl_pressed.lock().unwrap();
                            let mut alt = d.alt_pressed.lock().unwrap();
                            if matches!(code, "ControlLeft" | "ControlRight") {
                                *ctrl = matches!(etype, CGEventType::KeyDown);
                            } else if matches!(code, "AltLeft" | "AltRight") {
                                *alt = matches!(etype, CGEventType::KeyDown);
                            } else if code == "KeyM" && matches!(etype, CGEventType::KeyDown) && *ctrl && *alt {
                                let _ = d.hotkey_tx.send("TOGGLE_SOUND".to_string());                    
                                return CallbackResult::Keep; // pass through
                            }
                        }

                        if matches!(etype, CGEventType::KeyDown) {
                            let mut pressed = d.pressed_keys.lock().unwrap();
                            if pressed.contains(code) {
                                return CallbackResult::Keep;
                            }
                            pressed.insert(code.to_string());
                            drop(pressed);

                            let dt = idle_and_update;

                            if code == "Backspace" && dt < Duration::from_millis(10) {
                                return CallbackResult::Keep;
                            }
                            if dt > Duration::from_millis(1) {
                                let _ = d.keyboard_tx.send(code.to_string());
                            }
                        } else {
                            let mut pressed = d.pressed_keys.lock().unwrap();
                            pressed.remove(code);
                            drop(pressed);
                            let _ = d.keyboard_tx.send(format!("UP:{}", code));
                        }
                    }
                    LAST_EVENT_NS.store(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64, Ordering::Relaxed);
                    return CallbackResult::Keep;
                }

                // ===== Modifier (flags changed) =====
                if matches!(etype, CGEventType::FlagsChanged) {
                    // CLEAN STATE AFTER IDLE (modifier is also keyboard event)
                    let now = Instant::now();
                    let idle_and_update = {
                        let mut last = d.mouse_last_press.lock().unwrap();
                        let idle = now.duration_since(*last);
                        if idle > Duration::from_millis(STALE_CLEAR_MS) {
                            d.pressed_keys.lock().unwrap().clear();
                            d.pressed_buttons.lock().unwrap().clear();
                        }
                        *last = now;
                        idle
                    };               
                    // 1) Read flags (core-graphics 0.25 uses get_flags())
                    let flags: CGEventFlags = event.get_flags();
                    let ctrl_down  = flags.contains(CGEventFlags::CGEventFlagControl);
                    let alt_down   = flags.contains(CGEventFlags::CGEventFlagAlternate);
                    let cmd_down   = flags.contains(CGEventFlags::CGEventFlagCommand);
                    let shift_down = flags.contains(CGEventFlags::CGEventFlagShift);
                    // let caps_on    = flags.contains(CGEventFlags::CGEventFlagAlphaShift);

                    // Keep hotkey state in sync (your Ctrl+Alt+M)
                    { *data_for_cb.ctrl_pressed.lock().unwrap() = ctrl_down; }
                    { *data_for_cb.alt_pressed.lock().unwrap()  = alt_down; }

                    // 2) Figure out WHICH modifier key changed from the keycode
                    let kc = event.get_integer_value_field(KCG_KEYBOARD_EVENT_KEYCODE) as u16;
                    let code = map_macos_keycode_to_code(kc);
                    if !code.is_empty() {
                        // For that specific code, compute "down?" from flags
                        let is_down = match code {
                            "ControlLeft" | "ControlRight" => ctrl_down,
                            "AltLeft"     | "AltRight"     => alt_down,
                            "MetaLeft"    | "MetaRight"    => cmd_down,
                            "ShiftLeft"   | "ShiftRight"   => shift_down,
                            // "CapsLock"                         => caps_on, // toggle key
                            _ => false,
                        };

                        // 3) Emit synthetic KeyDown/KeyUp to your pipeline (with de-dup)
                        if is_down {
                            let mut pressed = d.pressed_keys.lock().unwrap();
                            if !pressed.contains(code) {
                                pressed.insert(code.to_string());
                                drop(pressed);
                                let _ = d.keyboard_tx.send(code.to_string());           // KeyDown
                            }
                        } else {
                            let mut pressed = d.pressed_keys.lock().unwrap();
                            if pressed.remove(code) {
                                drop(pressed);
                                let _ = d.keyboard_tx.send(format!("UP:{}", code));    // KeyUp
                            }
                        }
                    }

                    // Heartbeat
                    LAST_EVENT_NS.store(
                        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64,
                        Ordering::Relaxed,
                    );
                    return CallbackResult::Keep;
                }

                // ===== Mouse =====
                if matches!(
                    etype,
                    CGEventType::LeftMouseDown
                        | CGEventType::LeftMouseUp
                        | CGEventType::RightMouseDown
                        | CGEventType::RightMouseUp
                        | CGEventType::OtherMouseDown
                        | CGEventType::OtherMouseUp
                ) {

                    // CLEAN STATE AFTER MOUSE IDLE
                    let now = Instant::now();
                    let idle_and_update = {
                        let mut last = d.mouse_last_press.lock().unwrap();
                        let idle = now.duration_since(*last);
                        if idle > Duration::from_millis(STALE_CLEAR_MS) {
                            d.pressed_keys.lock().unwrap().clear();
                            d.pressed_buttons.lock().unwrap().clear();
                        }
                        *last = now;
                        idle
                    };

                    let btn = event.get_integer_value_field(KCG_MOUSE_EVENT_BUTTON_NUMBER);
                    let btn_code = map_macos_mouse_button(btn);

                    if btn_code != "MouseUnknown" {  
                        if matches!(
                            etype,
                            CGEventType::LeftMouseDown
                                | CGEventType::RightMouseDown
                                | CGEventType::OtherMouseDown
                        ) {
                            let mut pressed = d.pressed_buttons.lock().unwrap();
                            if pressed.contains(btn_code) {
                                return CallbackResult::Keep;
                            }
                            pressed.insert(btn_code.to_string());
                            drop(pressed);

                            let dt = idle_and_update;
                            if dt < Duration::from_millis(60) && dt > Duration::from_millis(1) {
                                println!("⚡ RAPID MOUSE EVENT: '{}' after {}ms", btn_code, dt.as_millis());
                            }

                            if dt > Duration::from_millis(1) {
                                let _ = d.mouse_tx.send(btn_code.to_string());
                            }
                        } else {
                            let mut pressed = d.pressed_buttons.lock().unwrap();
                            pressed.remove(btn_code);
                            drop(pressed);
                            let _ = d.mouse_tx.send(format!("UP:{}", btn_code));
                        }
                    }
                    LAST_EVENT_NS.store(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64, Ordering::Relaxed);
                    return CallbackResult::Keep;
                }

                // Pass through other events unchanged
                CallbackResult::Keep
            };

            let tap = CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly, // NEW: safer, we only listen
                events,
                cb,
            )
            .expect("Failed to create CGEventTap (enable Accessibility & Input Monitoring)");

            unsafe {
                // Add to runloop
                let port: &CFMachPort = tap.mach_port(); // core-foundation 0.10 type
                let port_ref = port.as_concrete_TypeRef();
                // store as usize to avoid Send/Sync bounds on raw pointers
                let _ = TAP_PORT_RAW.set(port_ref as usize);
                
                // Enable immediately after creation (in case it is disabled by default by the system)
                unsafe { CGEventTapEnable(port_ref, true); }
                // NEW: anti App Nap / energy saving sleep thread tap/audio
                begin_no_nap_activity();

                let source = CFMachPortCreateRunLoopSource(
                    kCFAllocatorDefault,
                    port_ref,
                    0,
                );
                let rl = CFRunLoopGetCurrent();
                CFRunLoopAddSource(rl, source, kCFRunLoopCommonModes);
        
                std::thread::spawn(|| {
                    loop {
                        std::thread::sleep(Duration::from_secs(2));
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
                        let last = LAST_EVENT_NS.load(Ordering::Relaxed);
                        // If no events for 5s, and tap is disabled, re-enable it
                        if last != 0 && now.saturating_sub(last) > 5_000_000_000 {
                            if let Some(raw) = TAP_PORT_RAW.get() {
                                let p = *raw as core_foundation::mach_port::CFMachPortRef;
                                // Check if tap is enabled
                                let is_enabled = unsafe { CGEventTapIsEnabled(p) };
                                if !is_enabled {
                                    // Throttle: only re-enable once every 10s
                                    let since_fix = now.saturating_sub(LAST_REPAIR_NS.load(Ordering::Relaxed));
                                    if since_fix > 10_000_000_000 {
                                        unsafe { CGEventTapEnable(p, true); }
                                        LAST_REPAIR_NS.store(now, Ordering::Relaxed);
                                        println!("🩹 Watchdog re-enabled CGEventTap");
                                    }
                                }
                            }
                        }
                    }
                });
                println!("✅ CGEventTap installed (check System Settings > Privacy & Security)");
                CFRunLoopRun();
            }
        });
    }
}

// Re-export for macOS
#[cfg(target_os = "macos")]
pub use macos_impl::start_unified_input_listener;
