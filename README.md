# Ogham

Ogham is a desktop EPUB novel manager built with Tauri, React, and Rust. It helps you import EPUB files, normalize their internal structure, read and inspect chapters, maintain the table of contents, process resources, and export the book again as a clean EPUB package.

[![Tauri](https://img.shields.io/badge/Tauri-2.x-orange)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18-blue)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

## Why Ogham

EPUB novels often come from different generators, websites, archives, and personal workflows. Their chapters, images, stylesheets, navigation files, and resource references can be scattered across inconsistent folder layouts.

Ogham treats an EPUB as a manageable project. When a book is imported, it is unpacked into a standardized EPUB workspace so the application can read, inspect, update, and repackage it with fewer surprises.

## Features

- Import EPUB files and normalize them into a predictable `OEBPS` layout.
- Browse chapters, stylesheets, images, fonts, OPF files, NCX files, and EPUB navigation files.
- Read chapters in the built-in reader with chapter navigation, font-size controls, scroll restoration, and image preview.
- View and maintain the table of contents, including item titles, linked chapter files, and ordering.
- Convert chapter text between Simplified and Traditional Chinese.
- Scan chapter content for remote image links, download images, store them in `OEBPS/Images`, and update EPUB references.
- Build a resource index for chapters, styles, images, fonts, and cross-resource references.
- Export the managed workspace back into a standard EPUB archive.

## How It Works

1. Choose an EPUB file from the desktop app.
2. Ogham validates and extracts the package into its managed library.
3. Chapters and resources are reorganized into a standard structure under `OEBPS/`.
4. The app works against the normalized files instead of the original EPUB internals.
5. After edits or processing, the managed structure can be exported as a new EPUB file.

## Getting Started

Ogham is currently developed as a Tauri desktop app. The main development entry point is:

```bash
npm install
npm run tauri dev
```

For setup notes, architecture details, EPUB structure conventions, and validation guidance, see [Docs/Development.md](Docs/Development.md).

## Technology

- Frontend: React, TypeScript, Vite, Zustand, Tauri JavaScript APIs
- Backend: Rust, Tauri 2, ZIP and XML processing, EPUB parsing, Chinese text conversion, image downloading

## Documentation

- [Development notes](Docs/Development.md)
- [EPUB tips](Docs/tips.md)

## License

Ogham is released under the [MIT License](LICENSE).
