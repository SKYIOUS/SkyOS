# SkyStore - App Store Guide

Welcome to **SkyStore**, the official application repository for **SARGA OS**.
SkyStore allows developers to distribute their applications directly to users via a GitHub-hosted backend.

## How it Works

SkyStore pulls application metadata and binaries from the `SKYIOUS/skystore-apps` repository.
The repository should contain a `manifest.json` file and a `bin/` directory with `.skp` packages.

## Contributing Your App

To add your application to SkyStore, follow these steps:

1. **Package your app**: Use `spkg` to create a `.skp` package for your application.
   ```bash
   spkg pack my-app /path/to/binaries
   ```

2. **Update the Manifest**: Add your app's information to the `manifest.json` in the `skystore-apps` repository.
   ```json
   {
     "name": "My App",
     "id": "my.app.id",
     "version": "1.0.0",
     "description": "A great application for SARGA OS",
     "author": "Your Name",
     "binary": "bin/my-app.skp"
   }
   ```

3. **Submit a Pull Request**: Submit your package and manifest changes to the `SKYIOUS/skystore-apps` repository.

## Binary Hosting

All binaries are stored as `.skp` files in the `bin/` directory of the repository.
SkyStore uses the GitHub API to fetch the list of available apps and download the packages upon user request.

## Security

Every package in SkyStore should ideally be signed. We are working on integrating a signing mechanism to ensure the authenticity of the applications.

---
**SARGA OS - Built for speed, elegance, and ease of use.**
