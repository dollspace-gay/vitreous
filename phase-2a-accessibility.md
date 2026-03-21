---
title: "Implement vitreous_a11y — accessibility tree, focus management, and keyboard navigation"
tags: [design-doc]
sources: []
contributors: [unknown]
created: 2026-03-21
updated: 2026-03-21
---


## Design Specification

### Summary

Implement the accessibility layer that makes vitreous's widget tree the accessibility tree. This includes `AccessibilityInfo` metadata on every node, `FocusManager` for keyboard navigation, AccessKit tree generation, default keyboard interaction tables for all built-in widgets, and dev-mode warnings for missing labels and insufficient contrast ratios.

### Requirements

- REQ-1: `AccessibilityInfo` struct with optional `Role`, `Label`, `Description`, `Value`, `LivePoliteness`, `AccessibilityState`, and `Vec<AccessibilityAction>`
- REQ-2: `AccessibilityState` struct with: disabled, selected, checked (tri-state), expanded (tri-state), has_popup, focusable, focused, read_only, required, invalid, busy, modal, level, value_min/max/now
- REQ-3: `Role` enum covering all standard widget roles: Button, Checkbox, Dialog, Grid, GridCell, Heading, Image, Link, List, ListItem, Menu, MenuItem, ProgressBar, RadioButton, ScrollView, Slider, Switch, Tab, TabList, TabPanel, TextInput, Text, Toolbar, Tooltip, Tree, TreeItem, Window, Group, None
- REQ-4: `FocusManager` with tab-order traversal following document order (depth-first), `focus_next()`, `focus_previous()`, `focus(id)`, `blur()`, `focused()`
- REQ-5: AccessKit tree generation: convert vitreous node tree to `accesskit::TreeUpdate` with correct roles, labels, states, and parent-child relationships
- REQ-6: Default keyboard navigation table: Button responds to Enter/Space, Checkbox to Space, Slider to Arrow keys, Select to Enter/Space/Arrows/Escape, Tab/Shift+Tab for focus cycling
- REQ-7: Dev-mode warnings (debug builds only) for: image without label, interactive element without label, color contrast below WCAG AA thresholds (4.5:1 normal text, 3:1 large text), focus traps
- REQ-8: `LivePoliteness` enum (Off, Polite, Assertive) for live region announcements

### Acceptance Criteria

- [ ] AC-1: `AccessibilityInfo::default()` has all fields as None/empty/false (REQ-1)
- [ ] AC-2: Focus order computation: v_stack with [button("A"), text("skip"), button("B"), text_input] produces focus order [A, B, text_input] — non-focusable text is skipped (REQ-4)
- [ ] AC-3: `focus_next()` from button A moves to button B; `focus_previous()` from button B moves to button A (REQ-4)
- [ ] AC-4: `focus_next()` from last focusable element wraps to first (REQ-4)
- [ ] AC-5: Generated AccessKit tree for `v_stack((text("Heading").role(Role::Heading), button("Click")))` has 3 nodes (root + 2 children) with correct roles (REQ-5)
- [ ] AC-6: Button node with on_click handler responds to simulated Enter and Space key events by invoking the handler (REQ-6)
- [ ] AC-7: Checkbox responds to Space by toggling checked state (REQ-6)
- [ ] AC-8: Slider responds to Left/Right arrows by decrementing/incrementing value (REQ-6)
- [ ] AC-9: In debug mode, `image("test.png")` without `.label()` produces `A11yWarning::MissingLabel` warning (REQ-7)
- [ ] AC-10: Contrast check: white text on white background flags as below WCAG AA threshold (REQ-7)
- [ ] AC-11: AccessKit tree checkbox node with `checked: Some(true)` maps to `accesskit::Toggled::True` (REQ-5)
- [ ] AC-12: `Role::None` produces a presentational node not exposed to assistive technology (REQ-3)

### Architecture

### File Structure

```
crates/vitreous_a11y/src/
├── lib.rs          # Re-exports
├── tree.rs         # AccessKit tree generation from node tree
├── focus.rs        # FocusManager, tab-order computation
├── keyboard.rs     # Default keyboard navigation handlers per Role
├── roles.rs        # Role enum, AccessibilityInfo, AccessibilityState, LivePoliteness
└── warnings.rs     # Dev-mode a11y warnings (contrast, missing labels, focus traps)
```

### Dependencies

- `vitreous_events` — for `KeyEvent`, `Key` types used in keyboard navigation
- `accesskit` (0.24) — for `TreeUpdate`, `NodeBuilder`, role mapping

### AccessKit Tree Generation

The `generate_accesskit_tree` function walks the vitreous node tree depth-first. For each node with `AccessibilityInfo`:

1. Create an `accesskit::NodeBuilder`
2. Map `Role` to `accesskit::Role`
3. Set label, description, value
4. Map `AccessibilityState` fields to AccessKit properties (toggled, expanded, disabled, etc.)
5. Set parent-child relationships
6. If `Role::None`, mark as presentation (excluded from AT)

The tree is regenerated incrementally — only nodes whose `AccessibilityInfo` changed since last frame emit updates.

### Focus Order Computation

Tab order = depth-first traversal filtered to nodes where `focusable == true`. Built-in widgets set `focusable` based on role:
- Always focusable: Button, TextInput, Checkbox, Toggle, Slider, Select, Tab, Link
- Never focusable by default: Text, Image, Container (Group/None)
- Override via `.focusable(bool)` modifier

### WCAG Contrast Check

Relative luminance formula: `L = 0.2126*R + 0.7152*G + 0.0722*B` (where R,G,B are linearized sRGB).
Contrast ratio = `(L1 + 0.05) / (L2 + 0.05)` where L1 is lighter.
WCAG AA requires 4.5:1 for normal text (< 18pt), 3:1 for large text (>= 18pt or >= 14pt bold).

Warnings are collected during tree traversal and emitted via `tracing::warn!` in debug builds only. No runtime cost in release builds.

### Out of Scope

- Platform-specific a11y bridge setup (that's `vitreous_platform` Phase 5 and `vitreous_web` Phase 4B)
- Custom `tab_index` ordering (basic tab-index modifier can be added post-v1)
- Screen reader testing automation
- WCAG AAA compliance (targeting AA only)

