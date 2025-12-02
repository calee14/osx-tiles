use rdev::{Event, EventType, Key, listen};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

fn main() {
    println!("Tile manager daemon starting...");
    println!("Press Ctrl+Shift+Q to quit");
    println!("Listening for hotkeys...\n");

    let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
    let pressed_keys_clone = pressed_keys.clone();

    if let Err(error) = listen(move |event: Event| callback(event, &pressed_keys_clone)) {
        eprintln!("Error: {:?}", error);
    }
}

fn callback(event: Event, pressed_keys: &Arc<Mutex<HashSet<Key>>>) {
    match event.event_type {
        EventType::KeyPress(key) => {
            pressed_keys.lock().unwrap().insert(key);

            check_hot_keys(&pressed_keys.lock().unwrap());
        }
        EventType::KeyRelease(key) => {
            pressed_keys.lock().unwrap().remove(&key);
        }
        _ => {}
    }
}

fn check_hot_keys(pressed: &HashSet<Key>) {
    // Check for Ctrl+Shift+T
    if pressed.contains(&Key::ControlLeft)
        && pressed.contains(&Key::ShiftLeft)
        && pressed.contains(&Key::KeyT)
    {
        println!("✅ Hotkey detected: Ctrl+Shift+T - Tile windows!");
    }

    // Check for Ctrl+Shift+Q (quit)
    if pressed.contains(&Key::ControlLeft)
        && pressed.contains(&Key::ShiftLeft)
        && pressed.contains(&Key::KeyQ)
    {
        println!("👋 Hotkey detected: Ctrl+Shift+Q - Quitting...");
        std::process::exit(0);
    }
}
