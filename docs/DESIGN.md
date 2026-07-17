---
name: FontNest
description: A calm, precise desktop workspace for inspecting and managing local fonts.
colors:
    dark-bg: '#111210'
    dark-surface: '#171815'
    dark-panel: '#131411'
    dark-raised: '#1D1E1A'
    dark-text: '#F1F0EA'
    dark-muted: '#AAA9A0'
    dark-subtle: '#8C8B82'
    dark-border: '#34342F'
    dark-accent: '#D8D2BE'
    dark-accent-ink: '#20201C'
    dark-selected: '#2C2D27'
    dark-focus: '#C6B98F'
    dark-success: '#71B184'
    dark-warning: '#E1AF59'
    dark-danger: '#EA7970'
    light-bg: '#F4F3EF'
    light-surface: '#FBFAF7'
    light-panel: '#EFEEE8'
    light-raised: '#FFFFFF'
    light-text: '#292A27'
    light-muted: '#6E6D66'
    light-subtle: '#817F77'
    light-border: '#D9D8D1'
    light-accent: '#33352F'
    light-accent-ink: '#FFFFFF'
    light-selected: '#DEDDD5'
    light-focus: '#766746'
    light-success: '#52705A'
    light-warning: '#91631F'
    light-danger: '#9D3B33'
typography:
    display:
        fontFamily: 'Instrument Serif, Georgia, serif'
        fontSize: '32px'
        fontWeight: 400
        lineHeight: 1.05
        letterSpacing: '-0.025em'
    headline:
        fontFamily: 'Geist, Inter, system-ui, sans-serif'
        fontSize: '24px'
        fontWeight: 600
        lineHeight: 1.2
        letterSpacing: '-0.025em'
    title:
        fontFamily: 'Geist, Inter, system-ui, sans-serif'
        fontSize: '16px'
        fontWeight: 600
        lineHeight: 1.3
        letterSpacing: '-0.015em'
    body:
        fontFamily: 'Geist, Inter, system-ui, sans-serif'
        fontSize: '14px'
        fontWeight: 400
        lineHeight: 1.5
        letterSpacing: 'normal'
    label:
        fontFamily: 'Geist, Inter, system-ui, sans-serif'
        fontSize: '12px'
        fontWeight: 600
        lineHeight: 1.35
        letterSpacing: '0.01em'
rounded:
    xs: '4px'
    sm: '6px'
    md: '8px'
    lg: '12px'
    xl: '16px'
    shell: '22px'
spacing:
    xs: '4px'
    sm: '8px'
    md: '12px'
    lg: '16px'
    xl: '20px'
    xxl: '24px'
    xxxl: '32px'
components:
    button-primary-dark:
        backgroundColor: '{colors.dark-accent}'
        textColor: '{colors.dark-accent-ink}'
        typography: '{typography.label}'
        rounded: '{rounded.md}'
        padding: '{spacing.sm} {spacing.md}'
        height: '36px'
    button-primary-light:
        backgroundColor: '{colors.light-accent}'
        textColor: '{colors.light-accent-ink}'
        typography: '{typography.label}'
        rounded: '{rounded.md}'
        padding: '{spacing.sm} {spacing.md}'
        height: '36px'
    button-secondary-dark:
        backgroundColor: '{colors.dark-raised}'
        textColor: '{colors.dark-text}'
        typography: '{typography.label}'
        rounded: '{rounded.md}'
        padding: '{spacing.sm} {spacing.md}'
        height: '36px'
    button-danger-dark:
        backgroundColor: '{colors.dark-danger}'
        textColor: '{colors.dark-bg}'
        typography: '{typography.label}'
        rounded: '{rounded.md}'
        padding: '{spacing.sm} {spacing.md}'
        height: '36px'
    input-dark:
        backgroundColor: '{colors.dark-raised}'
        textColor: '{colors.dark-text}'
        typography: '{typography.body}'
        rounded: '{rounded.md}'
        padding: '{spacing.sm} {spacing.md}'
        height: '36px'
    nav-selected-dark:
        backgroundColor: '{colors.dark-selected}'
        textColor: '{colors.dark-text}'
        typography: '{typography.label}'
        rounded: '{rounded.md}'
        padding: '{spacing.sm} {spacing.md}'
        height: '36px'
    chip-dark:
        backgroundColor: '{colors.dark-panel}'
        textColor: '{colors.dark-muted}'
        typography: '{typography.label}'
        rounded: '{rounded.shell}'
        padding: '{spacing.xs} {spacing.sm}'
        height: '24px'
---

# Design System: FontNest

## Overview

**Creative North Star: "The Working Type Archive"**

FontNest is a quiet, exact workspace built for long sessions among thousands of font faces. The frame borrows the discipline of an archival index: clear labels, stable rows, explicit provenance, and calm layers that let the typography under inspection remain the expressive material. It is professional and discerning without becoming sterile, nostalgic, or artificially cozy.

The visual system is restrained by design. Dark Quiet Ledger is the selected reference mode; Light Quiet Ledger is its equal functional counterpart. Both modes preserve the same hierarchy, density, semantic meanings, and component geometry. Color identifies action, selection, and state. It never decorates empty space.

### Logo system

- The master mark is [`../assets/branding/logo.svg`](../assets/branding/logo.svg). Always use the SVG in application UI, documentation, and scalable exports.
- [`../assets/branding/logo.png`](../assets/branding/logo.png) is the transparent raster fallback for contexts that cannot render SVG. Never enlarge it beyond its native dimensions.
- The mark is a monochrome open-book/F monogram. Do not redraw, rotate, crop, outline, add effects, or change its internal geometry.
- On Light Quiet Ledger, render the source mark unchanged. On Dark Quiet Ledger, render it as a pure light monochrome mark using the `logoFilter: invert(1)` treatment or an equivalent explicit light asset.
- Keep clear space of at least 15% of the rendered mark width on every side. The minimum UI size is 20px; use 32–40px for app identity and at least 64px for standalone brand presentation.
- When a wordmark is needed, place “FontNest” to the right in Geist SemiBold. The gap is one quarter of the mark width. Never set the product name in Instrument Serif.

**Key Characteristics:**

- Restrained, two-mode neutral palette
- Dense but breathable desktop information hierarchy
- Familiar controls with explicit state boundaries
- Tonal layering before shadow
- Geist for dependable UI; Instrument Serif only for controlled brand expression
- The selected user font always owns the preview surface

**The Quiet Frame Rule.** If the interface competes with a font specimen, the frame is too expressive.

**The Mode Parity Rule.** Dark and light modes must expose identical information, component states, and safety cues. A feature is not complete until both modes are verified.

## Colors

Quiet Ledger combines achromatic, slightly organic neutrals with a low-chroma accent. Dark mode feels like a focused evening work surface; light mode feels like a clean archival table under soft daylight.

### Primary

- **Dark Archive Accent** (`dark-accent`): Primary action, selected control emphasis, active slider fill, and deliberate focus moments in dark mode. It is never a decorative fill.
- **Light Archive Accent** (`light-accent`): Primary action and active-state equivalent in light mode.

### Secondary

- **Dark Focus Brass** (`dark-focus`): Keyboard focus rings and high-attention non-destructive emphasis in dark mode.
- **Light Focus Umber** (`light-focus`): Keyboard focus rings and high-attention non-destructive emphasis in light mode.

### Tertiary

- **Success, Warning, and Danger** (`dark-success`, `dark-warning`, `dark-danger`, and light equivalents): Reserved for system state. Every use must include an icon, label, or explanatory copy so color is never the only signal.

### Neutral

- **Workspace Ground** (`dark-bg`, `light-bg`): The application perimeter and deepest canvas.
- **Catalogue Surface** (`dark-surface`, `light-surface`): Main library and reading regions.
- **Navigation Panel** (`dark-panel`, `light-panel`): Sidebars, toolbars, and stable chrome.
- **Raised Control** (`dark-raised`, `light-raised`): Inputs, menus, inspectors, and controls that need separation from their parent surface.
- **Primary Ink** (`dark-text`, `light-text`): Headings, body copy, important values, and active labels.
- **Secondary Ink** (`dark-muted`, `light-muted`): Supporting text and metadata. This is the lowest allowed color for readable body-sized text.
- **Tertiary Ink** (`dark-subtle`, `light-subtle`): Compact labels and non-essential metadata only; never long-form copy or placeholders.
- **Structural Line** (`dark-border`, `light-border`): Dividers, field outlines, table rules, and panel boundaries.
- **Selected Wash** (`dark-selected`, `light-selected`): Selected navigation and list rows; always paired with an additional state cue where ambiguity is possible.

**The Ten Percent Rule.** Accent color occupies no more than 10% of a working screen. Its rarity creates hierarchy.

**The Semantic Signal Rule.** Success, warning, and danger colors communicate state only. Never use them for categories, branding, or visual variety.

**The Readability Floor.** Body and placeholder text must maintain at least 4.5:1 contrast. Large text must maintain at least 3:1. Visible focus indicators must maintain at least 3:1 against adjacent colors.

## Typography

**Display Font:** Instrument Serif (Georgia fallback)

**Body Font:** Geist (Inter, system-ui fallback)

**Character:** Geist makes dense system information feel neutral, modern, and trustworthy. Instrument Serif adds a precise editorial counterpoint, but it is deliberately rare so it never competes with the fonts being inspected.

Bundle the required WOFF2 files with the desktop application. FontNest must not depend on a network connection to render its own interface. Disable synthetic bold and italic where a real face is unavailable.

### Hierarchy

- **Display** (Instrument Serif Regular, 32px, 1.05): Brand-led empty states, onboarding moments, and documentation callouts only.
- **Headline** (Geist SemiBold, 24px, 1.2): Screen titles and major inspector headings.
- **Title** (Geist SemiBold, 16px, 1.3): Panel titles, dialogs, grouped settings, and font-family names.
- **Body** (Geist Regular, 14px, 1.5): Explanations, settings copy, and standard application text. Prose is capped at 70 characters per line.
- **Label** (Geist SemiBold, 12px, 1.35): Buttons, tabs, table headers, metadata labels, and compact navigation.
- **Micro** (Geist Medium, 11px, 1.35): Timestamps and low-priority metadata only. Never use micro text for actions, destructive warnings, or essential status.

**The Specimen Ownership Rule.** Real font previews always render in the selected user font. Instrument Serif is a brand voice and fallback specimen, never a substitute for inspected font data.

**The UI Sans Rule.** All controls, labels, metadata, navigation, and destructive warnings use Geist. Display serif in controls is prohibited.

**The Stable Scale Rule.** Application typography uses fixed sizes rather than viewport-fluid sizing. Desktop density must remain predictable across panels and window widths.

## Elevation

FontNest is flat by default. Depth comes from the ordered sequence of workspace ground, catalogue surface, navigation panel, raised controls, structural lines, and selected washes. Shadows are reserved for surfaces that physically move above the application plane: menus, popovers, command palettes, dragged rows, and blocking dialogs.

### Shadow Vocabulary

- **Floating Control** (`0 8px 24px rgba(0, 0, 0, 0.22)` in dark mode; `0 8px 24px rgba(41, 42, 39, 0.14)` in light mode): Menus, popovers, and command palettes.
- **Blocking Dialog** (`0 20px 60px rgba(0, 0, 0, 0.32)` in dark mode; `0 20px 60px rgba(41, 42, 39, 0.18)` in light mode): Destructive confirmation and system-permission dialogs only.
- **Dragged Item** (`0 10px 30px rgba(0, 0, 0, 0.25)`): Temporary feedback while a file or font row is actively dragged.

Use gently curved geometry: 4px for glyph cells, 6px for compact controls, 8px for standard controls, 12px for panels, 16px for large containers, and 22px only for the outer app shell. A child surface must never have a larger radius than its parent.

Motion communicates state in 150–220ms using `cubic-bezier(0.16, 1, 0.3, 1)`. Opacity and transform are preferred. Layout animation, bounce, elastic easing, and decorative page choreography are prohibited. `prefers-reduced-motion` reduces transitions to an immediate state change or a short crossfade.

**The Flat-by-Default Rule.** If a static panel needs a shadow to be understood, its tonal layer or border hierarchy is wrong.

**The One Plane Rule.** Never stack card inside card. Use dividers, spacing, or one tonal change to express internal structure.

## Components

Components are compact, familiar, and quietly tactile. Every interactive control must define default, hover, focus-visible, active, disabled, loading, and error states before it is complete.

### Buttons

- **Shape:** Gently curved rectangle (8px radius), normally 36px high; 32px is allowed in dense toolbars and 40px for high-attention dialogs.
- **Primary:** Active mode accent with accent ink, 12px horizontal padding, Geist SemiBold 12px. Use one primary action per local decision area.
- **Secondary:** Raised-control background, structural-line border, and primary ink. Hover shifts one tonal step; it does not gain a brand fill.
- **Ghost:** Transparent at rest with primary ink. Hover uses the selected wash.
- **Danger:** Danger fill is reserved for the final destructive confirmation. Earlier steps use a bordered secondary button with danger text.
- **Hover / Active:** Change tone over 150ms; active state may translate downward by 1px. Never scale buttons.
- **Focus:** A 2px focus-color outline with a 2px offset. The ring is never removed.
- **Disabled:** Preserve label legibility while reducing emphasis. Disabled controls must communicate why through nearby copy or a tooltip.
- **Loading:** Keep button width stable, replace the leading icon with a compact progress indicator, and retain the action label.

### Chips

- **Style:** Fully rounded compact control (22px radius), 24px high, with a structural border or panel fill.
- **State:** Unselected filter chips use muted ink; selected chips use the selected wash plus primary ink and a visible check or removal icon.
- **Purpose:** Filters, font technologies, scripts, sources, and non-destructive status. Chips are not miniature primary buttons.

### Cards / Containers

- **Corner Style:** 12px for standard panels and 16px for large empty states.
- **Background:** Use one semantic surface role. Nested containers stay transparent unless they are independently interactive.
- **Shadow Strategy:** Flat at rest; follow the elevation vocabulary only when a surface actually floats.
- **Border:** One-pixel structural line. Dashed borders are limited to drop zones and unscanned empty states.
- **Internal Padding:** 16px for compact panels, 20px for inspectors, and 24px for focused empty states.

### Inputs / Fields

- **Style:** 36px high, raised-control background, one-pixel structural border, 8px radius, and 12px horizontal padding.
- **Focus:** Border changes to focus color with an external 2px focus ring. Do not use glow.
- **Placeholder:** Uses muted ink and meets the same 4.5:1 contrast requirement as body text.
- **Error:** Danger-colored border plus an icon and concise inline message. Never signal error by border color alone.
- **Disabled:** Maintain readable content; reduce surface contrast and disable pointer feedback.

### Navigation

- Primary navigation uses a stable left rail on wide windows and a compact top or disclosed rail on narrow windows.
- Default items use muted ink. Hover uses the selected wash at reduced emphasis. Active items use the full selected wash, primary ink, and semibold label.
- Icons are 16px monochrome outline symbols with consistent 1.5–1.75px strokes. Filled and outline icon families must not be mixed.
- Navigation labels never use Instrument Serif, uppercase display styling, or decorative tracking.

### Font Family Row

- The row is the central inspection primitive: family identity, provenance, preview, style count, and risk state remain aligned across the list.
- Standard rows are 64–72px high. Compact mode may reduce them to 48px but must preserve keyboard targets of at least 32px.
- Selection uses the selected wash and a persistent non-color cue such as a check, leading marker, or inspector connection.
- System, protected, app-managed, duplicate, and conflicting files use explicit text labels. Never infer safety from icon or color alone.
- Preview text renders in the inspected font while all metadata remains in Geist.

### Destructive Confirmation

- Uninstall and conflict-resolution flows name the exact font family, faces, file paths, source, and protection status.
- System fonts and app-managed fonts must be visually and verbally distinct before the destructive action becomes available.
- The final action uses the danger treatment. Focus never lands on the destructive action by default.
- A successful operation reports what changed and whether recovery is possible.

## Do's and Don'ts

### Do:

- **Do** let inspected fonts provide the expressive typography while the application frame remains composed and restrained.
- **Do** use the SVG logo as the canonical asset and preserve its exact path geometry, proportions, and clear space.
- **Do** test every screen in both Quiet Ledger modes; mode parity is a release requirement.
- **Do** keep one primary action per local decision area and reserve accent color for action, selection, and state.
- **Do** keep controls fully keyboard-operable with visible focus, logical order, and minimum WCAG 2.2 AA contrast.
- **Do** label system fonts, app-managed files, duplicates, conflicts, and destructive boundaries explicitly.
- **Do** use virtualization and stable row geometry for large catalogues so the interface stays calm at scale.
- **Do** bundle Geist and Instrument Serif locally with stable system fallbacks.
- **Do** use familiar desktop affordances for search, filtering, tabs, menus, tables, and confirmation.

### Don't:

- **Don't** resemble a generic SaaS analytics dashboard.
- **Don't** resemble a novelty font marketplace.
- **Don't** become an overdecorated creative tool whose branding competes with the user's typefaces.
- **Don't** use gratuitous cards, nested cards, ornamental typography in controls, excessive motion, decorative glass effects, or unfamiliar interaction patterns where standard desktop affordances are clearer.
- **Don't** make system fonts appear casually removable or hide the difference between system and app-managed files.
- **Don't** use visual charm to soften a destructive action.
- **Don't** use Instrument Serif for controls, navigation, metadata, or warnings.
- **Don't** use gradients, gradient text, glassmorphism, neon glows, or colored side-stripe borders.
- **Don't** communicate success, warning, danger, selection, protection, or conflict through color alone.
- **Don't** animate layout properties, scale buttons on hover, or introduce bounce and elastic easing.
- **Don't** invent a new radius, shadow, color, font size, or component state when an existing token already covers the need.
