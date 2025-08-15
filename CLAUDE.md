# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Messauto is a macOS verification code extraction tool that monitors SMS messages and emails for verification codes, displays them in floating windows, and provides automatic copy/paste functionality.

## Development Commands

### Building and Running
- `cargo build` - Build the project
- `cargo run` - Run the application
- `cargo check` - Check for compilation errors
- `cargo fix --bin "messauto"` - Apply suggested fixes

### Testing Mode
- `cargo run -- --test` - Launch in test mode to verify floating window functionality

### Package Management
- Use `cargo add <package>` to add dependencies
- Use `cargo remove <package>` to remove dependencies
- Never edit Cargo.toml directly for package management

## Architecture

### Core Components

**Main Application (`src/main.rs`)**
- Entry point with two modes: test mode and normal operation
- Manages application lifecycle, logging, and configuration loading
- Handles floating window spawning via IPC

**Configuration System (`src/config.rs`)**
- TOML-based configuration with migration support
- Stores settings: auto_copy, auto_paste, direct_input, launch_at_login, listen_email, floating_window
- Config file location: `~/Library/Application Support/messauto/config.toml`

**Tray Interface (`src/tray.rs`)**
- System tray application with winit event loop
- Menu items for configuration toggles and actions
- Handles user interactions and config persistence

**Monitoring System (`src/monitor/`)**
- **mod.rs**: Coordinates message and email monitoring
- **watcher.rs**: File system monitoring with notify crate
- **message.rs**: SMS/iMessage processing from chat.db
- **email.rs**: Email processing from Apple Mail using emlx crate

**Clipboard & Input (`src/clipboard.rs`)**
- macOS-specific clipboard operations using `pbcopy`
- Auto-paste functionality via enigo library
- Supports both keyboard shortcut (Cmd+V) and direct input methods

**Floating Window (`src/floating_window/`)**
- Separate process spawned via IPC for displaying verification codes
- Built with eframe/egui for GUI rendering
- Displays extracted codes with copy functionality

**IPC (`src/ipc.rs`)**
- Simple process spawning for floating window communication
- Command-line argument parsing for window mode detection

### Key Dependencies
- **eframe**: GUI framework for floating window
- **tray-icon**: System tray functionality
- **enigo**: Keyboard/mouse automation for paste operations
- **notify**: File system monitoring
- **tokio**: Async runtime for monitoring services
- **emlx**: Apple Mail email parsing
- **regex**: Pattern matching for verification codes

### Application Flow
1. Main process loads configuration and starts tray application
2. Tray initializes monitoring services (message + optional email)
3. File watchers monitor system databases for new messages
4. When verification codes detected, spawn floating window processes
5. Clipboard operations provide automatic copy/paste based on user settings

### Configuration Logic
- `direct_input` and `auto_copy`/`auto_paste` are mutually exclusive
- When `direct_input` is enabled, auto copy/paste are disabled
- When `auto_paste` is enabled, `auto_copy` is automatically enabled
- All configuration changes are persisted immediately to TOML file

### Platform Support
- Currently macOS-only due to:
  - Apple Mail integration (emlx)
  - System-specific file paths
  - macOS clipboard operations
  - Launch-at-login functionality