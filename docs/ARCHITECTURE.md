# SARGA OS Architecture

## Unified Library Design
SARGA OS uses `libsarga` as its primary standard library. This library provides a unified interface for system calls, error handling, and high-level services like GUI widgets and AI integration.

## Process Model
1. **init (PID 1)**: The system entry point. Monitors essential services (login-manager, svc) and respawns them on failure.
2. **login-manager**: Handles user authentication and transitions to the desktop environment.
3. **ade (Sarga Desktop)**: The main windowing system and compositor.
4. **sash**: The standard system shell.

## Security
- **Authentication**: Uses PBKDF2-SHA256 for secure password storage.
- **Isolation**: Standard Unix-like process isolation enforced by the kernel.

## Graphics
- **DRM/GEM**: Kernel-level graphics control via the `gpu` module in `libsarga`.
- **Compositing**: Window-based composition with alpha transparency support.
