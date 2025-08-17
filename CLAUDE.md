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

### Testing
- `cargo test` - Run all tests
- `cargo test parser::tests` - Run parser tests for verification code extraction
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
- Stores settings: auto_paste, auto_enter, direct_input, launch_at_login, listen_email, floating_window
- Stores verification configuration: verification_keywords (trigger words), verification_regex (pattern for code extraction)
- Config file location: `~/Library/Application Support/messauto/config.toml`
- Clipboard functionality is enabled by default unless direct_input or floating_window is enabled
- Auto enter functionality is independent and can be enabled separately
- Verification keywords and regex patterns are user-configurable in config file

**Tray Interface (`src/tray.rs`)**
- System tray application with winit event loop
- Menu items for configuration toggles and actions
- Handles user interactions and config persistence

**Monitoring System (`src/monitor/`)**
- **mod.rs**: Coordinates monitoring services with async actor pattern
- **actor.rs**: MonitorActor that handles start/stop commands for message and email monitoring
- **commands.rs**: Command enum for controlling monitoring services (Start/StopMessageMonitoring, Start/StopEmailMonitoring, GetStatus)
- **watcher.rs**: File system monitoring with notify crate
- **message.rs**: SMS/iMessage processing from chat.db
- **email.rs**: Email processing from Apple Mail using emlx crate

**Clipboard & Input (`src/clipboard.rs`)**
- macOS-specific clipboard operations using `pbcopy`
- Auto-paste functionality via enigo library
- Auto enter functionality that simulates Enter key press
- Supports both keyboard shortcut (Cmd+V) and direct input methods

**Floating Window (`src/floating_window/`)**
- Separate process spawned via IPC for displaying verification codes
- Built with eframe/egui for GUI rendering
- Displays extracted codes with copy functionality

**Code Parser (`src/parser.rs`)**
- Generic verification code extraction using configurable keywords and regex patterns
- First checks for verification keywords (configurable in config file)
- Then extracts codes using configurable regex pattern with digit filtering
- Returns candidate with most digits when multiple matches found
- Includes comprehensive tests for various SMS formats and edge cases

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
2. Tray initializes monitoring actor via `start_monitoring_actor()` 
3. MonitorActor manages separate FileWatcher instances for message and email monitoring
4. Monitoring services can be dynamically started/stopped via MonitorCommand enum
5. File watchers monitor system databases for new messages
6. When verification codes detected, spawn floating window processes
7. Clipboard operations provide automatic copy/paste based on user settings

### Configuration Logic
- `direct_input` and `auto_paste` are mutually exclusive
- When `direct_input` is enabled, `auto_paste` is disabled
- When `floating_window` is enabled, `direct_input` is forced enabled and `auto_paste` is disabled
- `auto_enter` is independent and can be enabled regardless of other settings
- `auto_enter` triggers immediately after verification code input (direct input, auto-paste, or manual paste)
- Clipboard functionality is the default behavior when neither `direct_input` nor `floating_window` is enabled
- Verification keywords (default: 验证码, 动态密码, verification, code, Code, CODE, 인증, 代码) are used to detect SMS/emails containing verification codes
- Verification regex (default: `\b[a-zA-Z0-9][a-zA-Z0-9-]{2,6}[a-zA-Z0-9]\b`) is used to extract candidate codes
- Parser filters candidates to ensure they contain digits and returns the one with most digits
- All configuration changes are persisted immediately to TOML file

### Platform Support
- Currently macOS-only due to:
  - Apple Mail integration (emlx)
  - System-specific file paths
  - macOS clipboard operations
  - Launch-at-login functionality