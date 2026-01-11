# Agent Guide for EnvHub

Welcome, Agent. This document serves as your onboarding guide to the EnvHub project. It outlines the project structure, design principles, and key context you need to effective contribute.

## Project Overview

EnvHub is a tool for managing environment variables across different applications and profiles. It allows users to define "Apps" (e.g., specific projects or tools) and "Profiles" (e.g., dev, prod, staging) with associated environment variables.

## Directory Structure

The project is a Rust workspace with some web/Tauri components (though the current focus has been the TUI).

### Rust Workspace (`crates/`)

*   **`crates/envhub-core/`**:
    *   **Purpose**: Contains the core business logic, configuration management, and data structures.
    *   **Key Files**:
        *   `src/state.rs`: Manages the loading, validating, and saving of the user configuration (`config.json`).
        *   `src/lib.rs`: entry of the crate.
    *   **Data Storage**: User configuration is stored in `config.json` (formerly `state.json`) in the user's config directory (e.g., `~/.config/envhub/config.json`).

*   **`crates/envhub-tui/`**:
    *   **Purpose**: The Terminal User Interface (TUI) implementation using `ratatui`.
    *   **Architecture**:
        *   `src/main.rs`: Entry point and main event loop.
        *   `src/app.rs`: Application state, input handling, and logic. Uses `InputState` for modals and `App` struct for global state.
        *   `src/ui.rs`: Pure rendering logic. Contains the `render` function and `Theme` definitions.
    *   **Style**: Uses a "Cyan/DarkGray" theme. Header and borders use non-intrusive colors, while focused elements use the Primary color (Cyan).

*   **`crates/envhub-launcher/`**:
    *   **Purpose**: The CLI entry point or process wrapper (Details to be verified during specific tasks).

### Web/Desktop (`src/`, `src-tauri/`)
*   There appears to be a SvelteKit + Tauri setup in the root, likely for a GUI version of EnvHub.
*   **Status**: The recent focus has been entirely on the TUI (`envhub-tui`).

## Documentation

*   **`docs/design.md`**: High-level design document.
*   **`docs/core-launcher.md`**: Documentation specific to the launcher component.

## Development Principles & Context

### 1. Separation of Concerns (TUI)
We strictly separate the TUI Logic from Rendering.
*   **Logic (`app.rs`)**: Handles key events, state mutations, and input validation. It does *not* know about drawing primitives.
*   **Rendering (`ui.rs`)**: Takes a read-only reference to `App` and draws the frame. It does *not* mutate state.

### 2. Configuration (`envhub-core`)
*   **`config.json`**: The single source of truth for user data.
*   **Naming**: We recently renamed `state.json` to `config.json` to better reflect its purpose.
*   **Migration**: The migration code was intentionally removed to keep the core clean, assuming users are now on `config.json`.

### 3. UX Guidelines
*   **Visual Feedback**: Active elements should always be visually distinct (e.g., colored borders, bold text).
*   **Snapping**: When navigating lists (e.g., switching Apps), the selection should "snap" to the active profile/item if possible, rather than resetting to default.
*   **Theming**: Stick to the defined `Theme` struct in `ui.rs` for consistency.

### 4. Code Style
*   Run `cargo check` and `cargo fmt` frequently.
*   Prioritize explicit error handling over `unwrap()`.

### 5. Web UI Guidelines
*   **Component Library**: You MUST use **shadcn-svelte** components for implementing the web interface. Do not build custom UI components from scratch if a suitable primitive exists in shadcn-svelte.
*   **Consistency**: Ensure the web UI aligns with the aesthetic principles of the project (Clean, Modern, consistent coloring).

### 6. Documentation Maintenance
*   **Keep it Live**: This document `AGENTS.md` is the source of truth for high-level context. If you make architectural changes, rename files, or alter design principles that contradict this file, you **MUST** update this file in the same PR/Editor Session. Do not leave it outdated.

## Common Tasks

*   **Running the TUI**: `cargo run -p envhub-tui`
*   **Checking Core**: `cargo check -p envhub-core`

Good luck, Agent.
