---
slug: linux-native
status: approved
owner: krealalejo
created: 2026-10-29
approved_by: krealalejo (2026-10-29)
---

# PRD — Codenotch runs properly on Linux (Hyprland first)

## Mission

The Tauri port in `windows/` builds on Linux, but there it is a degraded X11 client: it
needs XWayland and compositor window rules to sit on an edge, it cannot be dragged, it never
clears a "done" session when you look at it, clicking it cannot jump back to the terminal,
and there is no way to install it as a regular app. This work closes those gaps for a
Hyprland desktop on Arch/CachyOS, while keeping the current X11 path as the fallback for
desktops without layer-shell (GNOME).

**Done when:** on Hyprland, `makepkg -si` installs Codenotch, it starts from the app
launcher as a native Wayland layer-shell surface on the chosen edge with no window rules,
it can be dragged along the border, a "done" Claude Code session clears when its terminal
is focused, and clicking it focuses that terminal.

---

## In Scope

- [ ] **Native Wayland edge.** When the compositor supports `wlr-layer-shell`, the notch
      and the carry overlay are layer-shell surfaces anchored to the configured edge and
      monitor, placed with anchors and margins instead of `set_position`. Without
      layer-shell, behaviour falls back to today's X11 path unchanged.
- [ ] **Pointer on Wayland.** The hover poll (`main.rs`) and the drag loop (`carry.rs`)
      read the global pointer from Hyprland IPC (`cursorpos`) when running native on
      Hyprland, because `cursor_position()` has no global meaning on Wayland.
- [ ] **Drag along the edge.** A Linux `left_button_down()` so `carry.rs` works: GDK
      pointer button state on X11; on Wayland the press/release seen by the notch surface.
- [ ] **Seen-clears-it.** Linux `proc_maps()` / `chain_of()` / `pid_hits_chain()` from
      `/proc`, and `fg_pid()` from Hyprland IPC (`activewindow`), or `_NET_ACTIVE_WINDOW`
      + `_NET_WM_PID` on X11, so `ack_scan` runs on Linux.
- [ ] **Jump back to the terminal.** Linux `focus_terminal()` walks the Claude process's
      parent chain to a window-owning pid and focuses it via `hyprctl dispatch
      focuswindow pid:<n>` (Hyprland) or `_NET_ACTIVE_WINDOW` (X11).
- [ ] **Packaging.** A `PKGBUILD` under `windows/packaging/arch/` that builds the app and
      the hook, installs the binary, a `.desktop` entry and icons. The Linux launcher sets
      what `run-linux.sh` sets today, minus forcing X11 when layer-shell is available.
- [ ] **Updater off on Linux.** The in-app update check is hidden/disabled on Linux.
- [ ] README Linux section updated to match.

---

## Out of Scope

- Antigravity on Linux (libsecret credential reading).
- App icons taken from installed applications.
- KDE/Sway/GNOME-specific IPC for focus and pointer. Layer-shell placement works on any
  layer-shell compositor; pointer and focus beyond Hyprland use the X11 path or degrade.
- `claude_io_bytes` / activity detection that is still Windows-only in `activity.rs`.
- `.deb`, AppImage, Flatpak, AUR publishing, CI changes, release workflow.
- Any change to the macOS app (`Sources/`) or to Windows behaviour.
- Opening a pull request upstream.

---

## Architecture

**Approach:** Every Linux implementation goes behind `cfg(target_os = "linux")` beside the
existing `cfg(windows)` code, matching how the port already splits platforms. A small
`linux/` module holds a Hyprland IPC client (std `UnixStream` to
`$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`), `/proc` readers, and
the layer-shell setup, applied to the GTK window behind each Tauri webview window before it
is first shown. Rejected alternative: keep X11 and ship Hyprland window rules — still
blurry under fractional scaling, depends on user config, and does not fix pointer or focus.

**Touches:**

| Path | Change |
|------|--------|
| `windows/codenotch/Cargo.toml` | add `gtk-layer-shell` for Linux only |
| `windows/codenotch/src/linux/*.rs` (new) | Hyprland IPC, `/proc`, layer-shell helpers |
| `windows/codenotch/src/main.rs` | Linux `left_button_down`, `ack_scan`, placement, hover pointer |
| `windows/codenotch/src/carry.rs` | Linux placement of the overlay and pointer source |
| `windows/codenotch/src/focus.rs` | Linux `fg_pid`, `proc_maps`, `chain_of`, `pid_hits_chain`, `focus_terminal` |
| `windows/codenotch/src/updater.rs` / UI | disable update check on Linux |
| `windows/packaging/arch/*` (new) | `PKGBUILD`, `.desktop`, launcher |
| `windows/scripts/run-linux.sh` | only force X11 when layer-shell is unavailable |
| `windows/README.md` | Linux section |

**Data model:** none. **Contracts:** none; config file format unchanged.

**Dependencies:** `gtk-layer-shell` (Rust bindings, gtk 0.18 line, Linux-only target).
System package `gtk-layer-shell` from the Arch repos. Needs approval.

**Risk & rollback:** The biggest risk is that Tauri/wry's GTK window does not accept
layer-shell after realisation, or that the transparent webview misrenders as a layer
surface; detected on the first manual run. Mitigation: layer-shell is opt-out with
`CODENOTCH_X11=1`, and the X11 path stays intact. Rollback: every slice is its own commit
on `linux-native`; revert the commit.

---

## Verification

| # | Check | How | Result |
|---|-------|-----|--------|
| 1 | Builds and tests pass on Linux | `cargo build --locked && cargo test --locked` in `windows/` | |
| 2 | Windows code still type-checks | `cargo check --target x86_64-pc-windows-gnu` if the target installs; otherwise reported as unverified | |
| 3 | Existing UI script tests pass | `node --test scripts/test-linux-notch.cjs` and the other `windows/scripts/*.cjs` | |
| 4 | Native edge on Hyprland | Run without `GDK_BACKEND=x11`; user confirms placement on each edge | |
| 5 | Drag | User drags the pill along the border and across a corner | |
| 6 | Seen + jump back | Finish a Claude Code task in a terminal, focus it → clears; click ring from another window → terminal focused | |
| 7 | Package | `makepkg -si` in `windows/packaging/arch`, launch from launcher | |

---

## Log

- `2026-10-29` — Antigravity excluded by the user. Discovered that `carry.rs` and the hover
  poll rely on `cursor_position()`, which has no global value on Wayland; native Wayland
  therefore also needs a pointer source (Hyprland IPC), added to scope under the edge item.
