# gpui-crittoutil

Native desktop crypto utility, built with [gpui](https://github.com/zed-industries/zed) and
[gpui-component](https://github.com/longbridge/gpui-component). It's a functional port of
`tauri-crittoutil` (Tauri + Vue) — same features, same validation rules, native Rust UI instead
of a webview.

## What it does

The app opens on a **session picker**: "New session" or resume one from the "Recent sessions"
list. A session is just its key/IV history (`session::Session`), persisted as JSON at
`~/Library/Application Support/gpui-crittoutil/sessions.json` — per-screen form fields are
transient UI state and aren't part of what a session restores. "All sessions" at the bottom of
the sidebar returns to the picker (saving the current session first).

A small bot icon in the sidebar header toggles **agentic mode**: a chat panel (right side of the
window) talking to a local LM Studio server (`http://localhost:1234/v1` by default) that can call
this app's own `generate_key`/`encrypt`/`decrypt` as tools (see `src/agent.rs`). No dedicated top
bar — the toggle lives in the sidebar header to avoid adding a permanent strip above the layout.

Inside a session, six screens, navigated from the left sidebar:
- **Home** — free-text search that scores against per-feature keywords and suggests a screen.
- **Converter** — text ↔ binary ↔ Base64.
- **Key Generator** — random alphanumeric key generation (64–512 bit), with history.
- **Encrypter** / **Decrypter** — MD5, AES-CBC/ECB, DES-CBC/ECB, with key/IV validation matching
  the original app's rules (byte-length checks, `+`/`/`=` base64-vs-plain-text IV heuristic).
- **File Hasher** — MD5 of a file picked via the native file dialog (`cx.prompt_for_paths`).

## Structure

- `src/crypto.rs` — the crypto core, ported ~verbatim from the Tauri app's Rust backend (it was
  already pure Rust). Keep its `#[cfg(test)]` unit tests intact — they're the functional-parity
  contract for this module; don't weaken or delete them.
- `src/crypto_meta.rs` — algorithm metadata (key/IV length requirements) and encrypt/decrypt
  dispatch, ported from the original app's JS composable.
- `src/converter.rs` — text/binary/base64 conversion logic + its own unit tests.
- `src/home_search.rs` — the Home screen's keyword-scoring search, English-only (the original had
  an it/en toggle; this port skips i18n).
- `src/app.rs` — the single top-level `CrittoUtil` entity. **All** view state lives here as plain
  fields (route enum, per-screen form state, shared `key_history: Vec<KeyEntry>`) — see the
  entity-nesting rule below for why.
- `src/views/*.rs` — one plain function per screen (`pub fn render(app: &CrittoUtil, window, cx) -> impl IntoElement`),
  plus `sidebar.rs` for navigation. None of these are `cx.new(...)` entities.
- `src/views/sidebar.rs` is a hand-rolled nav rail, not gpui-component's `sidebar::{Sidebar,
  SidebarMenu, SidebarMenuItem}` — those components bake in behavior (a hover highlight on
  `SidebarHeader`, `overflow: hidden` on the panel that also clips its own box-shadow) that
  fought the floating-card look this app wants, so plain `div()`s give full control instead. It's
  a macOS-app-sidebar style card: inset via `.m_4()`, rounded via `.rounded(px(10.0))`, a
  `.shadow_lg()` painted on an *unclipped outer wrapper* (the inner panel needs its own
  `overflow_hidden` to clip content to its rounded corners, which would clip a shadow painted on
  that same element — hence the two nested divs), background from the `sidebar.background` theme
  token. Active/hover nav-row state uses `sidebar_accent`/`sidebar_accent_foreground` (active,
  solid) and `muted` (hover) theme tokens — no raw colors.
- The content panel (`app.rs`) has no background/border of its own — it just shows the window
  background behind the sidebar card. No `src/ui/style.rs` helper exists anymore; per the
  [gpui-component design guide](https://longbridge.github.io/gpui-component/docs/design-guides),
  shadows are reserved for elevated/floating elements, not stacked onto every panel — here that's
  the sidebar card, not the flat content area.
  - Every "secondary" info/output box (converter output, encrypt/decrypt result, generated key,
    file info, MD5 hash) shares one treatment: `bg(theme.secondary)` + `border_1()` +
    `border_color(theme.border)` — same-kind surfaces must look identical. The Home screen's
    feature-card list is a different kind (nav row, not an output box) and is intentionally
    background-less — border only.
  - Every screen's root div owns its own `.p_6()` inset and `.overflow_y_scroll()`.
- `src/theme.rs` + `themes/custom.json` — a custom light/dark theme whose palette is adapted from
  Zed's built-in "One" theme (`assets/themes/one/one.json` in the Zed repo) — same neutral grays,
  border tones and accent blue as Zed's editor UI, remapped onto gpui-component's theme schema,
  loaded the same way as the sibling `gpui-playground` project's `theme.rs`.

## House rules (inherited from `gpui-playground/CLAUDE.md` — read that file too)

1. **Entity nesting depth breaks hover repaint.** Empirically confirmed on the pinned gpui/
   gpui-component commits: more than ~2 levels of `cx.new(...)` entities silently breaks
   `.hover()`'s repaint (mouse enter/leave stops visually updating until an unrelated redraw, e.g.
   a click, forces one). This is why the whole app is ONE entity (`CrittoUtil`) with every other
   piece of UI — sidebar, each screen, list rows, buttons — rendered as a plain function returning
   `impl IntoElement`. Do not introduce a new `cx.new(...)` for a visual fragment; only
   `Entity<InputState>` (gpui-component's own text-input state) is exempt, since it's a required
   leaf entity for text editing to work at all.
2. Dependencies (`gpui`, `gpui_platform`, `gpui-component`, `gpui-component-assets`) are pinned to
   specific git revisions in `Cargo.toml`/`Cargo.lock` — copied from `gpui-playground` for the
   first two. Don't let a bare `cargo update` re-resolve them to different, untested commits;
   bump deliberately and re-test hover/interactivity if you do.
3. No raw hex/`rgb`/`hsla` colors in view code — resolve everything through `cx.theme()` (see
   `src/ui/style.rs` for the shared surface helpers). The theme itself (`themes/custom.json`) is
   the one place color values are defined directly.
4. Icons come from `gpui-component-assets`' bundled set, which is small and has no dedicated
   home/lock/key glyphs. The sidebar (`src/views/sidebar.rs`) picks the closest thematic
   substitutes (`layout-dashboard`, `replace`, `cpu`, `eye-off`, `eye`, `file`) rather than
   inventing new icon assets — check the bundled SVG list before assuming an icon name exists.
5. On a selected/active `Button` (solid `primary` background), any icon or text inside it should
   use `cx.theme().primary_foreground` for contrast, not an arbitrary accent color — the button's
   own background already carries the "selected" signal.

## Deliberate simplifications vs. the original Tauri app

- Algorithm/type/key-size pickers use the standard `radio::{Radio, RadioGroup}` components
  (via the `views::radio_row` helper) rather than a `Select`/dropdown — a fixed, always-visible
  set of 2-6 mutually exclusive options reads better as a radio group than a dropdown.
- Key/IV "pick from history" opens a modal `Dialog` (`ui::key_picker::open_key_picker`), not an
  inline row of buttons.
- Copy-to-clipboard has no toast/snackbar confirmation.
- No i18n — English only.

## Testing

`cargo test` covers `crypto.rs` (ported 1:1 from the original, including edge cases like wrong
key length, invalid IV, wrong-key decryption failure), `converter.rs`, and `home_search.rs`. Treat
these as regression tests for functional parity with the original app — if you change behavior,
update the test that encodes the old behavior deliberately, don't just delete it.
