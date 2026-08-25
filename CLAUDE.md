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
window) talking to a local LM Studio server, always `agent::DEFAULT_BASE_URL`
(`http://localhost:1234/v1`) — no endpoint/model picker in the UI; the model id is auto-detected
via `GET /models` on every turn. The agent can call this app's own `generate_key`/`encrypt`/
`decrypt`/`convert` as tools (see `crates/agent/src/lib.rs`). No dedicated top bar — the toggle lives in the
sidebar header to avoid adding a permanent strip above the layout. The conversation lives on the
always-alive `CrittoUtil` entity (`AgentState::messages`), so it survives closing and reopening
the panel and only resets when the app restarts.

Because many local models loaded in LM Studio don't reliably emit real OpenAI-style `tool_calls`,
`agent::run_turn` mirrors the sibling `tauri-crittoutil-shadcn` app's proven fallback: if a model
narrates a call as a JSON blob in plain text instead, `extract_pseudo_tool_call` finds and
executes it for real (validating the name against `TOOL_NAMES` and accepting the `name`/`tool`/
`function` and `arguments`/`parameters`/`args` aliases models commonly use); if it just writes a
bare mention like `encrypt("hello")` with no arguments, `looks_like_bare_tool_call` detects that
and re-prompts the model instead of giving up. The chat panel's tool-call dropdown (collapsed by
default, click to expand name/arguments/result) only renders for genuine `tool_calls`, same as
the Tauri version — the text-fallback path is intentionally invisible plumbing, not something
worth surfacing as its own UI block.

Inside a session, six screens, navigated from the left sidebar:
- **Home** — free-text search that scores against per-feature keywords and suggests a screen.
- **Converter** — text ↔ binary ↔ Base64.
- **Key Generator** — random alphanumeric key generation (64–512 bit), with history.
- **Encrypter** / **Decrypter** — MD5, AES-CBC/ECB, DES-CBC/ECB, with key/IV validation matching
  the original app's rules (byte-length checks, `+`/`/`=` base64-vs-plain-text IV heuristic).
- **File Hasher** — MD5 of a file picked via the native file dialog (`cx.prompt_for_paths`).

## Structure

This is a Cargo **workspace** (`crates/*`), split by feature capability rather than by
implementation role — with one unavoidable exception forced by gpui's entity model, explained
below. The root `Cargo.toml` is `[workspace]`-only; all dependency versions/git revisions live
once in `[workspace.dependencies]` and every crate pulls them in via `dep = { workspace = true }`.

Pure-logic feature crates (no gpui dependency, freely reusable/testable in isolation):

- `crates/crypto_core` (`src/crypto.rs` + `src/crypto_meta.rs`) — the crypto core, ported
  ~verbatim from the Tauri app's Rust backend (it was already pure Rust), plus algorithm metadata
  (key/IV length requirements) and encrypt/decrypt dispatch, ported from the original app's JS
  composable. Keep its `#[cfg(test)]` unit tests intact — they're the functional-parity contract
  for this module; don't weaken or delete them.
- `crates/converter` — text/binary/base64 conversion logic + its own unit tests.
- `crates/home` — the Home screen's keyword-scoring search (`home_search.rs`), English-only (the
  original had an it/en toggle; this port skips i18n), plus the `Route` enum naming every
  navigable screen. `Route` lives here rather than in `crates/app` because Home is what actually
  needs to name every screen to score a search query against it; `crates/app` re-exports it as the
  single source of truth for navigation.
- `crates/session` — session persistence (`Session`, `StoredKeyEntry`, `load_all`/`save_all`,
  `~/Library/Application Support/gpui-crittoutil/sessions.json`), plus `KeyEntry`, the shared
  key/IV history entry type. `KeyEntry` lives here rather than in `crates/app` since it's data
  persisted by, and shared across, every feature that reads/writes the key history (key generator,
  encrypter, decrypter).
- `crates/agent` — the LM Studio chat/tool-calling logic (`run_turn`, `extract_pseudo_tool_call`,
  `looks_like_bare_tool_call`, `fetch_first_model_id`, `DEFAULT_BASE_URL`), built on top of
  `crypto_core` and `converter`. Pure request/response logic — no gpui dependency.

`crates/app` — the gpui-dependent binary crate, composing everything above:

- `src/main.rs` — binary entry point (window setup, `Root` wrapping).
- `src/app.rs` — the single top-level `CrittoUtil` entity. **All** view state lives here as plain
  fields (`Route` re-exported from `home`, per-screen form state, shared
  `key_history: Vec<session::KeyEntry>`) — see the entity-nesting rule below for why.
- `src/theme.rs` + `../../themes/custom.json` — a custom light/dark theme whose palette is adapted
  from Zed's built-in "One" theme (`assets/themes/one/one.json` in the Zed repo) — same neutral
  grays, border tones and accent blue as Zed's editor UI, remapped onto gpui-component's theme
  schema, loaded the same way as the sibling `gpui-playground` project's `theme.rs`.
- `src/ui/key_picker.rs` — the reusable "pick a key/IV from shared history" modal dialog, used by
  the key generator, encrypter and decrypter screens.
- `src/views/*.rs` — one plain function per screen (`pub fn render(app: &CrittoUtil, window, cx) -> impl IntoElement`),
  plus `sidebar.rs` for navigation and `mod.rs` for cross-screen helpers (`result_tile`,
  `radio_row`, `field_with_picker`). None of these are `cx.new(...)` entities.

**Why views/state aren't split into their own feature crates alongside their logic.** The
"feature crate = state + view + logic together" ideal (as used for the pure-logic crates above)
runs into a hard wall for anything interactive: every view function needs `&CrittoUtil` and
`&mut Context<CrittoUtil>` (for `cx.listener`, click handlers, `cx.notify()`, etc.), and gpui's
`Context<T>`/`Entity<T>` are concrete over `T = CrittoUtil`. Since the whole app is one entity
(see house rule 1), *any* code that touches `Context<CrittoUtil>` is necessarily coupled to the
crate that defines `CrittoUtil` — splitting `views/encrypter.rs` into a standalone
`crates/encrypt_decrypt` would require either making `CrittoUtil` generic (defeats the point of a
single concrete entity) or that crate depending on `crates/app` while `crates/app` also depends on
it to render the screen, an unavoidable cycle. So `crates/app` owns `CrittoUtil` itself, every
per-screen state struct embedded in it (`ConverterState`, `KeyGeneratorState`, `EncrypterState`,
`DecrypterState`, `FileHasherState`, `AgentState`), and every `views/*.rs` render function,
organized into one file per feature to preserve feature-oriented organization as far as gpui's
model allows. What *did* move out cleanly are the feature crates' pure data/logic and gpui-free
data types (`crypto_core`, `converter`, `home`'s `Route`, `session`'s `KeyEntry`/`Session`,
`agent`'s tool-calling loop) — `crates/app`'s views are thin gpui glue on top of those.

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
   specific git revisions once, in the root `Cargo.toml`'s `[workspace.dependencies]` (and
   `Cargo.lock`) — copied from `gpui-playground` for the first two. Don't let a bare
   `cargo update` re-resolve them to different, untested commits; bump deliberately and re-test
   hover/interactivity if you do. Note `gpui`/`gpui_platform` are pinned by plain `git = "..."`
   URL with no `rev`, matching how `gpui-component` itself depends on `gpui` transitively — adding
   an explicit `rev` there makes Cargo see two different source strings for the same commit and
   resolve two copies of the crate, which fails to compile.
3. No raw hex/`rgb`/`hsla` colors in view code — resolve everything through `cx.theme()` (see
   `crates/app/src/views/mod.rs` for the shared surface helpers: `result_tile`, `radio_row`,
   `field_with_picker`). The theme itself (`themes/custom.json`) is the one place color values are
   defined directly.
4. Icons come from `gpui-component-assets`' bundled set, which is small and has no dedicated
   home/lock/key glyphs. The sidebar (`crates/app/src/views/sidebar.rs`) picks the closest
   thematic substitutes (`layout-dashboard`, `replace`, `cpu`, `eye-off`, `eye`, `file`) rather
   than inventing new icon assets — check the bundled SVG list before assuming an icon name exists.
5. On a selected/active `Button` (solid `primary` background), any icon or text inside it should
   use `cx.theme().primary_foreground` for contrast, not an arbitrary accent color — the button's
   own background already carries the "selected" signal.

## Deliberate simplifications vs. the original Tauri app

- Algorithm/type/key-size pickers use the standard `radio::{Radio, RadioGroup}` components
  (via the `views::radio_row` helper) rather than a `Select`/dropdown — a fixed, always-visible
  set of 2-6 mutually exclusive options reads better as a radio group than a dropdown.
- Key/IV "pick from history" opens a modal `Dialog` (`crates/app/src/ui/key_picker.rs`'s
  `open_key_picker`), not an inline row of buttons.
- Copy-to-clipboard has no toast/snackbar confirmation.
- No i18n — English only.

## Testing

`cargo test --workspace` covers `crypto_core::crypto` (ported 1:1 from the original, including
edge cases like wrong key length, invalid IV, wrong-key decryption failure), `converter`, and
`home`'s `home_search`. Treat these as regression tests for functional parity with the original
app — if you change behavior, update the test that encodes the old behavior deliberately, don't
just delete it.
