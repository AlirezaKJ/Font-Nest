---
title: FontNest Todos
type: todo
status: active
created: 2026-06-05
updated: 2026-07-18
tags:
    - project
    - todo
    - fontnest
---

# FontNest — Master Todo

> [!info] How to use
> The full task list for [[FontNest]]. Grouped by phase. Roughly top-to-bottom order, but branding and setup can run in parallel. Check items off as you go.

> [!tip] First implementation slice
> Build one thin Windows-first vertical slice before expanding features: scaffold Tauri/SvelteKit → generate one Rust DTO → create the first SQLite migration → discover and persist font families → stream catalogue batches to a virtualized read-only list. No install/uninstall work until this path is tested end to end.

> [!success] Usable dashboard slice — 2026-07-17
> The Svelte/Tauri shell, typed catalogue DTOs, native `fontdb` discovery, responsive Quiet Ledger dashboard, search/filter controls, family inspector, potential-conflict view, and persisted appearance settings are implemented. A guarded Google Fonts discovery, preview, and Windows per-user install slice was added on 2026-07-18. Installed-catalogue persistence, streamed batches, true virtualization, and managed uninstall remain future work.

Related: [[FontNest]] · [[Font Explorer Doc]]

---

## 0. Project Setup & Tooling

- [x] Install prerequisites: Rust toolchain (`rustup`), Node, and Tauri system deps for Windows.
- [x] Scaffold with `create-tauri-app` → Tauri 2 + Svelte 5 / SvelteKit + TypeScript template.
- [x] Init git repo.
- [ ] Create GitHub repo (`FontNest`) and add the remote.
- [x] Add `.gitignore` (node_modules, target, build, .svelte-kit).
- [ ] Pick a license (MIT / Apache-2.0 / GPL) and add `LICENSE`.
- [x] Configure SvelteKit SPA mode: `adapter-static`, `ssr = false`, SPA fallback, strict TypeScript.
- [x] Wire up Tailwind CSS ([[Tailwind CSS]]) + base config.
- [x] Set up Prettier + ESLint + `rustfmt` + `clippy`.
- [x] Configure Rust 2024 and pin the toolchain/dependency versions used for the first release.
- [x] Add Rust lints (`unsafe_code = "warn"`; Clippy `all` + `pedantic`).
- [x] Configure a restrictive CSP and minimal Tauri capabilities; keep shell execution disabled.
- [x] Add editor config / `.editorconfig`.
- [x] Prove the `invoke()` round-trip with a hello-world command.
- [x] Generate shared Rust DTOs into TypeScript with `ts-rs`.
- [ ] Verify generated DTO bindings in CI.
- [ ] Set up commit conventions (Conventional Commits) — optional.

---

## 1. Branding & Assets

- [ ] Finalize the name "FontNest" (check it's free on GitHub, npm, domain, socials).
- [x] Decide brand direction / mood — “The Working Type Archive,” precise, calm, and discerning.
- [x] Pick the UI and display fonts — Geist + Instrument Serif, with system fallbacks.
- [x] Define the Dark and Light Quiet Ledger palettes.
- [ ] **Design the app logo** (the "nest" mark).
    - [ ] Concept sketches.
    - [x] Vector master in SVG (`assets/branding/logo.svg`).
    - [ ] Monochrome + full-color variants.
- [ ] **App icon set** for Tauri bundling:
    - [x] `icon.ico` (Windows).
    - [x] `icon.icns` (macOS).
    - [ ] PNGs (32, 128, 128@2x, 256, 512, 1024).
    - [x] Generate the current Tauri bundle icon outputs from the selected logo.
- [ ] Favicon for any web/landing presence.
- [ ] Social / OG preview image.
- [ ] Wordmark / lockup for README header.
- [ ] Screenshots & mockups for the README and store listings.
- [ ] (Optional) Short demo GIF / video of the app in action.

---

## 2. Rust Domain, Catalogue & Tauri Adapters

- [ ] Define separate domain types with private invariants:
    - [ ] `FontFile` — path, source, format, size, modified time, optional content hash.
    - [ ] `FontFace` — face index, names, weight, width, style, variable axes.
    - [x] `FontFamily` — private native grouping used to produce user-facing family summaries.
    - [x] `ManagedInstallation` — proof FontNest may uninstall a file.
    - [ ] `DuplicateGroup` — exact duplicate or semantic conflict.
- [x] Add serde catalogue DTOs separately from the native family grouping.
- [x] Generate TypeScript DTOs with `ts-rs`; generated files are never edited manually.
- [ ] Add SQLite through Rust-owned `rusqlite`:
    - [ ] Schema and migrations for files, faces, families, installations, duplicates, and scan state.
    - [ ] Repository boundaries for catalogue and managed installations.
        - [x] Phase 1: Rust-owned SQLite repository for Google Fonts managed installations.
    - [ ] Transactional catalogue reconciliation.
- [x] Font discovery through `fontdb`:
    - [x] Handle Windows system + per-user Microsoft font locations.
    - [x] Use `fontdb`'s built-in Windows, macOS, and Linux system-directory discovery.
    - [x] Preserve file identity and face index for `.ttc` / `.otc` collections.
- [ ] Metadata enrichment through `ttf-parser` — names, weight, width, version, foundry, license, axes.
- [ ] Implement two-phase scanning:
    - [ ] Show cached catalogue first.
    - [ ] Compare canonical path + size + modified-time fingerprints.
    - [ ] Parse only new/changed files and remove missing ones.
    - [ ] Defer hashes, glyph counts, script coverage, and variable details.
- [x] Move native font discovery off the command thread with Tauri's async runtime and `spawn_blocking`.
- [ ] Add bounded parallel orchestration for later parsing and hashing work.
- [ ] `scan_installed_fonts(channel)` — stream ordered batches through a Tauri `Channel`; use events only for coarse catalogue notifications.
    - [x] Phase 1: typed `scan_installed_fonts` command returning ordered family summaries.
- [ ] `get_catalog_page` — return minimal, paginated family summaries.
- [ ] `get_font_details` — load deep face/file metadata on demand.
- [ ] `validate_font_file` — confirm real, parseable font; detect corruption.
    - [ ] Install targets: `.ttf`, `.otf`, `.ttc`, `.otc` where supported by the OS.
    - [ ] Preview/import only: `.woff`, `.woff2`.
- [ ] `install_font` — copy to app-managed dir + register with OS (per-user, no admin).
    - [x] Phase 1: verified Google Fonts `.ttf` install for the current Windows user.
- [ ] `uninstall_font(installation_id)` — require a valid managed-installation ledger entry.
- [ ] `reveal_font_file(file_id)` — resolve the trusted path in Rust; never accept an arbitrary frontend path.
- [ ] `watch_font_folders` — `notify-debouncer-full` + incremental reconciliation.
- [ ] Duplicate detection — BLAKE3 exact hashes + separately labelled family/version conflicts.
    - [x] Phase 1: flag potential conflicts when the same family/weight/style appears in multiple files.
- [ ] Scoped internal preview protocol resolving opaque `face_id` values to trusted bytes.
- [ ] Platform adapter trait + Windows/macOS/Linux implementations for locations, install, uninstall, reveal, and cache refresh.
- [ ] Typed errors with `thiserror` → stable error codes + friendly UI messages.
- [ ] Structured diagnostics with `tracing`; never expose internal paths/errors unnecessarily.
- [ ] Treat all UI input as untrusted: validate IDs, canonical paths, and operation preconditions.

### Online font providers

- [x] Bundle a deterministic, commit-pinned Google Fonts manifest; the runtime never needs an API key.
- [x] Add build-time catalogue refresh from the Google Fonts Developer API and official `google/fonts` repository.
- [x] Restrict runtime downloads to trusted HTTPS hosts, reject redirects, cap sizes, and verify Git blob hashes.
- [x] Parse every downloaded font before preview or install and address it through opaque provider IDs.
- [x] Preserve the upstream licence beside FontNest's managed-installation records.
- [x] Add explicit install review/confirmation and refresh the local catalogue after installation.
- [x] Generate the complete release catalogue with a restricted Google Fonts API key.
- [ ] Evaluate Fontshare as a separate provider after confirming a stable catalogue/download API and licence metadata.

---

## 3. Frontend (SvelteKit UI)

- [x] App shell / layout + navigation.
- [x] Theme system (light/dark, follow OS) with persisted preference.
- [x] Typed `invoke()` wrapper using generated `ts-rs` DTOs.
- [ ] Channel consumer for ordered scan batches + correctly cleaned-up event listeners.
- [ ] **Font library screen** — TanStack Virtual grid/list with live previews.
    - [x] Render family summaries, not every face, in the main catalogue.
    - [x] Phase 1: progressive 120-family windows plus `content-visibility` containment.
    - [ ] Load only visible + overscan preview faces.
    - [ ] Use a small LRU of loaded `FontFace` objects and release distant entries.
    - [ ] Never create thousands of simultaneous `@font-face` rules.
- [ ] **Search / filters** — fuzzy name search; filter by weight, style, format, source, duplicate status.
    - [x] Phase 1: immediate term search plus source, format, monospaced, and conflict filters.
- [ ] **Font preview panel** — editable sample text, size/weight controls, glyph/character set, metadata, file path.
    - [x] Phase 1: editable specimen, available weights, size control, face list, provenance, and file names.
    - [ ] Load all family faces, glyph coverage, axes, and named instances on demand.
- [ ] **Duplicate detector UI** — grouped duplicates, choose-which-to-keep.
    - [x] Phase 1: read-only potential-conflict families and file comparison table.
- [ ] **Install / uninstall** — drag-and-drop + file picker; uninstall guarded to app-managed.
    - [x] Safe preview-only picker for `.otf`, `.ttf`, `.woff`, and `.woff2`; files are never installed.
    - [x] Discover, preview, select styles, and confirm per-user installs from Google Fonts.
- [ ] Live updates from folder-watch events.
- [x] Loading skeleton and native scan status.
- [ ] Streamed batch progress indicator for long scans.
- [x] Empty states (no fonts, no search results, no duplicates).
- [x] Error states and accessible status toasts.
- [x] Keyboard shortcuts for search, clearing, and family-row navigation.
- [ ] Settings screen (default install scope, theme, etc.).
    - [x] Phase 1: theme, catalogue density, and default specimen settings.

---

## 4. Cross-Platform Concerns

- [ ] Implement and test the `FontPlatform` adapter contract independently per OS.
- [ ] Windows font install/uninstall + registration tested.
- [ ] macOS font locations + `~/Library/Fonts` install tested.
- [ ] Linux font dirs + `fc-cache` refresh tested.
- [ ] Ensure system/protected fonts can never be deleted.
- [x] Default to per-user installation; system-wide installation is out of scope for v1.
- [ ] Verify preview protocol behavior across WebView2, WKWebView, and WebKitGTK.

---

## 5. Quality & Testing

- [ ] Rust unit tests for domain invariants, parsing, validation, fingerprinting, and dedupe.
    - [x] Phase 1: catalogue ID/style/source/format tests plus Tauri greeting command tests.
- [ ] Repository/migration tests against temporary SQLite databases.
    - [x] Phase 1: managed-installation ledger transaction test against a temporary SQLite database.
- [ ] Tests for install/uninstall on a sandboxed dir.
- [ ] Frontend component tests (Vitest).
    - [x] Typed frontend command-adapter tests (Vitest).
- [ ] Manual QA pass on each OS.
- [ ] Performance corpus with 1,000 / 5,000 / 10,000 faces, including large variable fonts and collections.
- [ ] Measure cold start, cached first paint, reconciliation scan, search latency, scroll FPS, duplicate scan, and steady-state memory.
- [ ] Initial budgets: usable shell <1s; cached library <250ms after frontend readiness; search/filter <50ms; 60 FPS virtualized scrolling.
- [ ] Verify memory returns to a stable range after scrolling through the complete corpus.
- [ ] Run `cargo test`, `cargo clippy -- -D warnings`, `cargo audit`, frontend tests, and type-checking in CI.
- [ ] Accessibility pass (keyboard nav, contrast, focus).
    - [x] Phase 1: keyboard/focus semantics and responsive browser QA for the dashboard slice.

---

## 6. Release & Distribution

- [x] App metadata: name, identifier, version, description in `tauri.conf.json`.
- [ ] Bundle installers (`.msi`/`.exe` for Windows; `.dmg` for macOS; `.AppImage`/`.deb` for Linux).
- [ ] Code signing (Windows + macOS notarization) — research cost/options.
- [ ] Signed auto-update setup with the Tauri updater — optional for v1.
- [ ] Production build: restrictive CSP/capabilities, DevTools and remote debugging disabled.
- [ ] GitHub Actions CI: build + test on push.
- [ ] GitHub Actions release workflow → build artifacts for all OSes.
- [ ] Tag + publish v0.1.0 release with notes.

---

## 7. Docs & Marketing

- [ ] `README.md` — logo header, screenshots, features, install instructions.
- [ ] Feature list / highlights section (like [[BetterSoundCloud]]).
- [ ] Contributing guide + issue templates.
- [ ] (Optional) Landing page / domain (compare with [fontexplorer.net](https://fontexplorer.net/)).
- [ ] Add FontNest to portfolio: [[!Portfolio]] / new project page.
- [ ] Launch post (LinkedIn / socials / Reddit r/fonts, r/rust, r/sveltejs).
- [ ] VirusTotal scan proof in README (you did this for BSC).

---

## 8. Backlog / Nice-to-haves

- [ ] Tag / collection system for organizing fonts.
- [ ] Favorites.
- [ ] Compare-fonts view (lean on [[Font Explorer]] ideas).
- [ ] Export font list / report.
- [ ] Activate/deactivate fonts without uninstalling.
- [ ] Variable-font instance previews.
- [ ] Import from a folder (batch install).
