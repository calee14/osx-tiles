# osx-tiles

## Current Hotkeys

- **Ctrl+Shift+T** - Test hotkey (prints message)
- **Ctrl+Shift+L** - Tile window to left half
- **Ctrl+Shift+R** - Tile window to right half
- **Ctrl+Shift+Q** - Quit the daemon

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

Or run the compiled binary:

```bash
./target/release/osx-tiles
```

## Running as a Background Daemon

To run in the background:

```bash
./target/release/osx-tiles &
```

To stop it:

```bash
pkill osx-tiles
```

## macOS Permissions

On macOS, you'll need to grant accessibility permissions:

1. Go to System Preferences → Security & Privacy → Privacy → Accessibility
2. Click the lock to make changes
3. Add your terminal application (Terminal.app or iTerm2)
4. You may also need to add the compiled binary itself

## Next Steps

This is a basic framework. The next steps would be:

1. Add macOS Accessibility API integration to query windows
2. Add window positioning/resizing functionality
3. Add more sophisticated hotkey combinations
4. Add configuration file support
5. Create a proper launchd agent for auto-start

## Notes

- The `rdev` library provides cross-platform keyboard/mouse event listening
- On macOS, you need to run this from a terminal that has accessibility permissions
- The daemon will continue running until you press Ctrl+Shift+Q or kill the process
