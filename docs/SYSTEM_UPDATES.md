# SARGA OS System Updates

This document describes the SARGA OS system update mechanism and the structure of the update repository.

## Overview

SARGA OS updates are delivered as a set of compiled binaries and a manifest file (`update.toml`). The system settings app checks for updates by fetching the manifest from a centralized repository (typically hosted on GitHub under the `SKYIOUS` organization).

## Update Repository Structure

The update repository (e.g., `SKYIOUS/sarga-updates`) should have the following structure:

```
/
├── update.toml          # Update manifest
├── bin/                 # Updated binaries
│   ├── sash
│   ├── ade
│   └── ...
└── libs/                # Updated libraries
    └── libsarga.so
```

### `update.toml` format

```toml
version = "0.6.0"
release_date = "2026-10-30"
description = "Major UI and stability update"

[[files]]
path = "/bin/sash"
source = "bin/sash"
hash = "sha256:..."

[[files]]
path = "/bin/ade"
source = "bin/ade"
hash = "sha256:..."
```

## Update Process

1. **Check**: The `sargasettings` app performs an HTTP GET request to fetch `update.toml`.
2. **Compare**: The system compares the version in the manifest with the current system version.
3. **Download**: If a newer version is available, the system downloads the manifest and individual files to a staging area (`/tmp/`).
4. **Apply**: Upon restart, the `skyd-update` daemon checks for staged updates and replaces the system binaries in `/bin/` and `/usr/lib/`.

## Developer Guide

### Contribution

Developers can contribute to system updates by submitting pull requests to the main source repository. Once merged, a GitHub Action can be triggered to build and deploy the update to the update repository.

### Filtering Safe Updates

The deployment GitHub Action includes logic to only include "safe" or "updatable" code. This is typically determined by:
- Passing all automated tests.
- Successful builds for all supported architectures (`x86_64`, `aarch64`).
- Absence of the `#no-deploy` tag in the commit message.

### Skipping Deployment

To prevent a specific commit from being deployed to the update repository, include `#no-deploy` in the commit message.

---
**SARGA OS - Fast, elegant, and secure.**
