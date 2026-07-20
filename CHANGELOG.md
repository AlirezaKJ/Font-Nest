# Changelog

All notable changes to FontNest are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

FontNest reads this file to show its release notes in the app. It pulls the copy from the `main`
branch so it can show notes for versions newer than the one you have installed, and falls back to
a bundled copy when you are offline.

## [Unreleased]

### Added

- Right-click now opens a FontNest menu instead of the browser one, and it changes depending on
  what you clicked. On a family: open it, save it to previews, review a conflict, copy the name or
  a ready-to-paste CSS font stack, or show the file in your file manager. On a single style: copy
  the PostScript name, the style, the file name, or a full CSS declaration. On a character: copy
  the character, its codepoint, an HTML entity, or a CSS escape, or drop it into your preview text.
  Right-clicking selected text offers to copy it, use it as your preview text, or search your
  library for it, and text fields get proper Cut, Copy, Paste, Select all, and Clear. Right-click
  anywhere else for rescan, theme, sidebar, and the main views. The menu works from the keyboard
  too, with the Menu key or Shift+F10, then arrows and typing to pick an item. The highlight
  slides between items the same way the sidebar's does.
- A Focus outlines setting under Settings > Accessibility. Turn it on to draw a visible outline
  around whatever has keyboard focus (the editable preview text, search, buttons, and controls),
  which helps if you navigate with the keyboard. It is off by default.

### Changed

- Previewing a font file from disk now opens the system file picker and shows the font in its own
  preview, instead of quietly adding a temporary entry to your library. For now you can preview
  TrueType and OpenType files (.ttf, .otf, .ttc, .otc).

### Fixed

- Show in file manager no longer dumps you in a random folder for fonts that came with Windows.
  Windows presents its own font folder as a control panel rather than a directory, and the files
  inside it cannot be picked out individually, so Explorer was quietly giving up and opening
  whatever folder it happened to default to. FontNest now opens the Windows Fonts folder itself
  and tells you why it could not highlight the file. Fonts you installed yourself are still
  selected directly, and font files whose names contain spaces or commas now work too.
- The window now appears as soon as it has finished painting instead of hanging back for a few
  seconds. It was missing a permission it needed to show itself, so it was falling through to a
  delayed safety net.
- Darkened a few of the faintest labels in light mode. The smallest secondary text was too low
  contrast to read comfortably against the lighter panels; it now meets the accessibility contrast
  bar.

### Security

- A font file you preview from disk is now checked by FontNest's Rust core before any of its bytes
  reach the preview. Every face is parsed and validated first, and the bytes are served to the
  preview through an internal handle rather than a file path, so a malformed or hostile font cannot
  slip straight into the app.

## [0.1.2] - 2026-07-19

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
- Moved the "Catalogue ready" status chip to the left of the title bar, next to the app name. It
  no longer crowds the search field on the busy right side.
- Dropped the "Workspace" and "Saved previews" headings in the sidebar and put a thin divider line
  between the two groups instead. Frees up vertical space so more of your saved previews fit.
- Added a Collapse sidebar / Expand sidebar option to the menu in the top bar, so you can toggle the
  sidebar without reaching for its edge.

### Fixed

- FontNest no longer flashes a blank white screen or the wrong theme when it launches. The window
  now stays hidden until its first frame is painted in your saved light or dark theme, so it opens
  straight into the right colors.

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

[Unreleased]: https://github.com/AlirezaKJ/Font-Nest/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/AlirezaKJ/Font-Nest/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/AlirezaKJ/Font-Nest/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/AlirezaKJ/Font-Nest/releases/tag/v0.1.0
