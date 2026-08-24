# gpui-crittoutil

Native desktop crypto utility, built with [gpui](https://github.com/zed-industries/zed) and
[gpui-component](https://github.com/longbridge/gpui-component). It's a functional port of
`tauri-crittoutil` (Tauri + Vue) — same features, same validation rules, native Rust UI instead
of a webview.

## What it does

Six screens, navigated from the left sidebar:
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
  plus `sidebar.rs` for navigation (built on gpui-component's standard `sidebar::{Sidebar, SidebarHeader,
  SidebarMenu, SidebarMenuItem}`, collapse animation disabled). None of these are `cx.new(...)` entities.
- `src/ui/style.rs` — small shared styling helpers (`surface`, `card`) — opaque theme-derived
  surfaces with a border and soft shadow. A glass/translucent style was tried and explicitly
  rejected in favor of this solid look.
- `src/theme.rs` + `themes/custom.json` — a custom warm-neutral light/dark theme (macOS system blue accent),
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
