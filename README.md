# Minesweeper (Rust + egui)

A modern, minimalist desktop **Minesweeper** game built in pure **Rust** (2021 edition) using **egui / eframe**.

All visual icons (mines, flags, trophies, clocks, speaker/mute indicators, and celebration particles) are rendered procedurally via the `egui::Painter` vector API without external bitmap image files, and all sound effects are synthesized procedurally in memory at 44.1 kHz without external audio assets.

---

## Features

- **Minimalist Modern UI**: Smooth Dark (`#0F1115` / `#1A1D24`) and Light themes with animated color interpolation and embedded **Inter** typography (`Regular` & `Bold`).
- **60fps Delta-Time Animations**:
  - Radial center-outward board reset stagger
  - Iterative BFS flood-fill wave propagation with `ease_out_cubic` & `ease_out_back` scaling
  - Flag drop-and-bounce placement and smooth fade-out removal
  - Loss shockwave rings, screen shake, and sequential mine pop-in
  - Victory confetti & 4-point vector star sparkles with count-up timer summary
  - Rolling 4-slot numeric counters for remaining mines and elapsed time
  - Optional **Reduce motion** accessibility mode that snaps animations immediately
- **Procedural Sound Effects (`rodio`)**:
  - Real-time 44.1 kHz mono PCM waveform synthesis generated entirely in Rust code (zero external `.wav`/`.mp3` files)
  - Distinct tactile audio cues for single-cell reveals, ascending pentatonic flood-fill cascades, harmonic dyad chording, flag placement/removal, victory chime arpeggio, sub-bass mine detonation, and UI interactions
  - Dedicated non-blocking background audio thread with graceful headless fallback when no audio output device is present
  - Quick mute/unmute toggle in the top bar (`M` shortcut) and volume slider (`0–100%`) in the Settings panel
- **Fair & Classic Gameplay**:
  - Guaranteed **first-click 3×3 safe zone** (mines are generated after the first click)
  - **Chording** support via middle-click or left-click on revealed numbered cells
  - Automatic pause on window focus loss, concealing the board while paused
  - Persistent settings and best completion times per preset difficulty saved as JSON

---

## Getting Started

### Prerequisites

Ensure you have the stable Rust toolchain (`rustc` and `cargo`) installed via [rustup](https://rustup.rs/).

### Build & Run

```bash
# Run the game in release mode (recommended)
cargo run --release

# Run all unit and headless UI/audio tests
cargo test

# Lint with Clippy (zero warnings policy)
cargo clippy --all-targets -- -D warnings
```

On Windows, release builds (`target/release/minesweeper.exe`) are configured with `windows_subsystem = "windows"` so the application launches directly without a background console window.

---

## Controls & Shortcuts

| Input | Action |
| :--- | :--- |
| **Left Click** (hidden cell) | Reveal cell (first click guarantees a safe 3×3 neighborhood) |
| **Right Click** (hidden cell) | Place or remove a flag |
| **Middle Click** or **Left Click** (revealed number) | **Chord**: reveal all adjacent non-flagged cells when adjacent flags match the cell's number |
| **`N`** or **`F2`** | Restart the current board |
| **`P`** | Pause / Resume the game (also auto-pauses when the window loses focus) |
| **`M`** | Mute / Unmute procedural sound effects |

---

## Difficulty Modes

- **Beginner**: `9 × 9` grid, `10` mines
- **Intermediate**: `16 × 16` grid, `40` mines
- **Expert**: `30 × 16` grid, `99` mines
- **Custom**: Configurable width (`5–50`), height (`5–35`), and mine count with automatic safe-zone validation in the **Settings** panel

---

## Project Structure

```text
minesweeper/
├── Cargo.toml               # Crate metadata, dependencies, and release optimization profile
├── README.md                # Project documentation (English)
├── assets/
│   └── fonts/               # Embedded Inter-Regular.ttf and Inter-Bold.ttf fonts
└── src/
    ├── main.rs              # Application entry point (eframe::App), event loop, and controller
    ├── lib.rs               # Core module declarations
    ├── game/
    │   └── mod.rs           # Pure game state machine: board, 3x3 safe mine generation, BFS flood-fill, chording
    ├── anim/
    │   └── mod.rs           # Delta-time animation manager, easing functions, particles, and rolling counters
    ├── audio/
    │   └── mod.rs           # Procedural 44.1 kHz sound effect synthesizer and non-blocking audio worker thread
    ├── ui/
    │   ├── mod.rs           # Top bar, difficulty bar, animated settings panel, and DPI-aware responsive board
    │   ├── icons.rs         # Pure vector icon primitives rendered via egui::Painter
    │   └── theme.rs         # Dark & Light color palettes and smooth RGBA interpolation
    └── storage/
        └── mod.rs           # JSON persistence (serde + serde_json) for preferences and best times
```
