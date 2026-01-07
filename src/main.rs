use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::CFAllocatorRef;
use core_foundation::base::CFTypeRef;
use core_foundation::base::kCFAllocatorDefault;
use core_foundation::base::{CFRelease, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::display::CGDisplay;
use rdev::{Event, EventType, Key, listen};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// FFI bindings to Accessibility API
type AXUIElementRef = *const std::ffi::c_void;
type AXError = i32;
type pid_t = i32;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCreateApplication(pid: pid_t) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;

}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const *const std::ffi::c_void,
        values: *const *const std::ffi::c_void,
        numValues: isize,
        keyCallBacks: *const std::ffi::c_void,
        valueCallBacks: *const std::ffi::c_void,
    ) -> CFTypeRef;
}

// Accessibility attribute constants
const K_AX_FOCUSED_APPLICATION_ATTRIBUTE: &str = "AXFocusedApplication";
const K_AX_FOCUSED_WINDOW_ATTRIBUTE: &str = "AXFocusedWindow";
const K_AX_WINDOWS_ATTRIBUTE: &str = "AXWindows";
const K_AX_POSITION_ATTRIBUTE: &str = "AXPosition";
const K_AX_SIZE_ATTRIBUTE: &str = "AXSize";
const K_AX_TITLE_ATTRIBUTE: &str = "AXTitle";
const K_AX_MINIMIZED_ATTRIBUTE: &str = "AXMinimized";

#[derive(Debug, Clone)]
struct WindowInfo {
    element: usize, // Store as usize to avoid lifetime issues
    title: String,
    pid: pid_t,
}

fn main() {
    println!("Tile manager daemon starting...");
    println!("Press Ctrl+Shift+T to tile current window to left half");
    println!("Press Ctrl+Shift+A to auto-arrange all visible windows");
    println!("Press Ctrl+Shift+Q to quit");
    println!("Listening for hotkeys...\n");

    let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
    let pressed_keys_clone = pressed_keys.clone();

    // Start a background thread to monitor for new windows (optional)
    let monitor_enabled = Arc::new(Mutex::new(false));
    let monitor_clone = monitor_enabled.clone();

    thread::spawn(move || {
        window_monitor(monitor_clone);
    });

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
    // Check for Ctrl+Shift+T - Tile to left half
    if pressed.contains(&Key::ControlLeft)
        && pressed.contains(&Key::ShiftLeft)
        && pressed.contains(&Key::KeyT)
    {
        println!("✅ Hotkey detected: Ctrl+Shift+T - Tiling window to left half!");
        if let Err(e) = tile_current_window_left() {
            eprintln!("Error tiling window: {}", e);
        }
    }

    // Check for Ctrl+Shift+A - Auto-arrange all windows
    if pressed.contains(&Key::ControlLeft)
        && pressed.contains(&Key::ShiftLeft)
        && pressed.contains(&Key::KeyA)
    {
        println!("✅ Hotkey detected: Ctrl+Shift+A - Auto-arranging all windows!");
        if let Err(e) = auto_arrange_windows() {
            eprintln!("Error arranging windows: {}", e);
        }
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

fn window_monitor(enabled: Arc<Mutex<bool>>) {
    let mut previous_window_count = 0;

    loop {
        thread::sleep(Duration::from_secs(2));

        if !*enabled.lock().unwrap() {
            continue;
        }

        // Check if window count changed
        if let Ok(windows) = get_all_visible_windows() {
            if windows.len() != previous_window_count && windows.len() > 1 {
                println!("🔔 Detected window count change: {} windows", windows.len());
                if let Err(e) = auto_arrange_windows() {
                    eprintln!("Auto-arrange failed: {}", e);
                }
            }
            previous_window_count = windows.len();
        }
    }
}

fn auto_arrange_windows() -> Result<(), String> {
    let windows = get_all_visible_windows()?;

    if windows.is_empty() {
        return Err("No visible windows found".to_string());
    }

    println!("Found {} visible window(s) to arrange", windows.len());

    // Get screen dimensions
    let display = CGDisplay::main();
    let bounds = display.bounds();
    let screen_width = bounds.size.width;
    let screen_height = bounds.size.height;

    // Arrange windows based on count
    match windows.len() {
        1 => {
            // Single window - maximize
            arrange_window(
                windows[0].element as AXUIElementRef,
                0.0,
                0.0,
                screen_width,
                screen_height,
            )?;
            println!("✓ Maximized single window");
        }
        2 => {
            // Two windows - split vertically
            arrange_window(
                windows[0].element as AXUIElementRef,
                0.0,
                0.0,
                screen_width / 2.0,
                screen_height,
            )?;
            arrange_window(
                windows[1].element as AXUIElementRef,
                screen_width / 2.0,
                0.0,
                screen_width / 2.0,
                screen_height,
            )?;
            println!("✓ Arranged 2 windows side-by-side");
        }
        3 => {
            // Three windows - one left, two stacked right
            arrange_window(
                windows[0].element as AXUIElementRef,
                0.0,
                0.0,
                screen_width / 2.0,
                screen_height,
            )?;
            arrange_window(
                windows[1].element as AXUIElementRef,
                screen_width / 2.0,
                0.0,
                screen_width / 2.0,
                screen_height / 2.0,
            )?;
            arrange_window(
                windows[2].element as AXUIElementRef,
                screen_width / 2.0,
                screen_height / 2.0,
                screen_width / 2.0,
                screen_height / 2.0,
            )?;
            println!("✓ Arranged 3 windows (1 left, 2 right)");
        }
        4 => {
            // Four windows - 2x2 grid
            arrange_window(
                windows[0].element as AXUIElementRef,
                0.0,
                0.0,
                screen_width / 2.0,
                screen_height / 2.0,
            )?;
            arrange_window(
                windows[1].element as AXUIElementRef,
                screen_width / 2.0,
                0.0,
                screen_width / 2.0,
                screen_height / 2.0,
            )?;
            arrange_window(
                windows[2].element as AXUIElementRef,
                0.0,
                screen_height / 2.0,
                screen_width / 2.0,
                screen_height / 2.0,
            )?;
            arrange_window(
                windows[3].element as AXUIElementRef,
                screen_width / 2.0,
                screen_height / 2.0,
                screen_width / 2.0,
                screen_height / 2.0,
            )?;
            println!("✓ Arranged 4 windows in 2x2 grid");
        }
        _ => {
            // More than 4 - cascade or grid
            let cols = (windows.len() as f64).sqrt().ceil() as usize;
            let rows = (windows.len() as f64 / cols as f64).ceil() as usize;
            let tile_width = screen_width / cols as f64;
            let tile_height = screen_height / rows as f64;

            for (i, window) in windows.iter().enumerate() {
                let col = i % cols;
                let row = i / cols;
                let x = col as f64 * tile_width;
                let y = row as f64 * tile_height;

                arrange_window(
                    window.element as AXUIElementRef,
                    x,
                    y,
                    tile_width,
                    tile_height,
                )?;
            }
            println!(
                "✓ Arranged {} windows in {}x{} grid",
                windows.len(),
                cols,
                rows
            );
        }
    }

    Ok(())
}

fn get_all_visible_windows() -> Result<Vec<WindowInfo>, String> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return Err("Failed to create system-wide element".to_string());
        }

        // Get running applications using NSWorkspace equivalent
        // For now, we'll get windows from the focused and recently used apps
        let mut all_windows = Vec::new();

        // Get focused app windows
        let focused_app_attr = CFString::new(K_AX_FOCUSED_APPLICATION_ATTRIBUTE);
        let mut focused_app: CFTypeRef = std::ptr::null();
        let result = AXUIElementCopyAttributeValue(
            system_wide,
            focused_app_attr.as_concrete_TypeRef(),
            &mut focused_app,
        );

        if result == 0 && !focused_app.is_null() {
            if let Ok(mut windows) = get_windows_for_app(focused_app as AXUIElementRef) {
                all_windows.append(&mut windows);
            }
            CFRelease(focused_app);
        }

        // For a complete solution, you'd need to enumerate all running applications
        // This would require using NSWorkspace (cocoa crate) or keeping track of PIDs

        Ok(all_windows)
    }
}

fn get_windows_for_app(app_element: AXUIElementRef) -> Result<Vec<WindowInfo>, String> {
    unsafe {
        let windows_attr = CFString::new(K_AX_WINDOWS_ATTRIBUTE);
        let mut windows_ref: CFTypeRef = std::ptr::null();

        let result = AXUIElementCopyAttributeValue(
            app_element,
            windows_attr.as_concrete_TypeRef(),
            &mut windows_ref,
        );

        if result != 0 || windows_ref.is_null() {
            return Ok(Vec::new());
        }

        let windows_array =
            CFArray::<AXUIElementRef>::wrap_under_create_rule(windows_ref as CFArrayRef);
        let mut window_infos = Vec::new();

        for i in 0..windows_array.len() {
            let window = *windows_array.get(i as isize).unwrap();

            // Check if window is minimized
            let minimized_attr = CFString::new(K_AX_MINIMIZED_ATTRIBUTE);
            let mut minimized_ref: CFTypeRef = std::ptr::null();
            let _ = AXUIElementCopyAttributeValue(
                window as AXUIElementRef,
                minimized_attr.as_concrete_TypeRef(),
                &mut minimized_ref,
            );

            // Skip minimized windows
            if !minimized_ref.is_null() {
                CFRelease(minimized_ref);
                // Assume if we got a value, check it (simplified)
                continue;
            }

            // Get window title
            let title_attr = CFString::new(K_AX_TITLE_ATTRIBUTE);
            let mut title_ref: CFTypeRef = std::ptr::null();
            let _ = AXUIElementCopyAttributeValue(
                window as AXUIElementRef,
                title_attr.as_concrete_TypeRef(),
                &mut title_ref,
            );

            let title = if !title_ref.is_null() {
                let title_string = CFString::wrap_under_create_rule(title_ref as CFStringRef);
                title_string.to_string()
            } else {
                "Unknown".to_string()
            };

            window_infos.push(WindowInfo {
                element: window as usize,
                title,
                pid: 0, // We'd need additional API calls to get PID
            });
        }

        Ok(window_infos)
    }
}

fn arrange_window(
    window: AXUIElementRef,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    unsafe {
        // Set position
        let position = create_cgpoint(x, y);
        let position_attr = CFString::new(K_AX_POSITION_ATTRIBUTE);
        let result =
            AXUIElementSetAttributeValue(window, position_attr.as_concrete_TypeRef(), position);
        CFRelease(position);

        if result != 0 {
            return Err("Failed to set window position".to_string());
        }

        // Set size
        let size = create_cgsize(width, height);
        let size_attr = CFString::new(K_AX_SIZE_ATTRIBUTE);
        let result = AXUIElementSetAttributeValue(window, size_attr.as_concrete_TypeRef(), size);
        CFRelease(size);

        if result != 0 {
            return Err("Failed to set window size".to_string());
        }

        Ok(())
    }
}

fn tile_current_window_left() -> Result<(), String> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return Err("Failed to create system-wide element".to_string());
        }

        let focused_app_attr = CFString::new(K_AX_FOCUSED_APPLICATION_ATTRIBUTE);
        let mut focused_app: CFTypeRef = std::ptr::null();
        let result = AXUIElementCopyAttributeValue(
            system_wide,
            focused_app_attr.as_concrete_TypeRef(),
            &mut focused_app,
        );

        if result != 0 || focused_app.is_null() {
            return Err("Failed to get focused application".to_string());
        }

        let focused_window_attr = CFString::new(K_AX_FOCUSED_WINDOW_ATTRIBUTE);
        let mut focused_window: CFTypeRef = std::ptr::null();
        let result = AXUIElementCopyAttributeValue(
            focused_app as AXUIElementRef,
            focused_window_attr.as_concrete_TypeRef(),
            &mut focused_window,
        );

        if result != 0 || focused_window.is_null() {
            CFRelease(focused_app);
            return Err("Failed to get focused window".to_string());
        }

        let display = CGDisplay::main();
        let bounds = display.bounds();
        let screen_width = bounds.size.width;
        let screen_height = bounds.size.height;

        arrange_window(
            focused_window as AXUIElementRef,
            0.0,
            0.0,
            screen_width / 2.0,
            screen_height,
        )?;

        CFRelease(focused_window);
        CFRelease(focused_app);

        println!("✓ Window tiled to left half successfully!");
        Ok(())
    }
}

fn create_cgpoint(x: f64, y: f64) -> CFTypeRef {
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;

    unsafe {
        let x_key = CFString::new("X");
        let y_key = CFString::new("Y");
        let x_val = CFNumber::from(x);
        let y_val = CFNumber::from(y);

        let keys = [
            x_key.as_concrete_TypeRef() as *const std::ffi::c_void,
            y_key.as_concrete_TypeRef() as *const std::ffi::c_void,
        ];
        let values = [
            x_val.as_concrete_TypeRef() as *const std::ffi::c_void,
            y_val.as_concrete_TypeRef() as *const std::ffi::c_void,
        ];

        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            2,
            std::ptr::null(),
            std::ptr::null(),
        )
    }
}

fn create_cgsize(width: f64, height: f64) -> CFTypeRef {
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;

    unsafe {
        let width_key = CFString::new("Width");
        let height_key = CFString::new("Height");
        let width_val = CFNumber::from(width);
        let height_val = CFNumber::from(height);

        let keys = [
            width_key.as_concrete_TypeRef() as *const std::ffi::c_void,
            height_key.as_concrete_TypeRef() as *const std::ffi::c_void,
        ];
        let values = [
            width_val.as_concrete_TypeRef() as *const std::ffi::c_void,
            height_val.as_concrete_TypeRef() as *const std::ffi::c_void,
        ];

        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            2,
            std::ptr::null(),
            std::ptr::null(),
        )
    }
}
