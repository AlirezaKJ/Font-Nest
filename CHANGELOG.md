# Changelog

All notable changes to FontNest are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

FontNest reads this file to show its release notes in the app. It pulls the copy from the `main`
branch so it can show notes for versions newer than the one you have installed, and falls back to
a bundled copy when you are offline.

## [Unreleased]

### Added

- A What's New screen that lists the changes in each version and marks the one you are running.
  Open it from Settings, or from the prompt that shows up the first time you launch a new build.

### Changed

- Reworked the sidebar so a single highlight slides between whatever item you are on, top to
  bottom. The buttons along the bottom (Fetch fonts, Patch notes, Settings) now widen to show
  their labels when you hover them or when they are active.
- Changed the default preview text to "What is life but a fevered dream".
- The Fetch fonts view now uses the same preview text as the rest of the app, so what you type in
  one place shows up everywhere instead of keeping its own separate copy.

## [0.1.1] - 2026-07-18

### Changed

- Tested the whole auto-update path, from the signed GitHub release feed through downloading,
  checking the signature, and installing.

### Removed

- Took some unused files and folders out of the app bundle to keep the installer smaller.

## [0.1.0] - 2026-07-18

The first public release of FontNest, a desktop app for browsing and managing the fonts installed
on your computer.

### Added

- A library that scans your installed fonts and groups them by family and face. Each family shows
  where it came from, its file formats, how many styles it has, and whether it is proportional or
  monospaced.
- Previews that render each family in its own typeface. Type your own preview text, change the
  size, and pin families to a list you can reorder and compare side by side.
- A closer look at any face: metrics, name and license fields, embedding permissions, and
  variable-font axes. You can render single glyph outlines, browse a character sample, and export
  the parsed data as JSON.
- Google Fonts discovery. Browse, search, and filter the catalogue with real previews. Installs
  happen per user, and every file is checked against the trusted catalogue before it lands, so a
  mismatched download gets rejected.
- Conflict detection that flags families with duplicate or clashing files so you can review them.
- The Quiet Ledger interface: a custom title bar and window layout, matching light and dark
  themes, comfortable and compact density options, keyboard shortcuts, and controls that work
  with the keyboard and screen readers.
- Automatic updates. FontNest checks the official GitHub release feed and installs an update once
  it has verified the signature.
- Apache-2.0 license.

[Unreleased]: https://github.com/AlirezaKJ/Font-Nest/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/AlirezaKJ/Font-Nest/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/AlirezaKJ/Font-Nest/releases/tag/v0.1.0
