# FontNest

A lightweight desktop font manager for browsing, searching, previewing, and safely managing the fonts installed on your computer.

FontNest is being built with Tauri 2, SvelteKit in SPA mode, Svelte 5, TypeScript, Tailwind CSS, and Rust 2024. Rust owns trusted operations and domain invariants; the frontend communicates with it only through typed Tauri commands.

## Current implementation

The first usable catalogue and online-install slices are now implemented:

- Native installed-font discovery through `fontdb`, kept off the Tauri command thread
- Rust family/face catalogue DTOs generated into TypeScript by `ts-rs`
- Dark and Light Quiet Ledger themes with OS-following and persisted preferences
- Responsive font-family catalogue with live system-font previews
- Immediate search plus source, format, monospaced, and conflict filters
- Connected inspector with editable specimen text, size, weight, face, format, and provenance details
- Read-only potential-conflict view for repeated family/weight/style combinations across files
- Safe preview-only file picker for `.otf`, `.ttf`, `.woff`, and `.woff2`
- Discover screen backed by a pinned Google Fonts catalogue snapshot
- On-demand previews that are downloaded, integrity-checked, parsed, and kept uninstalled
- Confirmed per-user Windows installs with a Rust-owned SQLite management ledger and preserved licence files
- Loading, empty, error, keyboard, compact-density, and browser-development states

Browser-only development uses clearly labelled sample data because native discovery and installation are available only inside Tauri. The desktop app bundles a complete, commit-pinned Google Fonts API snapshot that can be refreshed without shipping the API key. Catalogue persistence, streamed/virtualized installed-font batches, deeper metadata, and managed uninstall remain future milestones.

## Development

Requirements: pnpm, Node.js, Rust, and the platform prerequisites for Tauri.

```sh
pnpm install
pnpm desktop
```

Use `pnpm dev` for a browser-only UI preview. The interface uses labelled sample data in that mode; run `pnpm desktop` to scan the fonts installed on the computer.

### Refresh the Google Fonts catalogue

The API key is used only by the build-time refresh script. It is never embedded in FontNest or needed by app users.

1. Enable the Google Fonts Developer API in Google Cloud and create an API key restricted to that API.
2. Copy `.env.example` to `.env.local` and set `GOOGLE_FONTS_API_KEY` locally. Do not commit or share the key.
3. Refresh the pinned catalogue:

```sh
pnpm fonts:refresh
```

For deterministic development without a key, regenerate the six-family fixture with `pnpm fonts:fixture`.

## Verification

```sh
pnpm check
pnpm lint
pnpm test
pnpm rust:test
pnpm rust:fmt
pnpm rust:clippy
pnpm build
```

Running the Rust tests regenerates committed DTO bindings under `src/lib/bindings`. Generated files should not be edited by hand.

## Project structure

```text
src/                     SvelteKit UI
src/lib/bindings/        TypeScript generated from Rust DTOs
src/lib/catalogue/       Browser-only development catalogue
src/lib/components/      Dashboard, navigation, inspector, and settings UI
src/lib/tauri/           Typed frontend command adapters
src-tauri/src/catalogue.rs Native installed-font discovery and family grouping
src-tauri/src/google_fonts.rs Trusted online catalogue, preview, and install orchestration
src-tauri/src/font_platform.rs Font parsing and per-user platform registration
src-tauri/src/managed_installations.rs SQLite ownership ledger
src-tauri/resources/     Pinned provider catalogue snapshots
src-tauri/src/           Native commands and domain-facing Rust code
src-tauri/capabilities/  Tauri permission declarations
```

## License

Not chosen yet. This remains an explicit project decision before distribution.
