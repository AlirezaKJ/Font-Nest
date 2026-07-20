---
title: FontNest Todos
type: todo
status: active
created: 2026-06-05
updated: 2026-07-19
tags:
    - project
    - todo
    - fontnest
---

# FontNest — Master Todo

> [!info] Purpose
> This is the authoritative, living product and engineering roadmap for [[FontNest]]. It includes the Windows-first release path, the wider font-system feature surface, cross-platform work, and deliberately separated research ideas. It is an inventory, not a promise that every optional item belongs in the same release. Last competitive audit: 2026-07-19 against FontBase (free and Awesome), Typeface, RightFont, FontExpert, MainType, and FontGoggles.

> [!tip] Status and priority
>
> - `[x]` — shipped and verified in the repository.
> - `[ ]` — not shipped, partially shipped, or still needs release-grade verification.
> - **P0** — safety, data-integrity, or release blocker.
> - **P1** — core FontNest capability.
> - **P2** — valuable product expansion.
> - **P3** — optional polish or specialist workflow.
> - **R&D** — validate feasibility, licensing, and product fit before committing.

> [!success] Current usable slice — audited 2026-07-18
> FontNest has a Tauri 2 / Svelte 5 shell, native installed-font discovery, generated TypeScript DTOs, a Quiet Ledger catalogue and inspector, search and filters, read-only conflict hints, saved preview settings, light/dark/system themes, Google Fonts discovery and verified previews, and guarded Windows per-user Google Fonts installation with a SQLite ownership ledger. The bundled Google Fonts snapshot currently contains 1,928 families and 3,571 artifacts.

> [!warning] Current release blockers
> Installed-catalogue persistence, streamed batches, true virtualization, exact face previews, rename-stable database-backed opaque local IDs, crash-recoverable managed operations, and safe uninstall are not complete. Local preview files also need to cross the Rust validation boundary before their bytes are loaded into the WebView.

Related: [[FontNest]] · [[Font Explorer Doc]] · [Product contract](PRODUCT.md) · [Quiet Ledger design contract](DESIGN.md)

---

## 0. Product guardrails and scope

- [x] Adopt Tauri 2 + Svelte 5 / SvelteKit SPA + TypeScript + Rust 2024.
- [x] Keep Rust responsible for filesystem access, SQLite, font parsing, OS registration, provider trust, and destructive-operation invariants.
- [x] Keep the frontend responsible for presentation and interaction; it never receives authority through raw filesystem paths.
- [x] Adopt Quiet Ledger, dark and light parity, Geist for product UI, and Instrument Serif only for restrained brand, onboarding, and empty-state moments.
- [x] Make Windows the first fully supported platform.
- [x] Default to current-user installation; system-wide installation is outside v1.
- [x] Treat system, protected, package-owned, and unproven fonts as non-removable.
- [x] Keep online discovery opt-in and the installed catalogue local-first.
- [x] Keep performance figures as targets until measured on representative 1k / 5k / 10k-face corpora.
- [ ] **P0** Write the explicit v0.1 acceptance contract: read-only installed catalogue, exact preview, safe local/provider import, managed install/uninstall, conflicts, settings, diagnostics, and Windows installer.
- [ ] **P0** Define the ownership proof required before FontNest may modify, update, quarantine, restore, or remove a font.
- [ ] **P1** Define supported formats separately for inspect, preview, install, activate, export, and convert workflows.
- [ ] **P1** Define the legal boundary: FontNest reports metadata and warnings but does not guarantee that a licence permits embedding, redistribution, conversion, or team sharing.
- [x] Keep accounts, cloud sync, team libraries, marketplace behavior, font editing, and AI features outside the core release path; optional research remains separately gated below.

## 1. Verified foundation and immediate stabilization

### Shipped foundation

- [x] Scaffold the desktop app and configure SvelteKit static SPA mode with strict TypeScript.
- [x] Initialize Git and connect the GitHub remote.
- [x] Configure pnpm, Vite, Prettier, ESLint, rustfmt, Clippy, Rust 2024, and editor settings.
- [x] Exclude `src-tauri` from Vite file watching to avoid Windows rebuild contention.
- [x] Configure a restrictive CSP, frozen prototypes, minimal Tauri capabilities, and no shell permission.
- [x] Move native discovery off the command thread with Tauri async work plus `spawn_blocking`.
- [x] Generate TypeScript DTOs from Rust with `ts-rs`; generated bindings are not hand-edited.
- [x] Implement the Quiet Ledger application shell, custom title bar, navigation, catalogue, preview view, conflict view, Discover view, and Settings view.
- [x] Implement browser fixtures for frontend development without native commands.
- [x] Persist theme, density, default specimen, and saved previews.
- [x] Generate the current favicon and Tauri application icon assets from the supplied branding.
- [x] Add Google Fonts catalogue refresh tooling and a commit-pinned release manifest.

### Immediate cleanup and correctness

- [x] **P0** Route every user-selected local font through a Rust `validate_font_file` command before the WebView can load it; return an opaque preview handle rather than raw bytes or a path.
- [ ] **P0** Add a durable operation journal and startup recovery before enabling managed uninstall or update.
- [ ] **P0** Make uninstall authorization independent of editable SQLite paths: verify canonical containment, reparse-point status, managed filename, content hash, registry mapping, provider identity, and protection state.
- [ ] **P0** Bound, cancel, and page/stream the in-progress parser JSON export; do not build every codepoint and glyph into one unbounded pretty-JSON IPC string or hold catalogue state locked throughout parsing.
- [ ] **P1** Finish and verify the in-progress inspection backend: format/Clippy clean, derive the parser version, cap output, generate bindings, add frontend adapters/tests, and integrate the UI.
- [ ] **P0** Replace display-name-derived family IDs and scan-local path-derived face IDs with rename-stable, collision-resistant, database-backed IDs.
- [ ] **P1** Replace the one-response installed catalogue with cached SQLite summaries, pagination, and ordered Channel batches.
- [x] **P0** Restore a visible WCAG 2.2 focus indicator to editable specimen, search, and wrapped text controls.
- [x] **P0** Correct light-mode tertiary text contrast for small labels and verify every foreground/surface token pairing.
- [ ] **P1** Add keyboard and assistive-technology reordering for saved previews.
- [ ] **P1** Make glyph coverage truthful; never let system fallback masquerade as coverage in the selected font.
- [ ] **P1** Reconcile Tauri's `minWidth: 920` with the CSS breakpoints so Windows Snap and compact layouts are actually reachable.
- [ ] **P1** Make preview size controls accurate below 40px instead of clamping rendered output to 40px.
- [ ] **P2** Release every dynamically loaded local `FontFace` and revoke associated object URLs when previews change or close.
- [x] **P2** Apply the resolved theme before first paint to prevent a light-theme startup flash.
- [ ] **P2** Bundle the approved Geist and Instrument Serif WOFF2 assets locally with complete `@font-face` definitions and system fallbacks.
- [ ] **P2** Announce blocking errors assertively and keep safe technical details copyable.
- [ ] **P2** Remove slider/layout animations that lag direct manipulation; remove decorative style-row entrance choreography.
- [ ] **P3** Split the roughly 2,000-line route and Discover component into feature views, stores, domain adapters, and reusable controls.
- [ ] **P3** Consolidate duplicated CSS, shared filter controls, specimen utilities, `AppView` definitions, and safe font-stack code.
- [ ] **P3** Remove the scaffold greeting command/tests and either integrate or delete the unused `FontInspector` component.
- [ ] **P3** Remove unused CSS selectors and restore a clean `pnpm lint` baseline without overwriting unrelated work.
- [ ] **P3** Tokenize and document the intentionally dark title bar, including Windows close-button colors.
- [ ] **P3** Disable custom scrollbar styling in forced-colors mode and reconsider it on platforms where native behavior is clearer.
- [ ] **P1** Add a single source of truth and CI drift check for package, Cargo, Tauri, manifest, and release versions.

## 2. Repository, governance, and developer experience

- [x] **P1** Choose and add the project licence.
- [ ] **P1** Add `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, issue templates, and pull-request templates.
- [ ] **P1** Document the architecture, trust boundaries, generated-code policy, migration policy, and platform adapter contract.
- [x] **P1** Add an automated changelog/release-note path (repo `CHANGELOG.md` plus the in-app What's New view backed by the changelog parser and remote/bundled loader).
- [ ] **P1** Add Conventional Commit guidance.
- [ ] **P1** Pin and automatically update Rust, Node, pnpm, Cargo, and JavaScript dependencies with reviewed lockfile changes.
- [ ] **P1** Verify generated Rust/TypeScript bindings in CI and fail on drift.
- [ ] **P1** Add checks for formatted code, lint, type safety, tests, licences, vulnerabilities, generated resources, and docs links.
- [ ] **P2** Add reproducible local developer setup scripts and a fixture mode with small, medium, and pathological catalogues.
- [ ] **P1** Declare supported Node and pnpm versions through `engines`/`packageManager`, stop suppressing `svelte-kit sync` failures, and add one fail-fast local verification command.
- [ ] **P2** Record architecture decision records for IDs, SQLite migrations, preview transport, activation, uninstall recovery, and provider policy.
- [ ] **P2** Add a dependency and third-party font licence inventory.
- [ ] **P3** Add contributor fixtures and a documented process for legally adding test fonts.

## 3. Rust domain model and service boundaries

- [ ] **P1** Define invariant-rich domain entities with private fields and validated constructors:
    - [ ] `FontFile` — opaque ID, canonical identity, source, protection, format, size, timestamps, quick fingerprint, optional content hash.
    - [ ] `FontCollection` — TTC/OTC container identity, face count, and per-face indices.
    - [ ] `FontFace` — names, style coordinates, metadata state, collection index, axes, features, and coverage.
    - [ ] `FontFamily` — typographic/WWS grouping, aliases, localized names, and explicit user grouping overrides.
    - [ ] `ManagedInstallation` — immutable ownership proof, operation provenance, licence receipt, and current state.
    - [ ] `ActivationSession` — scope, consumers, selected faces, start/end policy, and recovery state.
    - [ ] `DuplicateGroup` — conflict kind, evidence, confidence, members, and resolution state.
    - [ ] `ProviderArtifact` — provider identity, source revision, digest, style/axes, licence, and cache state.
    - [ ] `ScanRun` and `ManagedOperation` — generation, progress, cancellation, journal state, warnings, and outcome.
- [ ] **P1** Add validated newtypes for file, collection, face, family, installation, provider, artifact, scan, operation, collection, and project IDs.
- [ ] **P1** Make opaque IDs stable across ordinary rescans and renames without exposing raw paths or display names.
- [ ] **P1** Separate native domain entities, persistence records, IPC DTOs, and view models.
- [ ] **P1** Add exhaustive enums for format, technology, source, install scope, protection reason, validation state, activation state, operation state, and conflict kind.
- [ ] **P1** Define repository, parser, scanner, platform, provider, preview, hashing, and diagnostics interfaces with deterministic test fakes.
- [ ] **P1** Keep Tauri command functions thin: validate DTOs, call an application service, map typed errors, and return bounded results.
- [ ] **P1** Add stable error codes with category, retryability, suggested recovery, safe message, internal cause, and correlation ID.
- [ ] **P1** Add progress, cancellation, timeout, and idempotency semantics to every long-running command.
- [ ] **P2** Model domain events for catalogue generation published, file changed, operation recovered, installation changed, and provider snapshot changed.
- [ ] **P2** Make invalid or incomplete states explicit rather than representing them with nullable strings and booleans.

## 4. Durable catalogue, SQLite, scanning, and watching

### Database and migrations

- [ ] **P0** Replace the managed-installation ledger's ad hoc table creation with numbered, forward-tested migrations before destructive management ships.
- [ ] **P1** Extend the same migration system to the complete catalogue and user metadata schema.
- [ ] **P1** Normalize tables for files, collections, faces, localized names, families, axes, named instances, features, coverage, licences, provider artifacts, managed installations, conflicts, scan roots, scan runs, tags, user collections, projects, and operation journal entries.
- [ ] **P1** Add foreign keys, unique constraints, indexes, busy timeout, WAL policy, and consistent connection initialization.
- [ ] **P1** Add FTS5 indexes for names, aliases, PostScript names, foundries, tags, features, licences, notes, and paths where disclosure is allowed.
- [ ] **P1** Stage reconciliation into a scan generation and atomically publish only a completed generation.
- [ ] **P1** Preserve per-file parse errors and partial success without failing the complete catalogue.
- [ ] **P1** Model stale, missing, offline-volume, externally removed, and tombstoned records.
- [ ] **P0** Add a single-instance/interprocess writer lock so two FontNest processes cannot mutate the catalogue or managed font state concurrently.
- [ ] **P0** Refuse unknown forward schema versions and define tested downgrade/rollback behavior.
- [ ] **P0** Fail closed into an explicit read-only recovery mode when ledger migration, integrity verification, operation journal, or managed-root initialization fails; disable install, update, uninstall, repair, and restore commands.
- [ ] **P0** Add managed-ledger integrity verification, rotating backup, recovery preview, and reconciled restore that never blindly treats restored paths as ownership proof.
- [ ] **P1** Add catalogue rebuild-index, salvage, WAL checkpoint, vacuum, cache reset, last-known-good generation, and corruption-recovery workflows.
- [ ] **P1** Preserve tags, collections, notes, projects, and managed ownership proof when rebuilding derived catalogue data.
- [ ] **P2** Expose schema version, database size, generation, health, and pending recovery work in diagnostics.

### Cached-first discovery and reconciliation

- [ ] **P1** Show cached family summaries first, then reconcile filesystem and platform state in the background.
- [ ] **P1** Record canonical path, platform file ID/inode where available, size, modified time, creation time, face count, and a quick fingerprint.
- [ ] **P1** Parse only new/changed files; remove or mark missing files without discarding user metadata.
- [ ] **P1** Defer BLAKE3, glyph coverage, script coverage, axes, features, and deep table inspection until needed or idle.
- [ ] **P1** Enumerate every face in TTC/OTC collections.
- [ ] **P1** Replace path-substring provenance with platform-native enumeration and protection metadata.
- [ ] **P1** Distinguish installed, active, disabled, missing, shadowed, protected, FontNest-managed, externally managed, preview-only, provider-available, and offline states.
- [ ] **P1** Handle Unicode normalization, case folding, long paths, non-UTF-8 paths where supported, hardlinks, symlinks, junctions/reparse points, network shares, removable media, inaccessible files, and locked files.
- [ ] **P1** Handle cloud-placeholder files (OneDrive Files On-Demand and similar): detect dehydrated placeholders, never trigger mass hydration during a scan, and label cloud-only fonts distinctly instead of treating them as missing.
- [ ] **P1** Add user-configurable watched folders, project folders, one-time scan roots, and exclusion rules.
- [ ] **P1** Debounce watcher storms and reconcile rename pairs as one change.
- [ ] **P1** Detect OS registration and font-cache changes in addition to directory changes.
- [ ] **P1** Add scan generation IDs, cancel, pause/resume, retry, partial-success warnings, and manual full rebuild.
- [ ] **P1** Prevent concurrent full scans and conflicting reconciliation with single-flight coordination.
- [ ] **P2** Add scan history and a calm “new, changed, removed since last scan” summary.
- [ ] **P2** Add a “why is this font here?” provenance trail.
- [ ] **P2** Let users temporarily disconnect slow/network roots without losing their catalogue metadata.

### Scalable IPC and scheduling

- [ ] **P1** Stream scan begin, batch, progress, warning, complete, and cancelled messages through a Tauri Channel.
- [ ] **P1** Query, filter, sort, facet, and page in Rust/SQLite instead of shipping every face to JavaScript.
- [ ] **P1** Return minimal family summaries and load face/file details on demand.
- [ ] **P1** Add backpressure and bounded queues between discovery, stat, parse, hash, persistence, and IPC stages.
- [ ] **P1** Use bounded I/O work and a separate bounded CPU pool for parsing and hashing.
- [ ] **P1** Prioritize visible-family metadata and explicit user requests over background enrichment.
- [ ] **P1** Track cancellation latency and avoid locks across awaits or unbounded `spawn_blocking` work.

## 5. Font formats, parsing, metadata, and diagnostics

### Format matrix

- [ ] **P1** Inspect and preview desktop SFNT fonts: TTF, OTF, TTC, and OTC.
- [ ] **P1** Inspect and preview web fonts: WOFF and WOFF2; keep them preview-only until an explicit conversion workflow is approved.
- [ ] **P1** Extend the local validation boundary to WOFF and WOFF2. Decode the container to SFNT bytes in Rust, under the same resource limits, before `validate_font_file` parses it, so decoded bytes still cross the trust boundary and preview through the `fontnest-preview` protocol. Keep them preview-only (no install/convert). Local import is SFNT-only today (`ttf`/`otf`/`ttc`/`otc`) because `ttf-parser` cannot read compressed containers. Crate options researched 2026-07-19: WOFF v1 is per-table zlib and is decodable with `flate2` or `allsorts`; for WOFF2 the choices are the pure-Rust `woff2` crate (`convert_woff2_to_ttf`, decode-only, but does not yet handle `hmtx` table transforms so it rejects some real fonts), `woofwoof` (wraps Google's reference WOFF2 C++ decoder with pure-Rust brotli, most correct but adds a C++ build step), or `allsorts` (pure Rust, parses WOFF and WOFF2 fully but pulls in a large shaping/subsetting dependency). Decode to SFNT then reuse the existing `local_fonts` validator; do not add a second parser path.
- [ ] **P1** Support TrueType outlines, CFF, CFF2, static fonts, variable fonts, and named instances.
- [ ] **P1** Detect COLR/CPAL v0/v1, SVG, sbix, CBDT/CBLC, bitmap strikes, and emoji/symbol fonts.
- [ ] **P1** Validate every face in a collection, not only face index zero.
- [ ] **P2** Inspect legacy Type 1, dfont, suitcase, or bitmap formats read-only only where a maintained parser and legal fixture corpus exist.
- [ ] **R&D** Evaluate safe conversion of unsupported legacy formats on a work-on-copy basis; never silently mutate originals.
- [ ] **P1** Never infer safety or capability only from an extension; validate the container signature and table directory.

### Names and descriptive metadata

- [ ] **P1** Preserve legacy, typographic/preferred, WWS, full, PostScript, unique, and localized family/subfamily names.
- [ ] **P1** Record version strings plus normalized comparable versions.
- [ ] **P1** Record manufacturer, designer, foundry/vendor ID, description, copyright, trademark, vendor/designer URLs, licence text, and licence URL.
- [ ] **P1** Read OS/2 embedding permissions and explain install, preview, print, editable embedding, restricted embedding, conversion, and redistribution implications without legal guarantees.
- [ ] **P1** Record weight, width/stretch, slant, italic angle, fixed-pitch state, PANOSE, IBM family class, Unicode ranges, and code pages.
- [ ] **P1** Record units per em, ascender, descender, line gap, cap height, x-height, underline, strikeout, caret metrics, advances, and bounding box.
- [ ] **P1** Record glyph count, cmap subtables, scripts, inferred languages, variation sequences, and missing/replacement-glyph behavior.
- [ ] **P1** Parse `fvar`, `avar`, `STAT`, named instances, registered/custom axes, and axis flags.
- [ ] **P1** Parse GSUB/GPOS scripts, languages, feature tags, kerning, ligatures, alternates, numeral features, small caps, mark positioning, and vertical layout.
- [ ] **P1** Parse GDEF, BASE, JSTF, VORG, legacy `kern`, vertical metrics, MATH constants/glyph construction, and `meta` records for specialist inspection.
- [ ] **P1** Record color palettes, color-font versions, bitmap strikes, hinting technology, `gasp`, table checksums, DSIG/signature status, and modification fingerprints.
- [ ] **P1** Finish the current Phase-1 Rust parser snapshot for core metrics, names, variation axes, table records, Unicode mappings, and glyph advances/bounds; persist or page the useful results instead of returning one unbounded document.
- [ ] **P2** Build a bounded expert OpenType viewer/export UI over the Rust parser snapshot.
- [ ] **P2** Track parser/enrichment version so derived metadata can be invalidated after upgrades.

### Validation and font-system diagnostics

- [ ] **P0** Enforce file, face-count, table-count, offset, decompression, memory, CPU-time, and output-size limits for untrusted fonts.
- [ ] **P0** Validate table bounds/overlaps, collection integrity, required names, control characters, checksums where applicable, and selected provider metadata.
- [ ] **P1** Classify corrupt, truncated, malformed, oversized, suspicious, unsupported, and partially readable fonts with actionable reasons.
- [ ] **P1** Add an explicit quarantine/review path; never register a font that only partially validated.
- [ ] **P1** Add font-system health checks for registered-but-missing files, unregistered files, stale registry entries, broken collections, corrupt fonts, and cache problems.
- [ ] **P1** Add safe repair recommendations and restore points; never silently edit OS state.
- [ ] **P2** Integrate optional FontBakery-style quality checks or a compatible local validator behind a clearly labelled expert workflow.
- [ ] **R&D** Evaluate parser isolation in a constrained helper process before registering arbitrary third-party fonts with the OS.

## 6. Identity, grouping, search, filters, and catalogue views

### Identity and family grouping

- [ ] **P1** Include file identity and collection face index in local face identity.
- [ ] **P1** Reconcile stable identity across rename, move, watcher events, duplicate paths, and external changes.
- [ ] **P1** Group with typographic family/subfamily first, then WWS and legacy fallbacks; do not group solely by the first display family name.
- [ ] **P1** Preserve localized names and aliases without creating duplicate families.
- [ ] **P2** Allow explicit user merge/split/alias overrides and make them reversible.
- [ ] **P2** Explain why faces were grouped and flag ambiguous naming metadata.

### Search and discovery inside the local catalogue

- [ ] **P1** Add fuzzy, token, prefix, quoted-phrase, negation, diacritic-insensitive, Unicode-normalized, and locale-aware search.
- [ ] **P1** Search family/subfamily, localized names, PostScript name, filename, foundry, designer, licence, version, path, tags, notes, features, and Unicode characters.
- [ ] **P1** Add structured search tokens for format, source, state, weight, width, style, variable, color, monospaced, script, language, feature, glyph, conflict, duplicate, and update status.
- [ ] **P1** Add filters for weight range, italic/oblique, width/stretch, static/variable, color/bitmap, collections, format, foundry, licence, version, location, scope, protected state, recent changes, corruption, exact duplicates, and semantic conflicts.
- [ ] **P1** Add facets with result counts and match highlighting.
- [ ] **P1** Add sorts for name, recent install/change/view, file size, glyph count, foundry, version, axes, conflict severity, and custom order.
- [ ] **P2** Add expert metric filters for x-height, cap height, proportion/width, density, and other measurable classification traits when normalized data is available.
- [ ] **P2** Add a consumer-grade visual classification browse (serif, sans-serif, slab, script/handwriting, monospaced, display, symbol) derived locally from PANOSE, OS/2, and post table data, with visible confidence and reversible manual correction.
- [ ] **P2** Add recent search history, saved searches, and smart collections backed by reusable predicates.
- [ ] **P2** Add “find fonts covering this pasted text, Unicode range, script, or language.”
- [ ] **P2** Add a coverage-aware fallback-stack builder.

### Catalogue presentation and selection

- [ ] **P1** Replace progressive 120-family windows with true variable-height virtualization and stable scroll anchoring.
- [ ] **P1** Load only visible plus overscan preview faces and keep a bounded byte-aware LRU.
- [ ] **P1** Preserve focus, selection, scroll, and expanded rows during incremental reconciliation.
- [ ] **P1** Add specimen, compact list, metadata table, and coverage-focused views.
- [ ] **P2** Add resizable/reorderable columns and persist visibility and widths.
- [ ] **P1** Add multi-selection, range selection, “select all filtered,” and safe bulk actions.
- [ ] **P1** Add keyboard-complete roving focus that works across virtualized content.
- [ ] **P2** Group by family, foundry, source, folder, collection, tag, script, or status.
- [ ] **P2** Label system, protected, user, preview-only, FontNest-managed, provider-managed, shadowed, and unavailable records explicitly.
- [ ] **P2** Add quick actions for preview, compare, favorite, collect, copy identifier, reveal, export metadata, activate, and managed uninstall.
- [ ] **P3** Add a calm library overview: family/face counts by format, source, foundry, static/variable, color capability, and disk usage, presented as archival summary rather than a dashboard.
- [ ] **P2** Add accessible result counts, `aria-busy`, and virtualized-list semantics without flooding screen readers.

## 7. Exact preview infrastructure and specimen workspace

### Trusted exact-face preview transport

- [ ] **P0** Serve local and provider bytes through a scoped internal protocol keyed only by opaque face/artifact IDs.
- [ ] **P0** Use synthetic preview family names so duplicate installed families cannot resolve to the wrong OS-selected face.
- [ ] **P0** Include collection face index and optional variation coordinates in preview resolution.
- [ ] **P0** Never accept a frontend path or return a raw filesystem path as preview authority.
- [ ] **P1** Add MIME, byte-size, cache, request-rate, origin, window-label, and cancellation controls to the protocol.
- [ ] **P1** Replace provider Base64 IPC/data URLs with binary/object-URL-backed transport.
- [ ] **P1** Add single-flight requests, visible/overscan priority, queued-work cancellation, and a bounded byte-aware memory/disk cache.
- [ ] **P1** Add cache quota, expiry, orphan cleanup, manual clear, and low-disk behavior.
- [ ] **P1** Detect fallback/tofu and explicitly state when the selected face did not render.
- [ ] **P2** Evaluate on-demand preview subsetting for very large CJK/color fonts only after measurement.

### Specimens and proofing

- [ ] **P1** Build a fuller "preview a font file" dialog. The shipped one shows a single specimen line, the container format, and a face list. It should let the user edit the specimen text inline, change size, tracking, line height, and alignment, preview every face in the file under its own style instead of only face zero (needs collection face index in preview resolution), drive variable axes and named instances, toggle OpenType features, inspect glyph coverage and metrics, switch between waterfall and paragraph layouts, copy the reproducible CSS declaration, and continue into installing the file. Keep the trust boundary unchanged: everything still renders from validated bytes served by opaque handle.
- [ ] **P1** Add built-in specimens for family name, pangram, paragraph, headline, UI copy, numerals/currency, code, punctuation, diacritics, and multilingual text.
- [ ] **P1** Add waterfall, paragraph, poster, type scale, UI component, code editor, print proof, and custom multi-block layouts.
- [ ] **P1** Let users add, duplicate, rename, reorder, save, import, and export specimen blocks/presets.
- [ ] **P1** Add numeric size plus px/pt/rem units, tracking, line height, measure, word spacing, alignment, case, direction, writing mode, and columns.
- [ ] **P1** Add restrained text/background color controls and print-safe modes.
- [ ] **P1** Select face, weight, width, slant, and style independently.
- [ ] **P1** Remember controls per family and offer an explicit reset to defaults.
- [ ] **P1** Add calm load/error/retry states that never silently substitute a fallback.
- [ ] **P2** Debounce preference writes instead of persisting on every specimen keystroke.
- [ ] **P2** Export specimen sessions as JSON and proof sheets as PNG/PDF/print.
- [ ] **P2** Add dynamic language-aware pangrams, bidirectional samples, vertical writing, and custom specimen libraries.
- [ ] **P2** Add classic kerning and spacing proof texts (kern-king-style pair strings and spacing samples) to the built-in specimen library.
- [ ] **P3** Preview specimen text over a user-supplied local image backdrop for poster and photography proofing; the image never leaves the device.
- [ ] **P3** Auto-refresh preview-only fonts when the underlying file changes on disk so type designers can iterate on work-in-progress fonts, FontGoggles-style, with every reload still crossing the Rust validation boundary.
- [ ] **P3** Add pixel/DPI rendering tests and optional WebView-versus-native rendering comparison.

## 8. Variable fonts, OpenType features, glyphs, and color fonts

### Variable-font controls

- [ ] **P1** Add sliders and numeric input for every supported axis with min/default/max and reset.
- [ ] **P1** Support named instances, `STAT` names, `avar` mappings, registered-axis labels, custom-axis tags, and hidden axes.
- [ ] **P1** Link or unlink registered weight/width/slant/italic/optical-size controls from generic preview controls.
- [ ] **P1** Save named coordinate presets and copy CSS `font-variation-settings`.
- [ ] **P2** Add restrained axis animation/proofing with pause, speed, range, export, and reduced-motion compliance.
- [ ] **P2** Add two-axis maps and controlled axis sweeps for exploring interactions between variable dimensions.
- [ ] **P2** Compare static faces, named instances, and arbitrary variable coordinates.

### OpenType feature and shaping controls

- [ ] **P1** Show only features supported by the selected face/script/language.
- [ ] **P1** Add controls for kerning, standard/discretionary/historical ligatures, small/petite caps, fractions, ordinals, superscripts/subscripts, numeral style/width, swashes, contextual alternates, character variants, and stylistic sets.
- [ ] **P1** Expose human-readable names while preserving four-character tags for experts.
- [ ] **P1** Select shaping script, language, direction, and vertical orientation where supported.
- [ ] **P1** Copy CSS `font-feature-settings`, higher-level CSS properties, and the complete reproducible preview declaration.
- [ ] **P2** Inspect ligature substitution, GSUB/GPOS lookup effects, kerning pairs, alternates, and mark positioning.
- [ ] **P2** Use a deterministic shaping engine such as rustybuzz/HarfBuzz for diagnostic glyph runs and compare its output with WebView/platform shaping where useful.

### Glyph and Unicode browser

- [ ] **P1** Parse the real cmap and display actual glyph and Unicode coverage.
- [ ] **P1** Build a virtualized glyph browser grouped by script, block, category, and plane.
- [ ] **P1** Search by character, scalar/code point, Unicode name, glyph name, glyph ID, block, script, and category.
- [ ] **P1** Filter assigned, unassigned, private-use, combining, color, alternate, and missing glyphs.
- [ ] **P1** Show Unicode name/sequence, glyph ID, advance, side bearings, bounds, components, anchors, and variation information.
- [ ] **P1** Copy character, Unicode notation, HTML entity, CSS escape, UTF-8/UTF-16 values, or glyph name.
- [ ] **P2** Export a selected glyph as SVG or PNG artwork and copy it to the clipboard, surfacing the embedding-permission and licence warning before export.
- [ ] **P1** Add “paste text to test coverage” with an exact missing-character report.
- [ ] **P1** Add complex-shaping specimens for Arabic, Indic scripts, combining marks, vertical text, bidirectional text, and variation selectors.
- [ ] **P2** Add confusing-character proof sets such as `I/l/1`, `O/0`, quotes, punctuation, currencies, and diacritics.
- [ ] **P2** Compare language/script/glyph coverage between fonts and export character maps.
- [ ] **P1** Use grid/roving-tabindex semantics and meaningful accessible glyph labels without thousands of tab stops.

### Color, emoji, metrics, and outlines

- [ ] **P1** Preview COLR/CPAL, SVG, CBDT/CBLC, sbix, bitmap strikes, palettes, and emoji sequences accurately.
- [ ] **P1** Add palette selection and show whether palettes are light/dark/usability variants.
- [ ] **P2** Add baseline, x-height, cap height, ascender, descender, line-box, side-bearing, advance, and bounds overlays.
- [ ] **P2** Add zoomable outline inspection with components, points, anchors, and hinting indicators without becoming a glyph editor.
- [ ] **P2** Export metrics, palette, feature, and coverage reports.

## 9. Duplicates, conflicts, versioning, and repair

- [ ] **P1** Distinguish same-file aliases, hardlinks, exact byte duplicates, canonical equivalents, duplicate PostScript names, family/style conflicts, version conflicts, registration conflicts, variable/static overlap, and benign multi-file families.
- [ ] **P1** Use BLAKE3 for exact local hashes; retain provider Git SHA values only as provider provenance.
- [ ] **P1** Include width, slant, axes, named instances, PostScript name, version, foundry, file identity, coverage, and collection index in semantic signatures.
- [ ] **P1** Detect OS shadowing/precedence where only one conflicting face is actually selected.
- [ ] **P1** Store conflict evidence and confidence rather than one `has_conflict` boolean.
- [ ] **P1** Show every member with full metadata, hash, path, source/scope, ownership, protection, validation, and current registration.
- [ ] **P1** Add exact-face specimen, version, metadata, outline, and coverage comparison before resolution.
- [ ] **P1** Recommend a preferred copy based on protection, ownership, version, integrity, format, coverage, licence, and source, with the reasoning visible.
- [ ] **P1** Never offer destructive resolution for protected, system, package-owned, unproven, or externally managed files.
- [ ] **P1** Add dry run, choose-which-to-keep, ignore/acknowledge, bulk resolve, notes, audit receipt, and reversible quarantine/restore.
- [ ] **P1** Recalculate affected groups incrementally after file or registration changes.
- [ ] **P2** Add outline/glyph-content divergence detection for same-name versions.
- [ ] **P2** Export conflict and font-system health reports.
- [ ] **R&D** Evaluate explainable visual similarity using local PANOSE, metrics, coverage, and outline features.

## 10. Local import, managed install, update, uninstall, and activation

### Import and preflight

- [ ] **P1** Add trusted file, multi-file, folder, watched-folder, and drag/drop import.
- [ ] **P1** Add staged ZIP/archive import with entry-count and expanded-size caps plus traversal, absolute-path, symlink, reparse-point, and archive-bomb defenses.
- [ ] **P1** Distinguish preview-only, installable, unsupported, corrupt, duplicate, and conflicting files before mutation.
- [ ] **P1** Support TTC/OTC face review and make collection installation semantics explicit.
- [ ] **P1** Show a preflight plan with faces, versions, licences, embedding rights, conflicts, destination, expected bytes, and protected boundaries.
- [ ] **P1** Validate selected metadata against parsed bytes instead of trusting filenames or frontend labels.
- [ ] **P1** Add per-file and batch progress, cancel, retry, partial-success reporting, and final audit summary.
- [ ] **P1** Offer atomic-family and best-effort-batch policies explicitly.

### Crash-safe managed operations

- [ ] **P0** Write a journal entry before filesystem, registry, platform, or database mutation.
- [ ] **P0** Model prepared, staged, registered, committed, rollback-needed, failed, recovered, and abandoned states.
- [ ] **P0** Recover interrupted install/uninstall/update/repair operations on startup.
- [ ] **P0** Surface rollback failures and create a repair task rather than discarding them.
- [ ] **P0** Add per-artifact locks and idempotency keys so concurrent operations cannot race.
- [ ] **P0** Use unique same-directory staging, durable rename where supported, stale-stage cleanup, and exact permitted-root containment.
- [ ] **P0** Record immutable source/artifact ID, original and installed hash, destination root, registry mapping, licence receipt, app/schema version, and operation ID.
- [ ] **P1** Reconcile ledger rows against filesystem and OS state at startup and after watcher events.
- [ ] **P1** Report copy, parse, registration, broadcast/cache-refresh, ledger commit, and rollback outcomes separately.

### Safe uninstall, restore, repair, and update

- [ ] **P0** Before uninstall, reparse and hash the file and verify canonical root, non-reparse status, managed filename, exact registry mapping, source identity, ownership, and protection.
- [ ] **P0** Back up or quarantine managed files so uninstall is recoverable; permanent deletion is never the first action.
- [ ] **P0** Verify a registry value still points to the exact managed file before removing it.
- [ ] **P1** Preserve shared licence records until the final related artifact is removed.
- [ ] **P1** Handle in-use/locked fonts, delayed cleanup, logoff/restart requirements, and interrupted restore.
- [ ] **P1** Add managed install inventory, uninstall review, undo window, restore, re-register, repair, reinstall, and verify actions.
- [ ] **P1** Detect updates; compare old/new metadata, coverage, axes, licence, and hashes; support update, update-all, pin, ignore, rollback, and downgrade.
- [ ] **P1** Model partial-family install state instead of one family-level installed boolean.
- [ ] **P2** Allow explicit adoption of an existing file only after reconstructing strong ownership/provenance proof; otherwise keep it read-only.
- [ ] **P3** If system-wide installs are ever added, isolate elevation behind a separately reviewed broker and never make it the default.

### Activation without destructive installation

- [ ] **P2** Design temporary app-private, session, persistent per-user, and project activation modes per platform.
- [ ] **P2** Add activate/deactivate controls with explicit app restart, logout, or cache-refresh implications.
- [ ] **P2** Add project activation profiles and optional launch/exit rules.
- [ ] **P3** Support exact document/project-manifest autoactivation by fingerprint and version, with ambiguity and substitution reporting instead of silent name matching.
- [ ] **P3** Add watched auto-activation folders and allowlists for supported creative applications.
- [ ] **P3** Add startup/tray behavior only if persistent activation sets require a background process.

## 11. Online providers, discovery, downloads, and licensing

### Current Google Fonts foundation

- [x] Bundle a versioned Google Fonts manifest with commit-pinned artifact URLs; runtime use requires no API key.
- [x] Restrict runtime URLs to approved HTTPS hosts and pinned repository revisions.
- [x] Reject redirects, cap response sizes, verify Git object hashes, and parse fonts before preview/install.
- [x] Activate newly installed verified artifacts in the current WebView session so Library and Preview render them without an app restart.
- [x] Validate opaque provider IDs and app origin for sensitive preview/install commands.
- [x] Preserve upstream licence data with FontNest-managed installations.
- [x] Require an explicit artifact review before current-user Windows installation.

### Provider hardening

- [ ] **P0** Test the actual bundled release manifest rather than only a small fixture manifest.
- [ ] **P0** Make refresh output deterministic and atomic; add timeout, retry, schema validation, collision detection, source-age, coverage, and skipped-family reports.
- [ ] **P0** Parse artifact styles and axes from font bytes rather than only filenames.
- [ ] **P1** Add stronger release-resource digest/signature validation while retaining Git SHA provenance.
- [ ] **P1** Add a `FontProvider` adapter contract with isolated catalogue, trust, licence, preview, download, and update rules.
- [ ] **P1** Share one hardened HTTP client with global concurrency, byte, timeout, retry/backoff, cancellation, and proxy policies.
- [ ] **P1** Single-flight identical artifact/licence downloads and prevent concurrent install races.
- [ ] **P1** Add download progress, cancel, retry, resumable download where supported, offline mode, metered/data-saver behavior, and cache status.
- [ ] **P1** Add disk cache quotas, expiry, manual clear, low-disk handling, corrupt-cache repair, and verified reuse.
- [ ] **P1** Show snapshot revision/age, source links, full licence text/URL, artifact size, version, styles, axes, scripts, and coverage.
- [ ] **P1** Distinguish available, same-name local, exact installed, partially installed, older/newer version, locally modified, removed upstream, and update-available states.
- [ ] **P1** Add static-versus-variable guidance and warn when selected artifacts overlap.
- [ ] **P1** Add signed catalogue delta updates so provider metadata can refresh independently of an app release.
- [ ] **P2** Add provider favorites/wishlist, compare-to-installed, release diff, update, update-all, pin, ignore, and rollback.
- [ ] **P2** Add multi-select batch install from Discover with one combined preflight review, shared progress, and a single outcome report.

### Additional sources

- [ ] **P2** Evaluate Fontshare only after confirming a stable official catalogue/download API and complete licence metadata.
- [ ] **P2** Evaluate Fontsource, Bunny Fonts, OFL collections, and foundry feeds as separate adapters with explicit provenance and licensing.
- [ ] **P3** Evaluate a Nerd Fonts / patched developer-font source, and detect already-patched fonts in the local catalogue through their private-use icon ranges.
- [ ] **P3** Support self-hosted/private read-only manifests with signature and schema validation.
- [ ] **P3** Support local manifest import for offline organizational libraries.
- [ ] **R&D** Consider Adobe Fonts or subscription sources only if official APIs and licences explicitly permit catalogue, preview, activation, and management; never scrape.
- [ ] **R&D** Design declarative signed provider extensions; do not load arbitrary native plugins into the trusted process.

### Licence management

- [ ] **P1** Preserve licence text, URL, provider, source revision, receipt, and artifact relationship.
- [ ] **P1** Show OS/2 embedding rights alongside human-readable licence metadata.
- [ ] **P2** Add optional user records for purchase proof, seat count, project, owner, expiry, notes, and foundry account link without storing credentials.
- [ ] **P2** Warn before export, conversion, subsetting, packaging, or team sharing when rights are missing, restricted, or unknown.
- [ ] **P2** Export licence inventories and project compliance reports with a clear “not legal advice” statement.

## 12. Favorites, collections, projects, history, and portability

- [ ] **P1** Make favorites a first-class filter and navigation destination.
- [ ] **P1** Add manual collections, nested folders, tags, ratings, notes, aliases, and restrained color labels.
- [ ] **P1** Add smart collections driven by saved search/filter rules.
- [ ] **P1** Support drag, keyboard, context-menu, and bulk assignment to collections.
- [ ] **P1** Add recently viewed, imported, installed, updated, activated, and used histories with clear retention controls.
- [ ] **P1** Add hidden/excluded fonts and intentional-conflict ignore rules.
- [ ] **P1** Add project font sets with roles, required versions/styles/axes, notes, licence records, and activation state.
- [ ] **P2** Add a quick shortlist/scratch collection for active comparison sessions.
- [ ] **P2** Add custom ordering with pointer, keyboard, and numeric position controls.
- [ ] **P2** Add collection/project import, export, backup, restore, merge, and schema migration.
- [ ] **P2** Add machine-to-machine inventory comparison without copying licensed font binaries.
- [ ] **P2** Add managed-library backup/restore with hashes, metadata, licences, and revalidation of ownership proof.
- [ ] **P3** Add importers for metadata from Typeface, FontBase, Suitcase, or legacy FontExplorer exports when documented formats exist.
- [ ] **P3** Add deliberate drag-out of a font file copy from the app to Explorer or a design tool; never expose raw managed paths implicitly, and warn when embedding or redistribution rights are restricted.
- [ ] **P3** Add a work-on-copy library consolidation tool that copies loose local fonts into an organized folder tree by family or foundry; never move or delete originals without explicit consent.
- [ ] **P3** Add portable mode for settings/catalogue metadata; keep OS registration semantics platform-specific.
- [ ] **R&D** Add optional encrypted sync for tags, collections, notes, settings, and project manifests; never sync font binaries by default.

## 13. Compare, pairing, reports, export, and developer workflows

### Compare and pairing

- [ ] **P1** Compare two or more installed, preview-only, or provider fonts with synchronized controls.
- [ ] **P1** Add side-by-side, stacked, overlay, A/B toggle, blink, difference, waterfall, metrics, and blind-comparison modes.
- [ ] **P1** Synchronize or unlink text, size, face, axes, features, width, direction, and color per slot.
- [ ] **P1** Compare coverage, metrics, OpenType features, file metadata, versions, palettes, and variable axes.
- [ ] **P2** Normalize comparison by point size, x-height, cap height, line height, or visual bounds and make the normalization explicit.
- [ ] **P2** Save pairs with roles such as display, body, UI, code, fallback, or multilingual companion.
- [ ] **P2** Add explainable similar-installed alternatives based on classification, metrics, coverage, and local analysis.
- [ ] **P2** Add a missing-font replacement shortlist without modifying project files automatically.
- [ ] **P2** Export comparison proof sheets and decision reports.

### Reports and export

- [ ] **P1** Export catalogue, collection, project, conflict, health, glyph-coverage, licence, and managed-operation reports as CSV/JSON/Markdown/HTML/PDF.
- [ ] **P1** Export printable specimen/contact/proof sheets with selected faces, settings, metadata, and licence footer.
- [ ] **P1** Copy family name, PostScript name, Unicode value, file hash, metadata, and reproducible preview CSS.
- [ ] **P2** Generate `@font-face`, CSS stacks, CSS variables, design tokens, Tailwind configuration, and platform font declarations.
- [ ] **P2** Generate CSS `font-palette`, `font-optical-sizing`, `font-synthesis`, `size-adjust`, `ascent-override`, `descent-override`, and `line-gap-override` declarations where supported.
- [ ] **P2** Build coverage-aware fallback stacks and optional `unicode-range` declarations.
- [ ] **P2** Create project font manifests containing IDs, hashes, versions, roles, axes, features, and licence references.
- [ ] **P2** Audit CSS, HTML, design exports, and document manifests for referenced, missing, or mismatched fonts without changing them automatically.
- [ ] **P2** Package project metadata and permitted font files only after explicit redistribution-right checks and review.
- [ ] **R&D** Add work-on-copy WOFF2 conversion, text/script subsetting, variable static-instance export, and optimization with explicit derivative/licence warnings.
- [ ] **R&D** Add a Rust-powered CLI for scan, search, details, validate, doctor, report, project audit, and safe managed operations.
- [ ] **R&D** Document importer/exporter/provider APIs only after domain boundaries and security policies stabilize.

## 14. Desktop UX, navigation, settings, and Quiet Ledger polish

### Onboarding and navigation

- [ ] **P1** Add first-run onboarding for local scanning, provider network use, system protection, FontNest-managed ownership, and recovery.
- [ ] **P1** Add first-scan phase/count/elapsed/progress/cancel/retry with a cached-catalogue path on later launches.
- [ ] **P1** Restore last view, selected family, expanded row, scroll, filters, and preview session.
- [ ] **P1** Add a command palette for navigation, family lookup, filters, preview, collection, diagnostics, and settings actions.
- [ ] **P1** Add native File/Edit/View/Window/Help menus, standard accelerators, About, release notes, Help, diagnostics, and report-issue entry points.
- [ ] **P1** Add family/file context menus using familiar desktop actions.
- [ ] **P1** Add view-change focus management, focus restoration after dialogs, and back/forward navigation history.
- [ ] **P1** Persist window size, position, maximized state, and monitor; recover if a display disappears.
- [ ] **P1** Verify Snap Layouts, double-click maximize, Alt+Space, keyboard window controls, high-contrast captions, and multi-monitor DPI with the custom title bar.
- [ ] **P2** Add configurable keyboard shortcuts with collision detection and visible menu/tooltip hints.
- [ ] **P3** Evaluate detachable preview/compare windows while reusing existing Tauri windows.
- [ ] **P3** Add launch-at-login, minimize-to-tray, and close behavior only where activation/background features justify them.
- [ ] **P3** Add an optional global shortcut that opens a compact FontNest quick-search window for font lookup while working in another application.

### Settings and storage controls

- [ ] **P1** Add startup scan, background watch, cached-first, manual-only, and low-power behavior.
- [ ] **P1** Manage included/excluded scan roots and explain protected platform locations.
- [ ] **P1** Add parsing/hash concurrency and resource-use presets with safe defaults.
- [ ] **P1** Add preview defaults for layout, text, language, size, face, features, axes, direction, and background.
- [ ] **P1** Add install destination, confirmation strictness, backup, update, activation, and recovery preferences.
- [ ] **P1** Add provider/network, offline, proxy, bandwidth, manifest-update, and cache settings.
- [ ] **P2** Add a user-visible network activity screen showing provider/updater endpoints, bytes, cache use, last access, and the control that enabled each request.
- [ ] **P1** Show database/cache/log usage with clear, independently safe clear/rebuild actions.
- [ ] **P1** Move durable settings to a versioned Rust-owned store or database and add corruption recovery.
- [ ] **P2** Add UI scale, density, enhanced contrast, reduced-motion override, and UI-font fallback/reset controls.
- [ ] **P2** Add locale, date/number formatting, RTL, specimen language, and collation preferences.
- [ ] **P2** Add update channel, automatic check/download/install policy, and signed-update status.
- [ ] **P2** Add privacy, diagnostics/crash-report consent, data location, retention, and export controls.
- [ ] **P2** Add settings search, reset by section, reset all, and import/export.

### States and restrained polish

- [ ] **P1** Design loading, cached/stale, partial, offline, cancelled, permission, corrupt-font, migration, disk-full, network, registry, cache, update, and recovery states.
- [ ] **P1** Give every destructive or system-changing flow a plan, confirmation, progress, outcome, recovery, and audit receipt.
- [ ] **P1** Add undo where the underlying operation is genuinely recoverable.
- [ ] **P2** Add subtle reduced-motion-safe crossfades when verified preview bytes replace skeletons.
- [ ] **P2** Add clear copy, glyph, export, favorite, collection, and undo feedback.
- [ ] **P2** Add useful “new since last scan” and remembered per-family specimen state.
- [ ] **P2** Audit all new components in both themes and every state against `docs/DESIGN.md`.
- [ ] **P2** Keep Discover archival and tool-like: no marketplace cards, popularity theater, glass effects, or promotional motion.
- [ ] **P3** Limit tracked uppercase microcopy/eyebrows and let specimen content carry visual character.

## 15. Accessibility, localization, input, and adaptive layout

- [ ] **P0** Meet WCAG 2.2 AA in dark and light themes for text, focus, controls, errors, selection, and disabled states.
- [ ] **P0** Add automated contrast checks for token/surface pairings plus manual verification for composited states.
- [ ] **P1** Support forced colors, Windows High Contrast, `prefers-contrast`, and reduced motion without losing essential boundaries or status.
- [ ] **P1** Test 200% zoom, Windows text scaling, common DPI scales, long localized strings, and window snapping.
- [ ] **P1** Provide visible focus for contenteditable fields, popovers, virtualized rows, glyph grids, reorder controls, dialogs, and custom title-bar controls.
- [ ] **P1** Add keyboard alternatives for drag/drop, ordering, range selection, bulk actions, glyph navigation, and compare slots.
- [ ] **P1** Restore focus after dialogs/actions and move focus predictably after navigation or destructive outcomes.
- [ ] **P1** Use native table semantics where practical or complete ARIA grid/table semantics with row groups and counts.
- [ ] **P1** Add accessible scan/download/install progress with phase, count, byte progress, cancellation, and final outcome.
- [ ] **P1** Announce blocking errors assertively; avoid noisy hover/live-region announcements.
- [ ] **P1** Use Unicode names in glyph labels and keep virtualized glyph content out of the sequential tab order.
- [ ] **P1** Replace view-specific skip links with “Skip to main content.”
- [ ] **P1** Add localization infrastructure, plural rules, RTL mirroring, locale-aware search/collation, and bidirectional specimen handling.
- [ ] **P1** Add pseudo-locale, expansion, mixed-direction, and missing-translation tests before shipping additional locales.
- [ ] **P1** Preserve UI legibility when a preview font is corrupt, missing, or intentionally extreme.
- [ ] **P2** Verify Narrator, NVDA, JAWS, VoiceOver, and Orca smoke paths on supported platforms.
- [ ] **P2** Verify mouse, keyboard, touch, pen, coarse pointer, IME, horizontal scroll, and screen-reader combinations.

## 16. Windows platform integration

- [ ] **P0** Use Windows Known Folder APIs for managed roots instead of trusting mutable environment variables.
- [ ] **P0** Model HKCU/HKLM registration, DirectWrite enumeration, user/system directories, package/Store fonts, substitutions, linking, and protection.
- [ ] **P0** Verify Add/RemoveFontResourceEx flags and registry operations are symmetrical and idempotent.
- [ ] **P0** Handle long paths, UNC paths, reparse points, hardlinks, case-insensitive containment, locked files, font cache behavior, and `WM_FONTCHANGE`.
- [ ] **P1** Test install/uninstall/repair/rollback in disposable Windows VMs across supported versions.
- [ ] **P1** Build and test x64 and ARM64 packages where the Tauri/WebView2/dependency matrix supports them.
- [ ] **P1** Test multi-session/RDP, suspend/resume during mutation, antivirus/indexer interference, disk-full transitions, and reboot-pending recovery.
- [ ] **P1** Explain when applications, Explorer, the user session, or Windows must restart.
- [ ] **P1** Add trusted “Reveal in Explorer,” “Open in Windows Font Viewer,” and “Open Fonts settings” actions resolved from opaque IDs.
- [ ] **P1** Verify WebView2 runtime requirements, offline installation, custom title bar, Snap, high contrast, per-monitor DPI, and installer repair.
- [ ] **P2** Add file associations and “Inspect with FontNest” Explorer integration with explicit opt-in.
- [ ] **P2** Detect Microsoft 365 cloud fonts delivered into the user profile (the Office CloudFonts cache) alongside Store-packaged fonts, label their provenance truthfully, and treat them as externally managed.
- [ ] **P2** Add deep links for trusted app navigation without accepting raw paths or commands.
- [ ] **P3** Evaluate MSIX, MSI, and NSIS tradeoffs for updates, signing reputation, repair, and uninstall cleanup.

## 17. macOS, Linux, and cross-platform expansion

### Shared platform contract

- [ ] **P1** Finalize an independently testable `FontPlatform` contract for enumerate, classify, protect, preview, install, uninstall, activate, reveal, refresh, and recover.
- [ ] **P1** Feature-gate platform dependencies and keep unsupported operations explicit.
- [ ] **P1** Verify the preview protocol on WebView2, WKWebView, and WebKitGTK.
- [ ] **P1** Add platform-specific messages for application restart, cache refresh, logout, and unsupported scopes.

### macOS

- [ ] **P2** Enumerate with CoreText/CTFontManager across user, local, system, and network domains.
- [ ] **P2** Model protected system fonts, Font Book disabled state, duplicate resolution, and coexistence with other managers.
- [ ] **P2** Implement current-user install/uninstall/activation with `~/Library/Fonts` and platform APIs.
- [ ] **P2** Verify WKWebView protocol behavior, sandbox entitlements, hardened runtime, Apple Silicon/x64, code signing, and notarization.
- [ ] **P2** Adapt title bar, menus, shortcuts, dialogs, and reveal actions to macOS conventions.

### Linux

- [ ] **P2** Enumerate through fontconfig and XDG user/system locations with package ownership/protection metadata.
- [ ] **P2** Implement current-user install/uninstall/activation and controlled font-cache refresh without arbitrary shell execution.
- [ ] **P2** Handle Flatpak/Snap portals, AppImage/deb/rpm packaging, Wayland/X11, and WebKitGTK constraints.
- [ ] **P2** Adapt title bar, menus, shortcuts, dialogs, and reveal actions to desktop-environment conventions.

## 18. Performance, memory, reliability, and observability

### Measurement corpus and budgets

- [ ] **P1** Build legal 1k / 5k / 10k-face corpora containing collections, variable fonts, CJK fonts, color fonts, duplicate-heavy sets, corrupt fonts, slow/network roots, and unusual paths.
- [ ] **P1** Measure first run, cold start, cached first paint, incremental reconciliation, watcher storms, parse/enrichment, exact hashing, search/facet/page, preview load, glyph view, conflicts, provider search, installation recovery, and shutdown.
- [ ] **P1** Measure scroll FPS, long tasks, IPC payloads, cache hit rate, memory return, file handles, download bytes, disk growth, cancellation latency, and battery/CPU impact.
- [ ] **P1** Validate or revise targets: usable shell under 1s, cached library under 250ms after frontend readiness, search/filter under 50ms, 60 FPS scrolling, and stable post-scroll memory.
- [ ] **P1** Set explicit loaded-face, preview-byte, cache, database, bundle-size, worker, and file-handle budgets.

### Optimization and resilience

- [ ] **P1** Batch database writes, use prepared statements/indexes/FTS, and tune WAL/checkpoints from measurements.
- [ ] **P1** Stream hashes rather than reading entire large files where supported.
- [ ] **P1** Compact or index the bundled provider manifest if startup or package measurements justify it.
- [ ] **P1** Virtualize long family, face, glyph, conflict, comparison, and provider-artifact views.
- [ ] **P1** Release dynamic `FontFace` resources and verify memory returns after full-corpus scrolling.
- [ ] **P1** Add recovery for corrupted preferences, catalogue DB, preview cache, provider cache, and interrupted managed operations.
- [ ] **P2** Add Criterion benchmarks, flamegraphs, WebView performance traces, and Windows ETW hooks where useful.
- [ ] **P2** Add long-running soak tests for watchers, previews, provider browsing, suspend/resume, sleep, and drive disconnect/reconnect.

### Production diagnostics

- [ ] **P1** Add structured spans for scan, parse, hash, query, preview, download, validation, install, rollback, uninstall, migration, and watcher reconciliation.
- [ ] **P1** Enable redacted rotating production logs with operation/correlation IDs and configurable levels.
- [ ] **P1** Preserve internal error chains in logs while returning safe, actionable UI errors.
- [ ] **P1** Add a FontNest Doctor check for DB, schema, manifest, caches, managed files, registry entries, watchers, platform adapter, and pending recovery.
- [ ] **P1** Add a safe mode that can disable providers/watchers, bypass derived caches, load last-known-good catalogue/provider snapshots, and open recovery/diagnostics without mutating font state.
- [ ] **P1** Persist bounded panic/crash breadcrumbs and pending-operation state early enough to guide the next safe-mode launch.
- [ ] **P1** Add user-approved redacted diagnostics export.
- [ ] **P2** Show cache hits, phase timings, parse failures, DB size, catalogue generation, watcher state, and pending operations in an expert diagnostics screen.

## 19. Security, privacy, and supply chain

- [ ] **P0** Maintain a threat model covering IPC, local preview, internal protocol, parsing, watched roots, providers, cache, install, uninstall, activation, updater, diagnostics, and plugins.
- [ ] **P0** Treat UI inputs, provider manifests, cached bytes, SQLite rows, registry entries, and filesystem metadata as untrusted at destructive boundaries.
- [ ] **P0** Validate origin, top-level frame, window label, IDs, enum values, lengths, rates, operation state, and authorization for sensitive commands.
- [ ] **P0** Add canonical containment checks resistant to path traversal, case differences, symlinks, junctions/reparse points, hardlinks, and TOCTOU changes.
- [ ] **P0** Never load unvalidated user/provider font bytes into the WebView or register them with the OS.
- [ ] **P0** Add malicious-font fixtures, parser fuzzing, property tests, and resource exhaustion guards.
- [ ] **P1** Add regression tests for CSP, frozen prototypes, capabilities, shell disabled, production DevTools, navigation restrictions, protocol scope, and remote IPC denial.
- [ ] **P1** Keep file selection scoped through trusted dialogs or opaque handles.
- [ ] **P1** Sign installers and updater artifacts; keep signing keys outside frontend/build variables and ordinary CI logs.
- [ ] **P1** Generate an SBOM and run `cargo audit`, `cargo deny`, npm audits, licence policy, secret scanning, and lockfile review.
- [ ] **P1** Add reproducible-build and artifact checksum verification where practical.
- [ ] **P1** Keep catalogue, queries, specimen text, paths, tags, projects, and font binaries local by default.
- [ ] **P1** Keep telemetry and crash reporting transparent, opt-in, redacted, and disabled by default.
- [ ] **P1** Redact usernames, paths, specimen text, queries, licences, and provider details from exported diagnostics by default.
- [ ] **P1** Add data/cache/log retention controls plus a complete local-data export and clear workflow.
- [ ] **P2** Add database backup/integrity and managed-root ACL checks.
- [ ] **R&D** Require signed, declarative, least-privilege extensions if a plugin system is introduced.

## 20. Testing, QA, CI, and release gates

### Rust and native tests

- [ ] **P0** Test operation-journal transitions, failure injection at every filesystem/registry/DB boundary, rollback failure, restart recovery, and idempotent retry.
- [ ] **P0** Prove a tampered ledger cannot unregister or delete an arbitrary file.
- [ ] **P1** Test domain invariants, stable ID uniqueness, grouping, localized names, collection indices, fingerprinting, parsing, coverage, conflict classification, and typed errors.
- [ ] **P1** Test migrations from every released schema plus backup, restore, integrity, and corrupt-DB recovery.
- [ ] **P1** Add malformed, truncated, oversized, overlapping-table, control-character, non-BMP, non-UTF-8-path, variable, color, symbol, CFF/CFF2, TTC/OTC, WOFF2, and huge-font fixtures.
- [ ] **P1** Add property tests for containment, managed filenames, IDs, migrations, grouping, and manifest validation.
- [ ] **P1** Add fuzz targets for SFNT/collection parsing, manifests, IPC DTOs, paths, and state-machine transitions.
- [ ] **P1** Mock provider HTTP for redirects, timeouts, size mismatch, chunk overflow, cancellation, corrupt cache, hash mismatch, retries, and concurrency.
- [ ] **P1** Test stale registry entries, externally removed/modified managed files, shared licences, watcher storms, rename, partial permissions, and drive disconnect.
- [ ] **P1** Test the actual bundled Google Fonts manifest in CI.

### Frontend, accessibility, and E2E

- [x] Pure TypeScript tests cover command adapters, preview queuing, saved-preview ordering utilities, and Tauri configuration.
- [ ] **P1** Add Svelte component tests for library, filters, preview, glyphs, conflicts, Discover, Settings, dialogs, and every state branch.
- [ ] **P1** Add keyboard tests for virtualized rows, popovers, dialogs, saved-preview reorder, glyph grids, compare slots, and command palette.
- [ ] **P1** Add Axe-based checks and manual assistive-technology matrices.
- [ ] **P1** Add visual regressions for both themes/densities, key window sizes, zoom, forced colors, loading, empty, error, offline, progress, and destructive states.
- [ ] **P1** Add packaged Tauri E2E tests with fake adapters plus real Windows scan/install/uninstall/reveal/recovery smoke tests.
- [ ] **P1** Add offline, timeout, corrupt preview, hash mismatch, partial install, cancel, retry, cache corruption, and provider-snapshot failure workflows.
- [ ] **P1** Add responsive/Snap, multi-monitor, DPI, text-scale, RTL, localization, IME, sleep/resume, and input-method QA.

### CI and release gates

- [ ] **P1** Add GitHub Actions for Windows lint, type-check, tests, rustfmt, Clippy, generated bindings, manifest validation, security checks, and package build.
- [ ] **P1** Add macOS/Linux adapter builds and tests as those platforms become supported.
- [ ] **P1** Fail CI on generated DTO drift, version drift, nondeterministic manifest output, missing licence data, broken docs links, or uncommitted generated resources.
- [ ] **P1** Add coverage reporting with meaningful thresholds for security and domain state machines.
- [ ] **P1** Make accessibility, visual, performance, memory-return, mode-parity, migration, recovery, and installer-upgrade tests release gates when their features ship.
- [ ] **P2** Add Windows VM and platform packaging matrices for install, update, repair, downgrade policy, uninstall, and cleanup.

## 21. Packaging, updates, release, docs, and brand assets

- [x] Configure app name, identifier, version, description, restrictive CSP, and application icons.
- [x] Adopt the supplied logo mark and Quiet Ledger brand direction.
- [ ] **P1** Finalize the FontNest name across GitHub, package ecosystems, domains, and social accounts.
- [x] **P1** Add the project licence before public binary distribution.
- [ ] **P1** Complete monochrome/full-color logo variants, wordmark/lockup, and PNG icon sizes.
- [ ] **P1** Bundle a signed Windows installer with clean install, repair, update, downgrade policy, uninstall, and residue tests.
- [ ] **P1** Choose and test offline, embedded, or download-bootstrap WebView2 installer behavior, including missing-runtime recovery.
- [ ] **P1** Add code signing and build reputation planning for Windows.
- [x] **P1** Add the signed Tauri updater with signature verification, tested end to end from the GitHub release feed.
- [ ] **P1** Add updater stable/beta channels, rollback policy, and recovery from interrupted updates.
- [x] **P1** Add update-check, download progress, and release notes to the update flow.
- [ ] **P1** Add updater defer/remind later, skipped versions, relaunch control, staged rollout, and last-known-good recovery.
- [ ] **P1** Disable production DevTools/remote debugging and verify packaged CSP/capabilities.
- [ ] **P1** Publish checksums, SBOM, third-party notices, release notes, and supported Windows/WebView2 requirements.
- [ ] **P1** Expand README with product story, features, safety model, screenshots, installation, development, architecture, privacy, and troubleshooting.
- [x] **P1** Add the CHANGELOG.
- [ ] **P1** Add contributor docs, security reporting, privacy statement, support policy, and user guide.
- [ ] **P1** Document backup/restore, database migration, managed uninstall, provider trust, licence warnings, and diagnostics export.
- [ ] **P2** Add screenshots for both themes, store artwork, social/OG image, and a short demo video.
- [ ] **P2** Add signed macOS `.dmg`/notarization and Linux AppImage/deb/rpm packages only after platform acceptance gates pass.
- [ ] **P2** Publish through Windows package channels: a winget manifest first, then evaluate Microsoft Store, Chocolatey, and Scoop once signing and update policy are settled.
- [ ] **P2** Add release automation for tagged, signed, reproducible artifacts with staged rollout and rollback.
- [ ] **P2** Add optional VirusTotal links/checksums as supporting evidence, not as the sole security claim.
- [ ] **P3** Build a restrained landing page and portfolio case study after the download/support path is ready.

## 22. Optional power-user and research expansions

- [ ] **P3** Add document/project missing-font audits for CSS, HTML, design exports, PDFs, and supported document manifests.
- [ ] **P3** Add optional creative-app integrations for project activation after stable, documented APIs are confirmed.
- [ ] **P3** Add local font-usage history with clear privacy/retention controls and no document-content collection.
- [ ] **P3** Add local-only similarity and pairing recommendations with visible reasons and manual controls.
- [ ] **P3** Add accessibility-oriented recommendations for legibility, language coverage, confusing glyphs, and UI roles without claiming universal readability.
- [ ] **P3** Add multi-window comparison and presentation/full-screen proof modes.
- [ ] **P3** Add an optional glyph anatomy overlay that names strokes, bowls, apertures, terminals, and joins on an enlarged specimen for evaluation and teaching.
- [ ] **P3** Add LAN/private-library inventory browsing without copying binaries by default.
- [ ] **R&D** Identify a font from an image using an opt-in local model; keep uploaded imagery and font files off remote services by default.
- [ ] **R&D** Add opt-in AI pairing or specimen suggestions only when recommendations are explainable and no font bytes/private text leave the device without consent.
- [ ] **R&D** Add encrypted team metadata sync, shared collections, licence seats, approvals, and private provider feeds; keep this outside the personal v1 architecture.
- [ ] **R&D** Add a signed extension system for providers, importers, exporters, and project auditors with explicit capabilities and isolation.
- [ ] **R&D** Add a headless service/API only after authentication, local permission, rate-limit, and destructive-operation boundaries are designed.
- [ ] **R&D** Add work-on-copy validation/repair, conversion, subsetting, and variable-instance generation as companion tools; FontNest remains a manager/inspector, not a full glyph editor.
- [ ] **R&D** Evaluate Incremental Font Transfer only for future web-export/tooling workflows; it is not a desktop catalogue requirement.
- [ ] **R&D** If a mobile product is ever useful, make it a separate specimen/collection companion rather than weakening the desktop font-manager architecture.

## 23. Open product and architecture decisions

- [x] Choose the repository/application licence.
- [ ] Freeze the opaque local file/face/family ID strategy and rename reconciliation rules.
- [ ] Define exact, semantic, version, registration, and intentional-conflict semantics.
- [ ] Define managed uninstall backup duration, undo behavior, locked-font recovery, and permanent-delete policy.
- [ ] Define temporary/session/project activation semantics on Windows and their v1 status.
- [ ] Choose SQLite migration, backup, and downgrade behavior before the first public schema ships.
- [ ] Set true virtualization overscan and loaded-face LRU limits from benchmarks.
- [ ] Define supported Windows versions, WebView2 policy, and installer format.
- [ ] Define the macOS/Linux release order and platform-specific minimum acceptance gates.
- [ ] Define provider admission, signing, snapshot update, and removal policy.
- [ ] Define the telemetry/crash-reporting policy; default remains off.
- [ ] Decide whether the custom title bar and current minimum window size survive Windows accessibility/Snap testing.
- [ ] Define cloud/team boundaries and confirm no font binary sync by default.
- [ ] Define export/conversion/subsetting licence checks and user acknowledgements.
- [ ] Decide whether a CLI belongs in the core workspace or a later companion package.

## 24. Delivery sequence and definitions of done

> [!note] Version mapping
> Milestones 0–5 are pre-release slices. The first Windows `v0.1` candidate is Milestone 6 after the earlier slices pass their combined safety, accessibility, recovery, and packaging gates.

### Milestone 0 — foundation and first usable slice

- [x] Tauri/Svelte shell, native discovery, generated DTOs, Quiet Ledger library/preview/conflicts/settings, Google Fonts discovery/preview, and guarded Windows per-user provider install.

### Milestone 1 — stabilize the current slice

- [ ] Complete P0 validation-boundary, install-journal, ID, exact-preview, accessibility, contrast, responsive-window, lint, and manifest-test work.
- [ ] Split oversized frontend modules and establish component/E2E/accessibility test foundations.

### Milestone 2 — durable catalogue at scale

- [ ] Ship versioned SQLite catalogue, cached-first reconciliation, native provenance, Channel batches, Rust-side query/page, true virtualization, watcher updates, and measured 10k-face behavior.

### Milestone 3 — deep type inspection

- [ ] Ship exact face previews, rich metadata, real glyph coverage, multilingual specimens, variable axes, OpenType controls, color-font support, and metrics/outline diagnostics.

### Milestone 4 — safe local font management

- [ ] Ship local import/preflight, crash-safe managed install/update/uninstall/restore, exact duplicates, semantic conflicts, font-system Doctor, and reversible repair.

### Milestone 5 — organization and decision tools

- [ ] Ship favorites, collections, smart searches, projects, compare/pairing, reports, proof sheets, and licence-aware exports.

### Milestone 6 — Windows v1 release

- [ ] Pass security, accessibility, performance, recovery, installer, upgrade, signing, updater, docs, and support gates on the supported Windows matrix.

### Milestone 7 — macOS and Linux

- [ ] Implement platform adapters and pass equivalent protection, preview, install/uninstall, accessibility, packaging, and recovery gates per OS.

### Milestone 8 — optional ecosystem

- [ ] Evaluate activation integrations, additional providers, CLI, conversion/subsetting, sync, teams, and local intelligence as separate product decisions.

### Definition of done for every shipped feature

- [ ] Domain invariants and authorization live in Rust, not only the UI.
- [ ] IPC uses bounded typed DTOs; generated bindings are refreshed and verified.
- [ ] Long work has progress, cancel, timeout, partial-success, retry, and recovery behavior where applicable.
- [ ] Loading, empty, error, offline, permission, disk-full, and interrupted states are designed.
- [ ] Keyboard, screen reader, contrast, zoom/DPI, forced-colors, reduced-motion, and both-theme behavior are verified.
- [ ] Security/privacy abuse cases and destructive-operation boundaries have tests.
- [ ] Representative large-corpus performance and resource use are measured against declared budgets.
- [ ] Database/settings migrations, backup/restore, and downgrade implications are documented.
- [ ] User documentation, diagnostics, audit history, and recovery steps ship with the feature.
- [ ] Platform-specific behavior is tested on a packaged build, not only in browser fixture mode.

## 25. Research anchors

These references define capabilities and terminology; they do not override FontNest's product and safety guardrails.

- [Microsoft OpenType specification overview](https://learn.microsoft.com/en-us/typography/opentype/spec/overview)
- [OpenType variable fonts: `fvar`](https://learn.microsoft.com/en-us/typography/opentype/spec/fvar)
- [OpenType Layout feature tags](https://learn.microsoft.com/en-us/typography/opentype/spec/featuretags)
- [OpenType OS/2 metrics and embedding permissions](https://learn.microsoft.com/en-us/typography/opentype/spec/os2)
- [OpenType cmap](https://learn.microsoft.com/en-us/typography/opentype/otspec190/cmap)
- [OpenType COLR and CPAL](https://learn.microsoft.com/en-us/typography/opentype/spec/colr)
- [OpenType SVG table](https://learn.microsoft.com/en-us/typography/opentype/spec/svg)
- [W3C WOFF2 recommendation](https://www.w3.org/TR/WOFF2/)
- [W3C CSS Fonts Level 4](https://www.w3.org/TR/css-fonts-4/)
- [W3C CSS Font Loading](https://www.w3.org/TR/css-font-loading/)
- [Windows DirectWrite custom font sets](https://learn.microsoft.com/en-us/windows/win32/directwrite/custom-font-sets-win10)
- [Apple Font Book guide](https://support.apple.com/en-asia/guide/font-book/fntbk1000/mac)
- [Typeface feature documentation](https://typefaceapp.com/help/articles)
- [RightFont documentation](https://rightfontapp.com/docs)
- [FontBase feature overview](https://fontba.se/)
- [FontGoggles shaping and variation previewer](https://fontgoggles.org/)
- [MainType font manager](https://www.high-logic.com/font-manager/maintype)
- [FontExpert font manager](https://www.proximasoftware.com/fontexpert/)
