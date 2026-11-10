---
name: Ledger Precision
colors:
  surface: '#f9f9ff'
  surface-dim: '#d3daef'
  surface-bright: '#f9f9ff'
  surface-container-lowest: '#ffffff'
  surface-container-low: '#f1f3ff'
  surface-container: '#e9edff'
  surface-container-high: '#e1e8fd'
  surface-container-highest: '#dce2f7'
  on-surface: '#141b2b'
  on-surface-variant: '#484456'
  inverse-surface: '#293040'
  inverse-on-surface: '#edf0ff'
  outline: '#797488'
  outline-variant: '#c9c3d9'
  surface-tint: '#5f36ef'
  primary: '#4000c6'
  on-primary: '#ffffff'
  primary-container: '#582be8'
  on-primary-container: '#d1c7ff'
  inverse-primary: '#cabeff'
  secondary: '#006c4a'
  on-secondary: '#ffffff'
  secondary-container: '#82f5c1'
  on-secondary-container: '#00714e'
  tertiary: '#850024'
  on-tertiary: '#ffffff'
  tertiary-container: '#b10032'
  on-tertiary-container: '#ffbdbf'
  error: '#ba1a1a'
  on-error: '#ffffff'
  error-container: '#ffdad6'
  on-error-container: '#93000a'
  primary-fixed: '#e6deff'
  primary-fixed-dim: '#cabeff'
  on-primary-fixed: '#1b0062'
  on-primary-fixed-variant: '#4600d7'
  secondary-fixed: '#85f8c4'
  secondary-fixed-dim: '#68dba9'
  on-secondary-fixed: '#002114'
  on-secondary-fixed-variant: '#005137'
  tertiary-fixed: '#ffdada'
  tertiary-fixed-dim: '#ffb3b6'
  on-tertiary-fixed: '#40000c'
  on-tertiary-fixed-variant: '#920028'
  background: '#f9f9ff'
  on-background: '#141b2b'
  surface-variant: '#dce2f7'
typography:
  display:
    fontFamily: Plus Jakarta Sans
    fontSize: 32px
    fontWeight: '700'
    lineHeight: 40px
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Plus Jakarta Sans
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.015em
  headline-md:
    fontFamily: Plus Jakarta Sans
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 24px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Plus Jakarta Sans
    fontSize: 15px
    fontWeight: '600'
    lineHeight: 20px
    letterSpacing: 0em
  body-lg:
    fontFamily: Plus Jakarta Sans
    fontSize: 15px
    fontWeight: '400'
    lineHeight: 22px
    letterSpacing: 0em
  body-md:
    fontFamily: Plus Jakarta Sans
    fontSize: 13px
    fontWeight: '400'
    lineHeight: 18px
    letterSpacing: 0em
  body-sm:
    fontFamily: Plus Jakarta Sans
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
    letterSpacing: 0.01em
  label-mono-lg:
    fontFamily: JetBrains Mono
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 24px
    letterSpacing: -0.02em
  label-mono-md:
    fontFamily: JetBrains Mono
    fontSize: 13px
    fontWeight: '500'
    lineHeight: 18px
    letterSpacing: -0.01em
  label-mono-sm:
    fontFamily: JetBrains Mono
    fontSize: 11px
    fontWeight: '500'
    lineHeight: 14px
    letterSpacing: 0.02em
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  gutter: 1rem
  margin: 1.5rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1.25rem
  space-xl: 2rem
---

## Brand & Style

This design system blends contemporary minimalist software design with the disciplined architecture of classical accounting ledgers. Created for a high-performance desktop financial cockpit, the interface projects quiet authority, mathematical exactness, and calm analytical control.

The visual style merges structured minimalism with subtle tactile grid accents:
- **Atmospheric Anchor**: An underlying subtle 16px accounting graph paper grid provides structural rhythm without noise.
- **Demeanor**: Unhurried, deliberate, and high-density without clutter. Information is grouped with structural borders and hairline rules rather than excessive card boxing.
- **Ledger Heritage**: Tabular alignment, column-first data scanning, and strict monospaced numeric readouts create an interface that behaves like an instrument of financial truth.

## Colors

The palette establishes strict semantic roles to ensure rapid interpretation of financial flow:

- **Primary (`#582be8` / `#6e44ff`)**: Royal Violet serves as the navigational spine, active planning indicators, focal callouts, and structural interactive states. Paired with muted lavender (`#ede9fe`) for subtle selections and badge backgrounds.
- **Secondary / Growth (`#059669` / `#10b981`)**: Deep refined emerald reserved exclusively for incoming transactions, positive variance, liquidity gains, and validated reconciliations. Accompanied by mint tint (`#ecfdf5`).
- **Tertiary / Outflow (`#e11d48` / `#f43f5e`)**: Desaturated crimson utilized for expenses, liabilities, budget exceedance, and critical alerts. Accompanied by soft rose (`#fff1f2`).
- **Caution / Pending (`#d97706` / `#f59e0b`)**: Amber ochre for unposted drafts, pending clearing windows, and warning thresholds. Supported by warm parchment tint (`#fef3c7`).
- **Surfaces & Grid Lines**:
  - **Light Mode**: Background `#fbfbf9`, card surface `#ffffff`, secondary surface `#f8f8f6`, grid line `#111827` at `0.05` opacity, border zinc `#e2e8f0`.
  - **Dark Mode**: Background `#0f1115`, card surface `#18191e`, secondary surface `#14151a`, grid line `#ffffff` at `0.04` opacity, border zinc `#27272a`.

## Typography

Typography establishes an intentional dichotomy between narrative controls and quantitative ledgers:

- **Executive & Interface UI (Plus Jakarta Sans)**: Drives navigation, page titles, form labels, tooltips, and informational descriptions with crisp contemporary proportions and humanist legibility.
- **Financial Ledger & Metrics (JetBrains Mono)**: Strictly assigned to monetary values (e.g., `KES 1,482,500.00`), account numbers, transaction hashes, dates, time markers, and tabular data columns.
- **Rules of Rendition**:
  - All numerical expressions must enforce `font-variant-numeric: tabular-nums lining-nums`.
  - Negative values employ true mathematical minus glyphs (`−`) rather than generic hyphens.
  - Currency ISO indicators (e.g., `KES`) are displayed at 85% computed opacity compared to the primary numeral figures to reduce scanning friction.

## Layout & Spacing

Engineered specifically for desktop desktop applications (optimized for 1440×900 baseline canvas). The interface utilizes an analytical 12-column layout pinned to a continuous 4px base increment system.

- **Workspace Anatomy**:
  - **Fixed Navigation Column**: 240px static width rail for high-level module navigation and balance summaries.
  - **Main Data Stage**: Fluid grid spanning remaining viewport, constrained by 24px window padding and 16px internal gutter channels.
  - **Auxiliary Ledger Flyout**: 380px contextual drawer for transaction audit trails and entry editing.
- **Graph Grid System**:
  - The window background features a persistent CSS/SVG geometric grid (`16px × 16px`) with 1px stroke weight set at `0.05` opacity in light mode and `0.04` in dark mode.
  - Key structural panels align their bounding borders precisely along the 16px grid coordinates to eliminate visual vibration.

## Elevation & Depth

Depth is established through low-contrast hairline outlines and tonal surface stratification rather than heavy blur shadows, maintaining a technical desktop aesthetic.

- **Surface Tiers**:
  - **Canvas Base**: Grid-etched surface (`#fbfbf9` / `#0f1115`).
  - **Panel / Ledger Surface**: Solid un-textured plate (`#ffffff` / `#18191e`) resting above the grid.
  - **Sub-row / Cell Hover**: Offset wash (`#f8f8f6` / `#1e2027`).
- **Borders & Outlines**:
  - Panels rely on single-pixel solid borders (`1px solid #e2e8f0` in light; `1px solid #27272a` in dark).
  - Hover states on clickable panels transition border colors to primary-toned slate (`#cbd5e1` / `#3f3f46`) rather than expanding elevation.
- **Floating Overlay Elements (Modals, Popovers, Date Pickers)**:
  - Bounded by sharp 1px borders and a subtle, architectural drop shadow: `0 8px 24px -4px rgba(17, 24, 39, 0.08), 0 2px 6px -1px rgba(17, 24, 39, 0.04)`.

## Shapes

The shape system adopts a refined, tailored radius standard (`4px` to `8px`) that balances crisp ledger formality with modern software refinement.

- **Micro Elements (Tags, Indicators, Inputs, Buttons)**: Formed with a 4px corner radius (`rounded-sm`), delivering geometric discipline.
- **Panels, Tables, and Cards**: Formed with an 8px corner radius (`rounded-lg`) along the exterior boundaries, while interior sub-cells and table headers preserve square internal corners (0px) against divider borders.
- **Pill Shapes**: Strictly reserved for financial status pills (e.g., `Cleared`, `Reconciled`, `Disputed`) to visually isolate categorical state badges from operational actions.

## Components

### Buttons
- **Primary**: Solid rich violet (`#582be8`), text pure white, height 36px, horizontal padding 14px, 4px corner radius, font weight 600 (Plus Jakarta Sans). Hover state: `#4c24cc`.
- **Secondary / Neutral**: White surface with 1px border (`#e2e8f0`), text slate charcoal (`#111827`), hover background `#f8f8f6`.
- **Destructive**: Soft rose surface (`#fff1f2`) with coral text (`#e11d48`) and hairline rose border; transitions to solid crimson on confirmation triggers.

### Data Tables & Ledgers
- **Header Cells**: 32px height, 11px JetBrains Mono uppercase, letter spacing 0.06em, text muted zinc (`#64748b`), hairline bottom border (`1px solid #e2e8f0`).
- **Data Rows**: 40px standard density height, vertical centering, background alternation optional; hover state applies an immediate `#f8f8f6` / `#1f2128` wash.
- **Alignment Rules**: Text labels and payees left-aligned; categories, accounts, and dates centered; all balances, debits, credits, and KES amounts strictly right-aligned with fixed tabular spacing.

### Input Fields & Controls
- **Standard Field**: 36px height, 1px border (`#e2e8f0`), background `#ffffff`, 4px radius. Focus state swaps border to `#582be8` with an immediate `0 0 0 1px #582be8` crisp ring (no blurred glow).
- **Monetary Input**: JetBrains Mono 14px, leading static badge reading currency code `KES` in desaturated weight, followed by right-aligned integer and decimal input.

### Status Chips & Tags
- **Success (Settled/Income)**: Mint background (`#ecfdf5`), border `#a7f3d0`, text `#065f46`, 20px height, pill radius, 11px JetBrains Mono.
- **Expense / Deficit**: Soft rose background (`#fff1f2`), border `#fecdd3`, text `#9f1239`, pill radius.
- **Draft / Pending**: Ochre background (`#fef3c7`), border `#fde68a`, text `#92400e`, pill radius.

### Financial Summary Cards
- White background plate, 8px radius, 1px perimeter border. Divided into an upper title/category row (12px Plus Jakarta Sans, uppercase tracking) and a primary value anchor (24px JetBrains Mono, weight 600), followed by an inline trend indicator badge with directional delta (`+12.4%`).