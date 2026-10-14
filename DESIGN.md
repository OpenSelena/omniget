---
version: alpha
name: OmniGet Universal Studio & OmniGuard Security Suite
description: Precision local media workbench and privacy security suite for OmniGet, OmniGuard, and OpenNami.
colors:
  primary: "#FF7D38"
  primary-container: "#F28500"
  secondary: "#0369A1"
  secondary-container: "#38BDF8"
  tertiary: "#8B5CF6"
  surface: "#131B2E"
  surface-variant: "#1E293B"
  background: "#0B0F19"
  outline: "rgba(255, 255, 255, 0.08)"
  outline-variant: "#E2E8F0"
  success: "#10B981"
  text-primary: "#F8FAFC"
  text-muted: "#94A3B8"
typography:
  hero:
    fontFamily: Space Grotesk
    fontSize: 48px
    fontWeight: 700
    letterSpacing: -0.03em
  h1:
    fontFamily: Space Grotesk
    fontSize: 32px
    fontWeight: 700
    letterSpacing: -0.02em
  h2:
    fontFamily: Space Grotesk
    fontSize: 24px
    fontWeight: 700
    letterSpacing: -0.02em
  body:
    fontFamily: Inter
    fontSize: 15px
    fontWeight: 400
    lineHeight: 1.65
  label:
    fontFamily: Inter
    fontSize: 13px
    fontWeight: 600
  mono:
    fontFamily: JetBrains Mono
    fontSize: 13px
    fontWeight: 500
rounded:
  sm: 10px
  md: 16px
  lg: 20px
  full: 9999px
spacing:
  xs: 8px
  sm: 16px
  md: 24px
  lg: 32px
  xl: 48px
  2xl: 64px
  3xl: 80px
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "#0F172A"
    rounded: "{rounded.full}"
    padding: 12px
  button-secondary:
    backgroundColor: "{colors.secondary}"
    textColor: "#FFFFFF"
    rounded: "{rounded.full}"
    padding: 12px
  security-badge:
    backgroundColor: "{colors.success}"
    textColor: "#0F172A"
    rounded: "{rounded.full}"
    padding: 8px
  workbench-card:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.md}"
    padding: 24px
  card-caption:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text-muted}"
    rounded: "{rounded.sm}"
    padding: 4px
  omnibox:
    backgroundColor: "{colors.background}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.lg}"
    padding: 16px
---

# Design System: OmniGet Universal Studio & OmniGuard Security Suite

Design specification for OmniGet desktop suite, OmniGuard privacy shield, and OpenNami social archiver.

## Overview
- **Core Mental Model**: Precision local media workbench. Unifies high-velocity media extraction with offline reading, Hi-Fi listening, and verifiable system security.
- **Surface Materiality**: Frosted glassmorphism (`backdrop-filter: blur(16px)`), precision hairline borders (`1px solid var(--border-line)`), layered card elevations, and tactile feedback. Avoids flat ungrounded cards or generic SaaS fluff.
- **Physical Feedback**: Smooth state transitions (`0.25s cubic-bezier(0.16, 1, 0.3, 1)`), interactive live Omnibox preview with instant platform badge recognition, animated progress rings, and responsive tabs.

## Colors
### Primary Foundations
- **Brand Primary Orange (`#FF7D38` / `#F28500`)**: Download triggers, primary action buttons, key metrics, active focus rings.
- **Accent Tech Sky (`#0284C7` / `#38BDF8`)**: Stream connections, protocol indicators, secondary actions, OpenNami accent.
- **OmniGuard Security Emerald (`#10B981` / `#059669`)**: Cryptographic SHA-256 verification, zero-telemetry badge, integrity check passes, offline status.
- **Course & Academic Violet (`#8B5CF6`)**: Course downloads, notes, PDF attachments, EPUB reader.

### Surface Tones (Adaptive Light & Dark Modes)
- **Light Theme**:
  - Page Canvas: `#F8FAFC`
  - Floating Nav & Surface: `rgba(255, 255, 255, 0.92)`
  - Card Fill: `#FFFFFF`
  - Sub-surface / Code: `#0B0F19`
  - Text Primary: `#0F172A`
  - Text Muted: `#475569`
  - Hairline Border: `#E2E8F0`
- **Dark Theme**:
  - Page Canvas: `#0B0F19`
  - Floating Nav & Surface: `rgba(15, 23, 42, 0.88)`
  - Card Fill: `#131B2E`
  - Card Hover: `#1E293B`
  - Text Primary: `#F8FAFC`
  - Text Muted: `#94A3B8`
  - Hairline Border: `rgba(255, 255, 255, 0.08)`

## Typography
- **Display Typeface**: `'Space Grotesk', 'Plus Jakarta Sans', system-ui, sans-serif`
  - Hero Title: `clamp(2.4rem, 5vw, 3.8rem)`, weight 700, letter-spacing `-0.03em`.
  - Section Titles: `1.85rem` to `2.25rem`, weight 700, letter-spacing `-0.02em`.
- **Body Typeface**: `'Inter', system-ui, -apple-system, sans-serif`
  - Subheadings: `1.15rem` to `1.25rem`, weight 400-500, line-height 1.6.
  - Body Text: `15px`, line-height 1.65, weight 400.
  - Metadata / Badges: `12px` to `13px`, weight 600.
- **Technical Readout & Monospace**: `'JetBrains Mono', 'Fira Code', monospace`
  - Command snippets, SHA-256 digests, hotkey chips (`Ctrl+Shift+D`), terminal streams.

## Layout
- **Standard Viewport**: `1280px` desktop baseline with fluid down-scaling for tablet and mobile.
- **Container Max-Width**: `1120px` centered with `24px` gutter padding.
- **Spatial Grid**: `8px` baseline rhythm (`8px`, `16px`, `24px`, `32px`, `48px`, `64px`, `80px`).

## Elevation & Depth
- **Surface Layers**:
  - Level 0 (Base Canvas): `#0B0F19` (Dark) / `#F8FAFC` (Light).
  - Level 1 (Workbench Cards): `#131B2E` with hairline border `1px solid rgba(255, 255, 255, 0.08)`.
  - Level 2 (Floating Overlays & Nav): Frosted glassmorphism `backdrop-filter: blur(16px)` with subtle elevation shadow `0 8px 32px rgba(0, 0, 0, 0.36)`.
  - Level 3 (Modals & Tooltips): Highest elevation with crisp edge separation and dark backdrop veil `rgba(0, 0, 0, 0.7)`.

## Shapes
- **Radius Scale**:
  - Pill badges & buttons: `9999px` (`full`)
  - Cards & containers: `16px` to `20px` (`md` to `lg`)
  - Interactive inputs & chips: `10px` to `12px` (`sm`)

## Components
1. **Floating Pill Navigation Bar**: High-contrast blur pill housing brand mascot (`Loop`), live tab switches (`Home`, `OmniGuard`, `OpenNami`, `158 Tools`, `Downloads`, `Release Notes`, `FAQ`), theme toggle, and GitHub star badge.
2. **Interactive Omnibox Simulator**: Real-time URL parser with platform icon auto-detection (YouTube, Udemy, Instagram, TikTok, Bilibili, Magnet), dynamic resolution selector, and simulated live download stream with real-time ETA/speed telemetry.
3. **OmniGuard Security & Privacy Matrix**: Detailed cryptographic security view with real SHA-256 hash checks, zero-telemetry guarantee, localhost 127.0.0.1 token verification, and sandboxed ABI isolation cards.
4. **Interactive 158 Tools Hub**: 25-category filterable toolbox with live instant search query filter, status tags (`Ready`, `Beta`, `Planned`), and Rust command-line inspector.
5. **OpenNami Social Archiver Shell**: Embedded interactive TUI and desktop console mirror for Instagram, TikTok, and X downloads with circular progress ring.
6. **Operating System Platform Matrix**: Direct detection of client OS with instant Windows (.exe/portable/.msi), macOS (Universal M1-M4 DMG), and Linux (.AppImage/.deb/.rpm) download cards.

## Do's and Don'ts
- **DO**:
  - Always use the 8px spatial grid for margins, paddings, and gaps.
  - Apply hairline borders (`1px solid var(--border-line)`) to cards for physical grounding.
  - Reserve Brand Orange (`#FF7D38`) for key triggers and call-to-actions.
  - Preserve frosted glassmorphism (`backdrop-filter: blur(16px)`) on floating elements.
- **DON'T**:
  - Do not use generic purple/indigo AI gradients.
  - Do not float cards without borders or grounding shadows.
  - Do not use low-contrast muted gray text for essential body copy.
  - Do not introduce arbitrary border radii outside the 10px / 16px / 20px / 9999px scale.
