---
slug: linux-compositor-agnostic
status: done
owner: krealalejo
created: 2026-10-29
approved_by: krealalejo (2026-10-29)
---

# PRD — Drag on any layer-shell compositor, seen/jump-back on Sway

## Mission

After `linux-native`, dragging the notch on Wayland reads the global pointer from Hyprland's IPC,
so on Sway or KDE the notch shows but cannot be dragged, and seen-clears-it and jump-back only
know Hyprland and X11. Wayland already delivers every pointer motion to the surface the button
was pressed on until it is released (the implicit grab), in that surface's coordinates; with the
notch's own placed rectangle that is enough to know where the pointer is on any compositor.

**Done when:** a Ctrl-drag on Hyprland follows the pointer with the Hyprland pointer query
unused, and on Sway the focused window and window focus come from Sway's IPC.

---

## In Scope

- [x] **Drag from the implicit grab.** The GDK event handler records pointer motion while the
      left button is down, in the notch surface's logical coordinates; during a drag the
      pointer position is the notch's placed origin plus that offset times the scale.
      Hyprland's `cursorpos` is no longer used for dragging.
- [x] **Sway seen-clears-it and jump-back.** `fg_pid()` from the focused view in Sway's tree
      (`GET_TREE` over `$SWAYSOCK`), and `focus_terminal()` focuses the view nearest the
      session's process chain with `[con_id=<id>] focus`.
- [x] README Linux section updated.

## Out of Scope

- KDE/KWin focus and seen-clears-it (needs a KWin script or D-Bus).
- i3 on X11 through the same IPC protocol.
- Any change to Windows behaviour, CI, packaging beyond a release bump.
- Removing Hyprland IPC for seen-clears-it.

## Architecture

**Approach:** pointer motion is captured in the existing global GDK event handler
(`linux_pointer.rs`), converted to toplevel-surface coordinates, and turned into physical
desktop coordinates in `linux_layer::cursor`. A new `linux_sway.rs` speaks the i3-ipc framing
(`i3-ipc` magic, length, type) with std only; `focus.rs` tries Hyprland, then Sway, then
`xdotool`. Rejected: per-compositor pointer queries — Sway and KWin expose none.

**Touches:** `linux_pointer.rs`, `linux_layer.rs`, `linux_sway.rs` (new), `focus.rs`,
`main.rs` (module), `README.md`.

**Data model / contracts / dependencies:** none.

**Risk & rollback:** if a compositor does not keep sending motion outside the surface, the
drag stops following at the notch's edge; detected on the first drag. Each item is its own
commit on `linux-native`.

## Verification

| # | Check | How | Result |
|---|-------|-----|--------|
| 1 | Unit tests | `cargo test --locked`, all `scripts/*.cjs` | 308 passed, all UI scripts pass |
| 2 | Drag without Hyprland's pointer query | Ctrl-drag on Hyprland, `cursorpos` removed | Confirmed by the user |
| 3 | Sway IPC parsing | Unit tests: tree parsing, view choice, framing against a fake socket | Pass |
| 4 | Sway live | Not available on this machine | Unverified |

## Log

- `2026-10-29` — Approved in-session after the plan was described. Sway is not installed here,
  so Sway is verified by unit tests only.
