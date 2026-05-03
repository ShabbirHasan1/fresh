# E2E Test Migration — From Imperative Harness to Declarative Theorems

**Status:** Phases 1, 2, 3, A, B, C, D all landed on
`claude/e2e-test-migration-design-HxHlO`. Test suite green (134 passing,
0 ignored). Framework proven by finding *and fixing* three real
production bugs and surfacing two further behavioral asymmetries
(Redo cursor; ExpandSelection on punctuation runs).
**Branch:** `claude/e2e-test-migration-design-HxHlO`
**Owner:** TBD
**Scope:** `crates/fresh-editor/tests/e2e/*` (~227 files)

**Forward plan (2025-Q3+):** see **Part II — Realignment toward full
coverage** at the bottom of this doc (§11–§19). The original §6/§9
expansion plan deferred ~50% of the suite (Class B viewport, modal
UI, LSP, filesystem, rendering); Part II revises that and targets
**every e2e file**, including rendering, on the basis that each
migrated test now produces three artifacts (regression + proptest
seed + shadow-model check) instead of one. Where Part I and Part II
disagree on forward direction, Part II takes precedence.

## What's landed (cumulative)

| Track | Commit | Lines | Result |
|---|---|---|---|
| 0 — design | `48a6c92` | +950 docs | This document |
| 1 — `EditorTestApi` seam | `af066ad` | +295 | Trait + `Caret` projection on `Editor`, `harness.api_mut()`, isolation lint script, smoke test |
| 2 — `BufferTheorem` + PoC | `448fd4f` | +245 | Runner framework, `tests/semantic/case_conversion.rs` rewritten as a 12-line theorem |
| 3 — Multi-cursor + Trace + minimal Layout | `d93d483` | +298 | Multi-cursor coverage, undo-roundtrip runner, viewport_top_byte observable |
| Result-shape refactor | `a22a47d` | +388 | `check_*` returns `Result<(), TheoremFailure>`; `assert_*` is a thin panicking wrapper. Enables external drivers (fuzzers, generators, proof-search) without `catch_unwind` |
| A — proptest properties | `53ec62c` | +356 | 3 properties driven by `check_*`; **found two real production bugs** (`actions.rs:1613` smart-dedent panic, `state.rs:462` delete-backward OOB) in 70s of fuzzing |
| B — E2E migrations | `9de5787` | +302 | sort_lines (3), indent_dedent (3), select_to_paragraph (2), smart_home (2). Theorem revealed an unstated `SortLines` selection-clearing asymmetry. |
| C — observables on demand | (in B commit) | — | `TerminalSize` + `assert_buffer_theorem_with_terminal` added because smart_home's wrap variant needs custom dimensions |
| D1 — serde failures | `4925f8a` | ~40 | `TheoremFailure: Serialize + Deserialize`; JSON round-trip meta-test. External drivers can write to dashboards / CI artifacts / replay logs without string parsing. |
| **Bug fixes (1)** | `d95b9d1` | +13/-21 | Both latent bugs fixed: `prefix_bytes.last()` instead of OOB indexing in actions.rs; `.min(deleted_text.len())` clamp in state.rs. All `#[ignore]` annotations removed. |
| **Track B (continued)** | `ad4887a` | +364/-30 | `duplicate_line` (5 theorems), `emacs_actions` (8 + 1 #[ignore]'d bug). Theorem migration of `test_open_line_basic` revealed a 3rd bug: `Action::OpenLine` advances the cursor instead of staying put (Emacs C-o intent). |
| **Bug fixes (2)** | `6b2f144` | small | OpenLine handler emits a follow-up `Event::MoveCursor` to restore the cursor position. The `#[ignore]`'d theorem flips to a passing regression test plus a "type-after-OpenLine inserts on the original line" companion. |
| **Track B (3)** | `e0b2a43` | +520 | `undo_redo` (4 theorems) + `auto_pairs` (14 theorems) + `BehaviorFlags` framework extension. Migration of `test_redo_skips_readonly_movement_actions` revealed a 4th asymmetry: Redo doesn't re-advance the cursor past the re-inserted bytes (pinned, not yet fixed). |
| **Track B (4)** | `79c648e` | +421 | `toggle_comment` (10 theorems) + 6 quote auto-pair theorems + `load_buffer_from_text_named` framework extension. Resolves the deferred language-detection blocker; `.rs`/`.py`/`.sh`/`.yaml`/`.yml`/`.c` comment-prefix selection now under permanent coverage (issue #774 pinned). |
| **Track B (5)** | `ad52e6f` | +553 | `selection` (17 theorems) + `save_state` (4 theorems) + `is_modified()` test_api observable. Issue #191 (undo-back-to-save-point) pinned. Found a 5th asymmetry: ExpandSelection on punctuation-then-word runs picks "**-" in e2e but "**-word" in the semantic harness — pinned. |
| **Track B (6)** | (next commit) | +~330 | `unicode_cursor` (12 theorems): UTF-8/grapheme invariants for arrow movement, backspace (Norwegian, emoji, Thai layer-by-layer), DeleteForward (atomic Thai cluster), selection-delete and selection-replace over multibyte ranges. |

**Final test count:** 134 passing, 0 ignored, 0 failing. Every
formerly-`#[ignore]`d property and regression repro is now permanent
coverage.

**Migrated tests by source file** (~80 e2e tests subsumed):
case_conversion (1), sort_lines (3), indent_dedent (3),
select_to_paragraph (2), smart_home (2), duplicate_line (5),
emacs_actions (8), undo_redo (4), auto_pairs (20), toggle_comment (10),
selection (17), save_state (4), unicode_cursor (12).

Each theorem mathematically pins down behavior the imperative
originals were silent or vague about — selection clearing after
SortLines, exact byte ranges of select-to-paragraph, the
Undo/Redo cursor asymmetry, the Emacs C-o intent gap, etc.

**Two latent production bugs found by the property tests, now fixed in `d95b9d1`:**

1. `actions.rs:1613` — smart-dedent: `prefix_bytes[prefix_len - 1]`
   panicked when `cursor.position` was stale (past buffer end) so
   `slice_bytes(line_start..cursor.position)` returned fewer bytes
   than `prefix_len` implied. **Fix:** use `prefix_bytes.last()` and
   guard on `!prefix_bytes.is_empty()`.
2. `state.rs:462` — `DeleteBackward`'s newline-counter:
   `deleted_text[..bytes_before_cursor]` panicked when `range.len()`
   exceeded `deleted_text.len()` (stale range end past buffer).
   **Fix:** also clamp with `.min(deleted_text.len())`.

Both were the same family — cursor position out of sync with buffer
state after a chain of selection-replace + deletion actions on
whitespace-only content. Option (b) from the original recommendation
(fix the call sites) was chosen over option (a) (force layout
reconciliation between actions in `handle_execute_actions`) because
the call-site fixes are local, defensive, and don't slow down the
batch dispatch path.

**Reachability that motivated the fix:**

| Path | Renders / reconciles between actions? | Crashed before fix? |
|---|---|---|
| Interactive keystrokes (verified in tmux on the release binary) | yes (per keystroke) | no |
| Macro replay (`play_macro` calls `recompute_layout` per action) | yes | no |
| Property test `dispatch_seq` (no render between) | no | **yes** |
| `handle_execute_actions` in `app/plugin_dispatch.rs` (vi-mode count prefixes like `3dw`, plugin-driven action batches) | **no** | **yes** |

The bugs were *latent* in the sense that the only production path
that reached them was the plugin/vi-count-prefix dispatch loop. The
property tests' `evaluate_actions` mimics that path exactly — no
render reconciliation between actions — which is what made them
findable. The `play_macro` implementation in `app/macro_actions.rs`
already calls `recompute_layout` between every action specifically
to avoid this class of bug; that load-bearing comment was the
first hint that the underlying invariant violation was real and
known to be fragile.

`diagnosis_bug{1,2}_does_not_panic_with_render_between` in
`tests/semantic/regressions.rs` confirmed the diagnosis: the same
shrunk repros pass cleanly when `harness.render()` is called between
every action. They remain in the suite as permanent guards against
regression of the layout-reconciliation invariant.

This validates the framework's premise: declarative theorem testing
with a typed-failure external driver is materially better at finding
bugs than imperative E2E. The two bugs found here are the kind that
*cannot* be reached by typing on the keyboard — they live behind
plugin and vi-mode dispatch — so even an exhaustive imperative suite
that drives every keystroke would never have surfaced them.

**Framework extensions landed alongside migrations:**

| Extension | Commit | Unblocked |
|---|---|---|
| `BehaviorFlags { auto_close, auto_indent, auto_surround }` + `assert_buffer_theorem_with_behavior` | `e0b2a43` | auto-pair / smart-editing tests that need production auto-* defaults |
| `load_buffer_from_text_named(filename, content)` + `_with_file` / `_with_behavior_and_file` runners | `79c648e` | toggle_comment language-detection tests; quote auto-pair (suppressed in `language="text"`) |
| `EditorTestApi::is_modified() -> bool` | `ad52e6f` | save-point / dirty-state undo tests (issue #191) |

**Behavioral findings — three fixed bugs and two pinned asymmetries:**

| # | Source | Finding | Status |
|---|---|---|---|
| 1 | proptest fuzzing | `actions.rs:1613` smart-dedent OOB on stale cursor | Fixed in `d95b9d1` |
| 2 | proptest fuzzing | `state.rs:462` delete-backward newline counter OOB | Fixed in `d95b9d1` |
| 3 | `emacs_actions` migration | `Action::OpenLine` advances cursor instead of staying put (Emacs C-o intent) | Fixed in `6b2f144` |
| 4 | `undo_redo` migration | Undo restores cursor to pre-write position, but Redo does *not* re-advance the cursor past re-inserted bytes | Pinned (asymmetry recorded as theorem) |
| 5 | `selection` migration | ExpandSelection on a punctuation-then-word run selects `"**-"` in e2e but `"**-word"` in the semantic harness — likely a `word_characters` resolution gap | Pinned (semantic-harness behavior recorded) |

**Still deferred (per the original plan):** the full `RenderSnapshot`
design from §9.1 and the issue-#1147-style Class B rewrites that
depend on it. Smaller remaining batches without framework needs:
`multicursor.rs` edge cases, `vi_mode.rs` action subset, the
`test_select_word_accented_characters` proptest-style migration.
Each should land alongside the first theorem that demonstrably
needs the corresponding extension.

**Superseded by Part II.** The "deferred" set is no longer the
boundary of the migration. Part II (§11–§19) lays out the
twelve scenario types needed to represent every e2e file in the
suite — including rendering, LSP, filesystem, mouse, and
animations — and the implementation order for getting there. The
text immediately above is preserved as the Part-I plan it was
written under.

---

## 1. Motivation

The current E2E suite drives the editor through a virtual `crossterm`
keyboard, a `ratatui` `TestBackend`, and explicit `harness.render()` cycles.
A typical test reads:

```rust
harness.send_key(KeyCode::Home, KeyModifiers::NONE).unwrap();
for _ in 0..5 {
    harness.send_key(KeyCode::Right, KeyModifiers::SHIFT).unwrap();
}
harness.render().unwrap();
harness.send_key(KeyCode::Char('u'), KeyModifiers::ALT).unwrap();
harness.render().unwrap();
assert_eq!(harness.get_buffer_content().unwrap(), "HELLO world");
```

This shape has three structural problems:

1. **Coupling to physical keys.** `KeyCode::Char('u') + ALT` is a property of
   the default keymap, not of the case-conversion feature. Tests break when
   shortcuts move.
2. **Coupling to the render loop.** Every state mutation needs a `render()`
   to "settle" output, slowing the suite and obscuring intent.
3. **Coupling to UI screen-scraping.** Many assertions read characters out
   of the `ratatui::Buffer`, conflating logic bugs with rendering bugs.

The proposed style replaces the trio (`send_key`, `render`, screen-scrape)
with a pure data structure (a *Theorem*) declaring `(initial state, action
sequence, expected final state)`, evaluated by a runner that touches no
terminal.

## 2. Existing Seams (production already exposes most of what we need)

A reconnaissance pass on the editor crate found we do **not** need to refactor
production:

| Need | Existing API | Where |
|---|---|---|
| Semantic alphabet | `pub enum Action` | `src/input/keybindings.rs:305` |
| Apply one action headlessly | `Editor::dispatch_action_for_tests(action)` | `src/app/editor_init.rs:1327` (`#[doc(hidden)]`, already `pub`) |
| Read buffer text | `editor.active_state().buffer.to_string()` | `src/app/mod.rs:1265` |
| Read cursors | `editor.active_cursors()` returning `&Cursors` | `src/app/mod.rs:1277` |
| Read viewport | `editor.active_viewport()` | `src/app/mod.rs:1294` |

`Action` already covers the cases the inspiration sketch's `BufferAction`
covers — `MoveLeft`, `SelectRight`, `ToUpperCase`, `Undo`, `Redo`,
`AddCursorNextMatch`, etc. — and is already a serializable data enum
(`Debug, Clone, PartialEq, Eq, Serialize, Deserialize`). It is the
"alphabet" of the system.

`dispatch_action_for_tests` routes through the same `handle_action` path
the production input layer uses, so semantic-level coverage is identical
to keystroke-level coverage for actions that don't depend on modal UI
state (popups, menus, prompts).

**Conclusion:** Phase 1's "minimal API exposure" is mostly *re-export*
work, not new surface — *but the seams must be wrapped*; see §2.1.

### 2.1 Tests bind to a named test API, not arbitrary internals

A non-negotiable design principle: theorem tests **never** reach into
`editor.active_state()`, `editor.active_cursors()`, or
`editor.active_viewport()` directly. Those accessors are production
internals; if the test suite depends on their exact shape, refactoring
them becomes a cross-cutting churn (this is half of why the current
harness is sticky).

Instead, all observation flows through one explicit, versioned, named
surface:

```rust
// Test-only module on the editor. ~100 LOC. Zero behavior, all reads.
//
// crates/fresh-editor/src/test_api.rs   (or `app/test_api.rs`)
#[doc(hidden)]
#[cfg(any(test, feature = "test-api"))]
pub mod test_api {
    use crate::input::keybindings::Action;

    /// The single entry point for test-driven mutation.
    pub trait EditorTestApi {
        // ── Drive ────────────────────────────────────────────────
        fn dispatch(&mut self, action: Action);
        fn dispatch_seq(&mut self, actions: &[Action]);

        // ── Class A: pure state observables ──────────────────────
        fn buffer_text(&self) -> String;
        fn primary_caret(&self) -> Caret;
        fn carets(&self) -> Vec<Caret>;
        fn selection_text(&self) -> String;

        // ── Class B: layout observables (Phase 3) ────────────────
        fn render_snapshot(&mut self) -> RenderSnapshot;

        // ── Class C: styled observables (Phase 3+) ───────────────
        fn styled_frame(&mut self, theme: &Theme) -> StyledFrame;
    }

    /// Small projection over Cursor: only the fields tests assert on.
    /// Hides sticky_column, deselect_on_move, block_anchor unless the
    /// test explicitly asks (variant constructor).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Caret {
        pub position: usize,
        pub anchor:   Option<usize>,
    }

    pub struct RenderSnapshot { /* §9.1 */ }
    pub struct StyledFrame    { /* §9.5 */ }
    pub struct Theme          { /* opaque handle */ }
}
```

`Editor` implements `EditorTestApi`. **No other accessors are reachable
from theorem tests.** Concretely:

- `tests/semantic/**` may `use fresh::test_api::*;` and nothing else
  from the editor.
- `tests/semantic/**` may **not** `use fresh::app::Editor`,
  `fresh::model::cursor::Cursor`, `fresh::model::buffer::Buffer`, or
  `fresh::view::viewport::Viewport`.
- The runner type (`assert_buffer_theorem`) holds an
  `&mut dyn EditorTestApi`, not an `&mut Editor`.

This buys us four things:

1. **Refactor freedom.** Internal renames (`active_state` → `state_for`
   …) don't touch a single test.
2. **Explicit observation contract.** A reviewer reading `test_api.rs`
   sees the entire dependency surface in one file.
3. **Forces "what does a test need to see?" to be a design question.**
   If a theorem can't be expressed against `EditorTestApi`, the right
   reflex is to *propose adding an observable*, not to bypass the API.
4. **Caps the migration's reverse-coupling.** Production code can never
   accidentally depend on something tests rely on, because the test
   API is one-directional.

A minimal Phase 2 only exposes `dispatch` + `buffer_text` +
`primary_caret` + `carets` + `selection_text`. That is sufficient for
the case-conversion PoC. Everything else (`RenderSnapshot`,
`StyledFrame`, modal observables) is added *only when the next theorem
type lands*, with a code review check that the new entry is the
smallest sufficient observable.

The runner sketch in §5.2 is updated accordingly: it does **not** read
from `h.editor()`; it reads from `h.test_api()` (or, if the harness is
itself made to implement `EditorTestApi`, from `h` directly).

## 3. The One Real Caveat: Viewport Scroll Depends On Rendering

Several E2E tests (notably `issue_1147_wrapped_line_nav`,
`scroll_*`, `line_wrap_scroll_bugs`) assert on `viewport.top_byte`.
That field is only reconciled by `Viewport::ensure_visible_in_layout`
(`src/view/viewport.rs:993`), which consumes `ViewLine`s computed by the
render pipeline. Without a render, `top_byte` does not move when the
cursor moves.

This means **two classes of tests** exist, and only one is fully
"renderless":

- **Class A — pure state.** Buffer text, cursor positions, selection,
  undo/redo, multi-cursor layout, indent/dedent, case conversion,
  duplicate-line, sort-lines, smart-home (text-only assertions),
  toggle-comment. ≈ 60 % of the suite by file count.

- **Class B — viewport / layout.** Anything asserting on `top_byte`,
  `top_line_number`, scrollbar geometry, screen cursor `(x, y)`, visible
  rows, virtual-line positioning. Needs the layout pipeline.

The migration handles each class differently (§5). Class A is the PoC.

## 4. Target Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Theorem<Domain>                                            │
│  ─────────────────                                          │
│  description:    &'static str                               │
│  initial:        Domain::State                              │
│  actions:        Vec<Domain::Cmd>                           │
│  expected:       Domain::Expectation                        │
└─────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  fn assert_theorem<D: Domain>(t: Theorem<D>)                │
│   1. instantiate Editor offscreen (no terminal draws)       │
│   2. seed Domain::State into Editor                         │
│   3. for each cmd: editor.dispatch_action_for_tests(cmd)    │
│   4. read Domain::Observable out of editor                  │
│   5. assert observable == expected                          │
└─────────────────────────────────────────────────────────────┘
```

A `Domain` trait describes *what kind of test this is*: pure-buffer,
viewport, multi-cursor, undo-trace, etc. Each domain carries its own
`State`, `Cmd` (an alias for `Action` or a subset), `Expectation`, and
projection function. This avoids one bloated `Theorem` struct that
accumulates `Option<…>` fields for every possible assertion.

### 4.1 Why we don't introduce a parallel `HeadlessEditorCore`

The inspiration sketch shows a separate `HeadlessEditorCore` that
re-implements buffer, cursor, and wrap math. Building a second core would
*add* a verification problem (does the core agree with the editor?)
rather than remove one. Since `Editor` already runs headlessly when fed
through `dispatch_action_for_tests` (no terminal IO occurs unless
`terminal.draw` is called), the cheaper move is to **reuse the editor as
the denotation**. The runner just skips the rendering step.

### 4.2 Production code changes (Phase 1)

Strictly additive, gated behind an existing `#[doc(hidden)]` test API.

1. **No new types.** Use `Action` as the command alphabet.
2. **One new accessor** if needed, on `Editor`:
   ```rust
   #[doc(hidden)]
   pub fn dispatch_action_sequence_for_tests(&mut self, actions: &[Action]) {
       for a in actions { let _ = self.handle_action(a.clone()); }
       let _ = self.process_async_messages();
   }
   ```
   This is purely a convenience over a loop and can be skipped if the
   runner calls `dispatch_action_for_tests` itself.
3. **Cursor projection helper** (test-only) so theorems can express
   expected cursors without depending on the in-tree `Cursor` struct's
   default fields:
   ```rust
   #[doc(hidden)]
   pub fn cursor_snapshot_for_tests(&self) -> Vec<CursorSnapshot> { … }
   ```
   This *is* new surface — but only ~30 lines, marked test-only,
   and never reachable from `main.rs`.

That's the entire production diff for Phase 1: roughly two `#[doc(hidden)]`
methods. The terminal application path is untouched.

## 5. The PoC (Phase 2)

### 5.1 Test selected for the PoC

`tests/e2e/case_conversion.rs::test_to_uppercase` (lines 5–39).

**Why this one:**

- Pure Class A: only buffer text and cursor are observed.
- Touches three independent concerns the architecture must cover:
  cursor movement (`MoveLineStart`), selection extension (`SelectRight`
  ×5), and a transformation (`ToUpperCase`).
- Has an obvious "lift to repetition" opportunity: `for _ in 0..5
  { send_key(Right, SHIFT) }` becomes `Repeat(SelectRight, 5)`.
- Failure modes are unambiguous (text + selection range), so a green
  test is convincing evidence the architecture works.

The current test (38 lines, four `harness.render()` calls) becomes:

```rust
#[test]
fn theorem_to_uppercase_selection() {
    assert_buffer_theorem(BufferTheorem {
        description:      "Alt+U uppercases the selected range and leaves selection intact",
        initial_text:     "hello world",
        initial_cursor:   Caret::EndOfBuffer,
        actions:          vec![
            Action::MoveLineStart,
            Repeat(Action::SelectRight, 5),
            Action::ToUpperCase,
        ],
        expected_text:    "HELLO world",
        expected_primary: CursorExpect::range(0, 5),
        expected_extra_cursors: vec![],
    });
}
```

No `KeyCode`. No `render()`. No `harness.get_selected_text()` round-trip.

### 5.2 PoC code structure

```
crates/fresh-editor/tests/
├── common/
│   └── theorem/
│       ├── mod.rs              ← Domain trait + Theorem<D> + Repeat helper
│       ├── buffer_theorem.rs   ← Class A: text + cursor assertions
│       └── runner.rs           ← assert_buffer_theorem
└── e2e_theorems/
    └── case_conversion.rs      ← rewritten test (PoC)
```

`buffer_theorem.rs` (sketch, ~80 LOC) — **only imports from
`fresh::test_api`, never from `fresh::app` / `fresh::model` /
`fresh::view`:**

```rust
use fresh::test_api::{EditorTestApi, Caret, Action};
use crate::common::harness::EditorTestHarness; // hosts the headless Editor

#[derive(Clone)]
pub enum InitialCaret {
    StartOfBuffer,
    EndOfBuffer,
    Byte(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CursorExpect {
    pub position: usize,
    pub anchor: Option<usize>,
}

impl CursorExpect {
    pub fn at(p: usize) -> Self { Self { position: p, anchor: None } }
    pub fn range(anchor: usize, position: usize) -> Self {
        Self { position, anchor: Some(anchor) }
    }
}

pub struct BufferTheorem {
    pub description:            &'static str,
    pub initial_text:            &'static str,
    pub initial_cursor:          InitialCaret,
    pub actions:                 Vec<Action>,
    pub expected_text:           &'static str,
    pub expected_primary:        CursorExpect,
    pub expected_extra_cursors:  Vec<CursorExpect>,
}

pub fn assert_buffer_theorem(t: BufferTheorem) {
    let mut h = EditorTestHarness::new(80, 24).unwrap();
    let _fix = h.load_buffer_from_text(t.initial_text).unwrap();

    // EditorTestHarness exposes &mut dyn EditorTestApi via h.api_mut().
    let api: &mut dyn EditorTestApi = h.api_mut();

    seed_initial_caret(api, t.initial_cursor, t.initial_text.len());
    api.dispatch_seq(&t.actions);
    // No render() call. Ever.

    assert_eq!(api.buffer_text(), t.expected_text,
               "buffer text mismatch in: {}", t.description);

    let primary = api.primary_caret();
    assert_eq!(primary, t.expected_primary.into(), "primary caret in: {}", t.description);

    let extras: Vec<_> = api.carets().into_iter().skip(1).collect();
    assert_eq!(extras.len(), t.expected_extra_cursors.len(), …);
    for (got, want) in extras.iter().zip(&t.expected_extra_cursors) {
        assert_eq!(*got, want.clone().into(), …);
    }
}
```

The harness change is small — add one method:

```rust
// crates/fresh-editor/tests/common/harness.rs
impl EditorTestHarness {
    pub fn api_mut(&mut self) -> &mut dyn fresh::test_api::EditorTestApi {
        &mut self.editor   // because Editor: EditorTestApi
    }
}
```

That's the *only* surface the new test directory needs.

`Repeat` is a tiny helper, not a new variant on `Action`:

```rust
pub fn Repeat(a: Action, n: usize) -> impl Iterator<Item = Action> { … }

// usage:
actions.extend(Repeat(Action::SelectRight, 5));
```

We *could* push `Repeat` into the production enum, but that adds a
variant to a 600-case enum that production code doesn't need. Keeping
it in the test layer is cheaper and more honest.

### 5.3 What the PoC proves

- `Action` + `dispatch_action_for_tests` is sufficient to express a real
  bug-class test (case conversion is in production, has a bug history).
- Tests run faster: zero `terminal.draw` cycles, no `process_async_messages`
  per keystroke, no shadow-string mirroring.
- The test is keymap-agnostic: changing the Alt+U binding doesn't break it.
- Test reads as a *specification* of the feature, not a transcript of a
  user session.

### 5.4 What the PoC does **not** address (intentionally)

- Viewport / scroll assertions (Class B). Held back to Phase 3.
- Modal-UI tests (command palette, file open prompt, settings tree).
  These need an additional vocabulary item (e.g.,
  `OpenCommandPalette / FilterTo("duplicate line") / ConfirmSelection`).
- Plugin-driven actions (these route through async dispatch and may need
  one extra `process_async_messages()` call inside the runner).
- LSP and filesystem-dependent tests (need fakes; orthogonal to the
  semantic-test idea).

## 6. Phase 3 — Expansion plan

After the PoC merges and is reviewed, expansion proceeds **per
domain**, each adding *one* new theorem type alongside `BufferTheorem`:

| Domain | New theorem | Approx. tests covered | New API surface |
|---|---|---|---|
| Cursor & text mutation | `BufferTheorem` (PoC) | ~80 | 0 |
| Multi-cursor | `MultiCursorTheorem` | ~25 | 0 |
| Undo / redo trace | `TraceIsomorphismTheorem` (forward + `undo_all`) | ~15 | 0 |
| Modal popups | `ModalTheorem` (`Open`, `Filter`, `Confirm`) | ~30 | 1 helper for prompt state |
| Viewport / scroll | `LayoutTheorem` (single explicit `render_for_layout()` call inside the runner; everything else still declarative) | ~40 | 0 |
| Theme projection | `ProjectionTheorem<State, View>` (pure function over `Theme + State`) | ~10 | 1 pure projection function per UI surface |

For Class B (viewport), the runner *does* invoke a layout pass — but
crucially **once at the end**, not after every action. This is the
minimal bridge that lets us keep declarative tests while honoring the
fact that scroll is layout-dependent. The runner shape:

```rust
pub fn assert_layout_theorem(t: LayoutTheorem) {
    let mut h = EditorTestHarness::new(t.width, t.height).unwrap();
    let _fix = h.load_buffer_from_text(t.initial_text).unwrap();
    for a in t.actions { h.editor_mut().dispatch_action_for_tests(a); }
    h.render().unwrap(); // single, terminal-side-effect-free layout pass
    assert_eq!(h.top_byte(), t.expected_top_byte, …);
}
```

Migration is **incremental and reversible**: old `EditorTestHarness`
tests continue to compile and run side by side with new theorem tests.
A test is migrated when:

1. It fits a domain.
2. Its domain has a runner.
3. The author judges the rewrite reads more clearly than the original.

Tests that don't migrate (e.g., genuinely visual regression tests, GUI
mouse-drag flows) stay imperative — that's fine. The goal is **not**
100 % migration; the goal is to remove keymap and render coupling
*where they aren't actually being tested*.

## 7. Tension with `CONTRIBUTING.md` rule #2 — and how to resolve it

`CONTRIBUTING.md` currently states:

> **E2E Tests Observe, Not Inspect**: Any new user flow must include an
> end-to-end test that drives keyboard/mouse events and asserts only on
> rendered output. Do not call accessors that return model, view, or
> context state — if an invariant isn't visible on screen, cover it with
> a unit test on the component.

The theorem-style tests this design proposes are, by that definition,
**unit tests on the editor component**, not E2E tests. The rule was
written to prevent two real failure modes:

1. **False-green drift.** An "E2E" test that pokes internal state and
   never renders can pass while the user-visible output is broken.
2. **Bug class blindness.** Cursor blink, selection highlight, scrollbar
   geometry, theme contrast, gutter alignment — none of these surface
   in `editor.active_state()` and would never be caught by state-only
   assertions.

The migration must **not** weaken this protection. The resolution:

1. **Rename, don't reclassify.** Theorem tests live under
   `tests/semantic/` (or `tests/component/`), not `tests/e2e/`. They
   are explicitly *component-level* tests on `Editor` as a state
   machine. The directory name is the contract: a reader knows
   immediately what guarantees the test does and does not provide.
2. **The `tests/e2e/` directory keeps its current rule.** Anything in
   `tests/e2e/` continues to drive keys/mouse and assert on rendered
   output. We do not migrate `tests/e2e/` files into theorems by
   *moving* them; we *add* a semantic-test sibling and, only when the
   semantic test fully covers the bug, optionally retire the E2E one
   case-by-case during review. Most E2E tests will stay.
3. **Update the contributing rule** to make the categories explicit:
   ```
   E2E (tests/e2e/): drive input, assert on rendered output. Required
       for any new user flow. Cover GUI/render/keymap concerns.
   Semantic (tests/semantic/): apply Action sequences, assert on
       Editor state. Required for any new editor-logic invariant
       that is not visible on screen, *and* allowed as a faster
       redundant proof for bugs already covered by an E2E.
   Property/shadow (tests/property_*, tests/shadow_*): unchanged.
   ```

This keeps the *intent* of rule #2 — "if it isn't on screen, you didn't
test the user-facing thing" — while letting us put logic-only
invariants (case conversion preserves selection range; multi-cursor
undo is atomic; smart-home toggles between two specific byte offsets)
in a faster, clearer harness. **Whenever a bug has both a logic and a
visual symptom, both tests are required.**

The PoC in §5 should be revised accordingly: the new test goes under
`tests/semantic/case_conversion.rs`, *not* `tests/e2e_theorems/`. The
existing `tests/e2e/case_conversion.rs::test_to_uppercase` stays.

## 8. Testing rendering issues

The migration is explicitly **not** a strategy for testing rendering.
Anything that can break in the renderer — color, contrast, glyph
choice, cursor visibility, gutter width, scrollbar position, line-wrap
indent, syntax highlighting — needs a test that actually runs the
render pipeline. Theorem tests cover state. Below is how each render
concern stays covered:

### 8.1 Existing render-side coverage (kept as-is)

| Concern | Existing harness |
|---|---|
| Frame contents (cell-level) | `harness.render() + harness.buffer()` / `screen_to_string()` |
| ANSI escape correctness | `harness.render_real()` / `render_real_incremental()` (vt100 parser) |
| Visual regression (themes, snapshots) | `tests/common/visual_testing.rs`, `tests/common/snapshots/` |
| Hardware cursor show/hide | `harness.render_observing_cursor()` |
| Multi-cursor secondary cursor styling | `harness.find_all_cursors()` |
| Theme screenshots | `tests/e2e/theme_screenshots.rs` |

None of these change. Theorem-style tests *cannot* replace them and
shouldn't try.

### 8.2 Three new render-test patterns the migration *adds*

Once Class A theorems exist, three render-targeted patterns become
cheap to write and should be standard practice:

#### A. Pure projection theorems (`ProjectionTheorem<S, V>`)

For UI surfaces that are pure functions of state — settings tree
nodes, status-bar segments, tab labels, gutter cells, diff-hunk
markers — extract the projection function:

```rust
fn project_settings_node(node: &SettingsNode, theme: &Theme) -> CellStyle { … }
```

…and test it in isolation. This is the
`theorem_settings_label_projection` pattern from the inspiration
sketch. It catches *render bugs* (foreground == background, cursor
visible while not editing, wrong fg in selection) without driving keys
*or* running the full layout. The test runs in microseconds.

The pre-condition is that the projection function exists as a pure
function in production. Some surfaces don't yet — extracting them is a
small, additive refactor. **No production refactor is on the critical
path for Phase 2;** projection theorems are a Phase 3+ pattern.

#### B. Layout theorems (`LayoutTheorem`)

Apply the action sequence headlessly, then run **one** layout pass and
assert on layout-level observables (`top_byte`, `top_view_line_offset`,
visible row → byte mapping, soft-wrap row count). This is described in
§6 (`assert_layout_theorem`) and is the right shape for issues like
#1147 (viewport scrolls when it shouldn't).

What this catches that pure state can't: incorrect viewport
reconciliation, wrap-row miscounts, gutter-width drift.

What it doesn't catch: anything below the layout layer (color,
attributes, glyph rendering). Those need pattern (C).

#### C. Render-diff theorems (`RenderTheorem`)

For visual bugs that survive the layout (e.g., scrollbar in the wrong
column, overlay color confusion, off-by-one row), apply the action
sequence, render once, and assert on a small, *named* slice of the
buffer:

```rust
RenderTheorem {
    description: "Scrollbar uses theme.scrollbar.fg, not theme.text.fg (issue #1554)",
    initial_text: …,
    actions: vec![Action::MovePageDown],
    width: 80, height: 24,
    inspect: Inspect::ColumnFg { col: 79, row_range: 2..22 },
    expected: ExpectedFg::All(theme.scrollbar.fg),
}
```

The runner takes the `Inspect` enum and pulls out exactly the cells
that matter, comparing them to the expected color/symbol/modifier.
This is the "snapshot test, but tightly scoped" shape: easier to read
than a full screen diff, less brittle than asserting on a substring of
`screen_to_string()`.

`Inspect` variants would start small:
`Cell { x, y }`, `Row { y }`, `Column { x }`, `Region(Rect)`,
`HardwareCursor`. The runner returns a typed result so the assertion
reads as data, not as ad-hoc string parsing.

This is the smallest delta from current screen-scraping practice — the
test still calls `terminal.draw` once — but moves the assertion from
"does this substring appear somewhere?" to "what fg does cell (79, 5)
have?". That precision is what catches theme regressions.

### 8.3 What stays imperative, forever

Some tests will never become declarative without losing their value:

- **Visual regression / golden-image tests.** A bug like "the gutter
  glyph for a fold marker shifted by one column" is best detected by a
  byte-for-byte comparison of a saved screenshot. These already exist
  in `docs/visual-regression/` and `tests/common/visual_testing.rs`.
- **Animations.** Cursor blink, scroll smoothing, fade-out highlights —
  the test's subject *is* the temporal evolution of the rendered
  buffer.
- **GUI-mode tests.** `tests/e2e/gui.rs` exercises the
  `winit/wgpu` layer; theorem tests can't reach it.
- **Crossterm / terminal-emulator integration.** ANSI escape
  generation, focus events, OSC 52, mouse-encoding. These are
  *rendering*, by definition.

Migration plans must enumerate these and **not** convert them.

## 9. The middle layer — view model between state and pixels

Both the §3 caveat (viewport scroll only settles during render) and the
§8 render-test patterns point to the same gap: there is no named,
stable, *observable* layer between `EditorState` and the styled
`ratatui::Buffer`. Today's pipeline collapses several conceptual stages
into a single `render()` call:

```
EditorState  ──[ layout ]──▶  ViewLine[]  ──[ style + glyph ]──▶  ratatui::Buffer
   (data)       (mostly                       (theme +                (cells
                pure)                          symbols)                with
                                                                       fg/bg)
```

`ViewLine` exists internally but is not a publicly testable artifact,
and crucially it doesn't include cross-cutting things tests care about
(scrollbar thumb position, hardware cursor row/col, popup placement,
fold-indicator column). Tests therefore choose between two unappealing
extremes: state-only (misses display bugs) or buffer-cell scraping
(brittle, theme-coupled, breaks when the gutter layout changes).

A **named view-model layer** would be the right test target for a large
subset of "looks wrong" bugs. Below are four candidate shapes,
discussed in increasing scope.

### 9.1 Option A — `RenderSnapshot` (smallest, most pragmatic)

A typed struct produced by the *layout* phase, before any colors or
glyphs. Roughly:

```rust
pub struct RenderSnapshot {
    pub width: u16,
    pub height: u16,
    pub viewport: ViewportSnapshot,           // top_byte, top_view_line_offset, scroll thumb
    pub gutter:   GutterSnapshot,             // per-row { line_number, fold_marker, diagnostic }
    pub rows:     Vec<RowSnapshot>,           // per visible content row, semantic segments
    pub hw_cursor: Option<(u16, u16)>,        // screen cell of the primary cursor
    pub secondary_cursors: Vec<(u16, u16)>,
    pub decorations: Vec<DecorationSnapshot>, // diagnostics, search highlights, multi-cursor etc.
    pub popups:   Vec<PopupSnapshot>,         // {kind, area, content_lines}
    pub status:   StatusBarSnapshot,
    pub tabs:     TabBarSnapshot,
}

pub struct RowSnapshot {
    pub view_line: usize,                     // index into the buffer's ViewLine sequence
    pub source_byte_range: Option<Range<usize>>,
    pub kind: RowKind,                        // Source | WrappedContinuation | Virtual { plugin }
    pub segments: Vec<Segment>,
}

pub enum Segment {
    Text { byte_range: Range<usize>, role: TextRole },  // role = Normal | Selection | Match | Inactive
    Whitespace { kind: WsKind },
    Tab { stops: u16 },
    WrapMarker,
    Conceal { replacement: String },
}
```

Crucially: **no colors, no theme.** Roles are semantic
(`TextRole::Selection`), not pigment.

Tests target a `RenderSnapshot` to assert claims like:

- "After PageDown, row 0 shows view_line 24, not 25." (issue #1147)
- "The fold marker for line 12 is on screen row 7, column 4."
- "After Ctrl+End, hw_cursor is on the last source byte's row."
- "The scrollbar thumb covers rows 18..22 of the content area."
- "Search match decoration covers the byte range 142..147 on row 9."

Theme regressions are *out of scope* for `RenderSnapshot`-targeted
tests; they get a separate styling-layer test (§9.5).

**Cost:** A new pass `EditorState → RenderSnapshot` already exists in
spirit inside the renderer; the work is to factor it out cleanly. ~300
LOC of additive code, plus a `pub fn snapshot_for_tests(&mut self) ->
RenderSnapshot` accessor on `Editor`.

**Win:** The 40-ish viewport/scroll tests (Class B) become declarative.
The render-diff theorem pattern from §8.2 (C) targets `RenderSnapshot`
instead of `ratatui::Buffer`, gaining theme-independence.

### 9.2 Option B — Per-surface view models

Instead of one big `RenderSnapshot`, expose one view model per
top-level UI surface, behind small traits:

```rust
trait HasViewModel { type Vm; fn view_model(&self) -> Self::Vm; }

impl HasViewModel for TabBar    { type Vm = TabBarVm;    … }
impl HasViewModel for StatusBar { type Vm = StatusBarVm; … }
impl HasViewModel for Gutter    { type Vm = GutterVm;    … }
impl HasViewModel for SettingsPanel { type Vm = SettingsTreeVm; … }
```

Each surface tests *its own* view model in isolation:

```rust
let vm = harness.editor().tab_bar_vm();
assert_eq!(vm.tabs[1].title, "main.rs");
assert_eq!(vm.tabs[1].state, TabVmState::ModifiedActive);
```

**Pro vs. Option A:** Surface-local. Adding a new surface doesn't
require extending one mega-struct. Closer to how the code is already
organized (per-widget modules under `view/ui/`).

**Con:** Cross-surface invariants (e.g. "popup area doesn't overlap
status bar") need a coordinator step on top. Test discoverability is
worse — there's no single "what does the screen show?" object.

### 9.3 Option C — `EditorView` as algebraic data type

Treat the entire UI as an immutable ADT computed from state:

```rust
pub enum EditorView {
    Buffer { tabs: TabBarVm, body: BufferBodyVm, status: StatusBarVm, popups: Vec<PopupVm> },
    Settings(SettingsVm),
    FileBrowser(FileBrowserVm),
    Splash(SplashVm),
}
```

This is the most denotational of the four — closest to "the screen *is*
a function of state" — and makes mode transitions explicit (you can't
have a settings tree visible while in `EditorView::Buffer`, by
construction).

**Pro:** Forces every UI mode to be reified, which would catch real
bugs (e.g., the "popup is open but its input handler is dead"
race-condition class).

**Con:** Big upfront refactor; touches every UI module. Not
incrementally adoptable. The migration would block on it.

### 9.4 Option D — Semantic cell-grid (style-free buffer)

A `ratatui::Buffer`-shaped grid where cells carry **roles**, not
colors:

```rust
pub struct SemanticCell {
    pub symbol: String,
    pub role: CellRole,             // Normal | Selection | Cursor | LineNumber | …
    pub tags:  TagSet,              // {InMatchHighlight, InFold, OnVirtualLine}
}
```

Render is then `(SemanticCell grid + Theme) → styled cells`. Tests on
the grid catch "wrong cell got the cursor role" without coupling to
colors; tests on the styled cells catch theme bugs.

**Pro:** Drop-in for existing screen-scraping patterns. Minimal change
to assertion shape.

**Con:** Grid-shaped data is the *least* declarative form — you still
write `assert grid[12][4].role == Cursor` rather than `assert
hw_cursor == (4, 12)`. Better than today, worse than (A)/(B).

### 9.5 Recommendation

**Adopt Option A (`RenderSnapshot`) in Phase 3, after the Class A PoC
is merged.** Justification:

- It's the smallest unit of architectural change that unblocks the
  largest test category (Class B viewport/scroll).
- It composes with Option B: the snapshot's surface fields can be
  individual view models if a surface earns one.
- It does **not** require Option C's mode-ADT refactor and is
  forward-compatible with it (a future `EditorView` would *contain* a
  `RenderSnapshot`).
- It is theme-free, which protects against the existing screen-scraping
  brittleness the migration is trying to escape.

A typical Phase-3 layout-theorem then looks like:

```rust
LayoutTheorem {
    description: "Issue #1147: Up arrow at end-of-file does not scroll viewport",
    initial_text: ISSUE_1147_CONTENT,
    width: 80, height: 25,
    actions: vec![
        Action::MoveDocumentEnd,
        repeat(Action::MoveUp, 4),
    ],
    expect: |s: &RenderSnapshot| {
        assert_eq!(s.viewport.top_byte, ISSUE_1147_FINAL_TOP_BYTE);
        assert!(s.hw_cursor.unwrap().1 < (s.height - 4));
    },
}
```

The two theme-sensitive tests (`Bug #2: Foreground maps to selection_bg`
in the inspiration sketch) get a separate `StyleTheorem` that pairs a
`RenderSnapshot` with a `Theme` and asserts on the styled output:

```rust
StyleTheorem {
    snapshot: built_above,
    theme:    Theme::high_contrast(),
    expect:   |styled| assert_ne!(styled.cell(4, 12).fg, styled.cell(4, 12).bg),
}
```

This three-layer split — `State → RenderSnapshot → StyledFrame` —
gives every test a precise target:

| Test target | Layer | What it catches | What it can't catch |
|---|---|---|---|
| `BufferTheorem` | State | logic, cursor math, undo, multi-cursor | anything visual |
| `LayoutTheorem` | RenderSnapshot | viewport, gutter columns, popup placement, hw cursor row/col | colors, glyph choice, ANSI |
| `StyleTheorem` | StyledFrame | theme contrast, role-to-color mapping, modifier flags | terminal-emulator quirks |
| Existing E2E | Terminal | ANSI escape correctness, end-to-end user flow | (final backstop) |

The migration starts at the top and stops at the row that gives
diminishing returns. Phase 2 (PoC) only commits to the top row;
Phase 3 adds `RenderSnapshot` if and only if Class B tests prove the
investment is worth it.

## 10. Risks & non-goals

- **Risk: Action coverage gaps.** A test might exercise a path triggered
  only by a keymap (e.g., a chord that produces multiple events). Mitigation:
  if no `Action` exists, that's a finding — production should expose one,
  not the test should fall back to `KeyCode`.
- **Risk: Cursor-snapshot drift.** The `Cursor` struct has fields
  (`sticky_column`, `deselect_on_move`) that tests usually shouldn't care
  about. The `CursorExpect`/`CursorSnapshot` helpers project away these
  fields. We accept that two cursors with different `sticky_column`
  compare equal in a `BufferTheorem`; tests that care use
  `MultiCursorTheorem`.
- **Non-goal: rewriting the editor core.** No changes to buffer, cursor,
  viewport, or input dispatch.
- **Non-goal: replacing property tests.** `tests/property_*.rs` and
  `tests/shadow_model_*.rs` already operate at the model layer and stay
  as they are.
- **Non-goal: deleting `EditorTestHarness`.** It remains the host for
  the headless `Editor` instance and provides fixture loading,
  filesystem isolation, etc. The runner is a thin layer over it.

## 8. Acceptance criteria for Phase 2 (PoC)

Phase 2 is "done" when, on the migration branch:

- [ ] `crates/fresh-editor/tests/common/theorem/mod.rs` exists with
      `BufferTheorem` and `assert_buffer_theorem`.
- [ ] `crates/fresh-editor/tests/e2e_theorems/case_conversion.rs`
      contains a rewritten `theorem_to_uppercase_selection` test.
- [ ] The new test passes.
- [ ] The original `test_to_uppercase` is **left in place** and still
      passes (proof of additivity).
- [ ] Production diff is ≤ 150 LOC, all behind `#[doc(hidden)]` or
      `#[cfg(any(test, feature = "test-api"))]`. The diff is dominated
      by the `test_api` module from §2.1.
- [ ] No new dependency added to `Cargo.toml`.
- [ ] No `harness.render()` call in the new test.
- [ ] No `crossterm::KeyCode` import in the new test.
- [ ] **No `use fresh::app::…`, `use fresh::model::…`, or `use
      fresh::view::…` in `tests/semantic/**`.** Only `fresh::test_api`
      and the harness are reachable. CI lint or a tidy script enforces
      this.

## 9. Open questions for review

1. **Should `Repeat` be a real `Action` variant?** Pro: makes macros and
   plugin replay simpler. Con: introduces nesting into a thus-far flat
   enum, and `Action` is `Serialize` (so we'd need to think about JSON
   shape). *Recommendation: keep `Repeat` test-side for now.*
2. **Should the runner accept a `Vec<Action>` or `&[Action]`?** Owned
   `Vec` reads better in declarative theorems; `&[_]` allows reuse of
   action sequences across theorems. *Recommendation: take `Vec`,
   document `theorem.actions = base_actions.to_vec(); …` for sharing.*
3. **Should we add a `theorem!` macro?** Removes boilerplate but adds
   a layer of indirection. *Recommendation: defer until ≥ 5 theorems
   exist and the boilerplate is real.*
4. **Class B runner: render once, or expose `Editor::layout_for_tests()`?**
   The latter is more principled (no terminal at all) but requires
   factoring out the layout pass from the render pass. *Recommendation:
   render-once for Phase 3 to limit scope; consider extraction later if
   the test count justifies it.*

## 10. Appendix — file-level inventory

For triage in Phase 3. Counts approximate (`ls tests/e2e | wc -l = 224`,
some are non-test support files).

```
Pure buffer/cursor (Class A, ~80):
  basic.rs, case_conversion.rs, sort_lines.rs, smart_home.rs,
  duplicate_line.rs, indent_dedent.rs, toggle_comment.rs,
  triple_click.rs, select_to_paragraph.rs, undo_redo.rs,
  block_selection.rs, multicursor.rs, …

Viewport / scroll (Class B, ~40):
  issue_1147_wrapped_line_nav.rs, scroll_clearing.rs,
  scroll_wrapped_reach_last_line.rs, scrolling.rs,
  line_wrap_scroll_bugs.rs, search_center_on_scroll.rs,
  search_viewport_stall_after_wrap.rs, ctrl_end_wrapped.rs,
  horizontal_scrollbar.rs, scroll_sync, …

Modal UI (palette / settings / file picker, ~30):
  command_palette.rs, file_browser.rs, file_explorer.rs,
  settings_*.rs (multiple), keybinding_editor.rs, action_popup_global.rs

Plugin / LSP / filesystem (~40):
  language_features_e2e.rs, hot_exit_*.rs, slow_filesystem.rs,
  remote_*.rs, universal_lsp.rs, dabbrev_completion.rs

Visual / theme / rendering (kept imperative, ~30):
  theme_screenshots.rs, visual_regression.rs, theme.rs,
  cursor_style_rendering.rs, blog_showcases.rs

GUI / terminal-emulator / mouse-flow (kept imperative, ~10):
  gui.rs, terminal*.rs, tab_drag.rs, ansi_cursor.rs
```

The rough triage suggests ≈ 45 % of the suite (Class A + Class B) is
mechanically migratable to declarative theorems; another ~15 % (modal
UI) needs a small additional vocabulary; the rest stays imperative
because what they test *is* the rendering / GUI / external behavior.

(Part II below revises this triage to "every file is migratable" —
see §13 for the per-category mapping.)

---

# Part II — Realignment toward full coverage (2025-Q3+)

## §11. Goal realignment

The original plan framed the work as: "make tests declarative;
harvest the design wins (no keymap coupling, no render coupling, no
screen scraping); accept that ~30 % of the suite stays imperative
because rendering, GUI, LSP, and filesystem can't be reduced to pure
state."

The realigned plan keeps every Part-I win and adds one observation:
declarativity is the *means*. The *ends* are two specific kinds of
leverage that fall out for free once tests are data:

1. **Property-test leverage.** Every scenario doubles as a generator
   seed and a shrinking target. The corpus teaches `proptest` what
   action shapes are realistic; counterexamples shrink to a serializable
   `Vec<Action>` that becomes a permanent regression file with no
   source change.
2. **Shadow-model leverage.** A shadow model is one `step` function
   away from being a differential check against every scenario in
   the corpus. Adding a shadow doesn't require per-shadow
   scaffolding; the runner feeds each scenario through both editor
   and shadow and asserts equal observables.

Both wins are concrete day-one wins, not future possibilities. Track
A already showed (1) — uniform-noise `proptest` found two latent
panics in 70s. The bespoke shadow models in `tests/shadow_model_*.rs`
already exist but operate in their own loops with their own
observable contracts; (2) folds them into the same data path so a
new shadow is one trait-impl away.

The realignment changes one practical decision: **target every e2e
file**, including the categories the original plan deferred or
carved out, because each migration now produces three artifacts
(regression + proptest seed + shadow check) instead of one. The
ratio inverts the cost/benefit on Class B and on the
modal/LSP/FS/rendering categories.

A second observation, sharpened in §15: rendering does not have to
sit outside the framework. The pipeline `EditorState → RenderSnapshot
→ StyledFrame → AnsiStream` is four pure-ish layers; each layer has
a scenario type; tests target the *highest* layer they care about.
This subsumes today's split between `tests/e2e/` (renders) and
`tests/semantic/` (doesn't render) into a single layered family.

## §12. Per-test leverage

Why each migrated test is worth ≥ 3× a non-migrated one:

| Artifact | Today (imperative e2e) | After migration |
|---|---|---|
| Regression check | yes | yes |
| Proptest seed (corpus-guided generation) | no | free |
| Shadow-model differential check | no | free |
| Shrinkable counterexample on failure | no | free (`proptest` shrinks `Vec<Action>`) |
| Serializable for regression file / CI artifact | no | free (`Action`, `TheoremFailure` already `Serialize`) |
| Replayable across editor versions / branches | no | free |
| Mutation-test target | no | free |
| Cross-feature property check | no | free (the corpus *is* the property's domain) |
| CI dashboard signal (typed failure) | panic-string parse | typed JSON |

The same `Scenario` value drives every row. The *write* cost stays
roughly constant; the *read* count multiplies. This is what flips
the migration ROI on the previously-deferred categories: a
`PersistenceScenario` for `hot_exit_recovery_lsp_sync.rs` doesn't
just replace one e2e — it joins the proptest harness, the shadow
fs model, and the regression-file pipeline simultaneously.

## §13. Scenario taxonomy — covering every e2e

The framework expands from one struct (`BufferTheorem`) to a family
of scenarios indexed by primary observable. Each e2e file maps to
exactly one scenario type; secondary observables ride along as
context fields. Files that legitimately exercise two subsystems
(e.g., `lsp_code_action_modal.rs`) carry both subsystems in their
`ScenarioContext` and assert on the union of observables — see §14
for how composition works.

### 13.1 Twelve scenario types

| Type | Primary observable | Files (~) | Status |
|---|---|---|---|
| `BufferScenario` | text + cursors + selection | 25 done / 50 total | landed (§5, Track-B) |
| `LayoutScenario` | `RenderSnapshot` (viewport, gutter, hw cursor) | 1 done / 32 total | minimal landed; needs §9.1 expansion |
| `ModalScenario` | prompt/palette/picker/menu state | 0 / 43 | not started |
| `WorkspaceScenario` | splits, tabs, dock layout, buffer list | 0 / 19 | not started |
| `PersistenceScenario` | `VirtualFs` + session/recovery state | 0 / 23 | not started |
| `LspScenario` | scripted LSP exchange + buffer | 0 / 29 | not started |
| `StyleScenario` | `StyledFrame` (cell role × theme) | 0 / 12 | not started |
| `InputScenario` | mouse/composition events as data | 0 / 7 | not started |
| `TemporalScenario` | timed sequence of frames (`MockClock`) | 0 / 3 | not started |
| `TerminalIoScenario` | ANSI bytes via vt100 round-trip | 0 / 7 | not started |
| `PluginScenario` | plugin-driven actions + plugin script | 0 / 5 | not started |
| `GuiScenario` | wgpu/winit observables | 0 / 1 | not started; lowest priority |

Total ≈ 231 (some files are dual-category; unique e2e file count is
227 per `ls tests/e2e | wc -l`).

### 13.2 Per-category mapping

Each row below names representative files. Full file lists belong
in the per-phase implementation tickets, not here.

**`BufferScenario` (~50)** — pure text/cursor/selection. Already
landed: `case_conversion`, `sort_lines`, `indent_dedent`,
`smart_home`, `duplicate_line`, `toggle_comment`, `unicode_cursor`,
`undo_redo`, `selection`, `auto_pairs`, `save_state`, `emacs_actions`.
Pending: `basic`, `movement`, `paste`, `shift_backspace`,
`triple_click`, `block_selection`, `multibyte_characters`,
`smart_editing`, `tab_indent_selection`, `select_to_paragraph`,
`document_model`, `goto_matching_bracket`, `multicursor`,
`undo_redo_marker_roundtrip`, `undo_bulk_edit_after_save`,
`issue_1288_word_select_whitespace`, `issue_1566_arrow_selection`,
`issue_1697_ctrl_d_after_search`, `search_selection_on_punctuation`,
`overlay_extend_to_line_end`, `search_navigation_after_move`.

**`LayoutScenario` (~32)** — viewport scroll, soft-wrap, gutter,
hardware cursor row/col. The unblocking dependency is
`RenderSnapshot` per §9.1. Files: `issue_1147_wrapped_line_nav`,
`scroll_clearing`, `scroll_wrapped_reach_last_line`, `scrolling`,
`line_wrap_*` (5 files), `line_number_bugs`,
`search_center_on_scroll`, `search_*_stall_after_wrap`,
`hanging_wrap_indent`, `horizontal_scrollbar`,
`issue_1502_word_wrap_squished`, `issue_1574_*_scroll`,
`virtual_line*`, `popup_wrap_indent`, `margin`, `vertical_rulers`,
`memory_scroll_leak`, `side_by_side_diff_*`, `markdown_compose*`,
`redraw_screen`, `tab_scrolling`, `folding`,
`issue_1571_fold_indicator_lag`, `issue_1568_session_fold_restore`,
`issue_779_after_eof_shade`, `issue_1790_compose_wrap_highlight`,
`test_scrollbar_keybinds_cursor`.

**`ModalScenario` (~43)** — adds a `ModalState` observable and
modal-aware actions (`OpenPrompt(kind)`, `FilterPrompt(s)`,
`ConfirmPrompt`, `CancelPrompt`, `MenuSelect(item)`). Files:
`command_palette`, `file_browser`, `file_explorer`,
`action_popup_global`, `prompt`, `prompt_editing`, `popup_selection`,
`menu_bar`, `menu_*_bleed`, `explorer_*`, `live_grep`, `search`,
`search_replace`, `lsp_code_action_modal`, `lsp_completion_*`,
`dabbrev_completion`, `status_bar_message_click`,
`update_notification`, `sudo_save_prompt`,
`save_nonexistent_directory`, `settings`, `settings_*` (multiple),
`keybinding_editor`, `unicode_prompt_bugs`,
`issue_1718_settings_search_utf8_panic`, `preview_lsp_popup_focus`,
`cursor_under_popup`, `toggle_bars`.

**`WorkspaceScenario` (~19)** — adds `WorkspaceState { splits, tabs,
docks, buffer_list }` to the context and observable. Splits and
tabs are addressable as `SplitId`/`TabId`. Files: `buffer_groups`,
`buffer_lifecycle`, `buffer_settings_commands`, `multi_file_opening`,
`preview_tabs`, `split_focus_tab_click`, `split_tabs`, `split_view`,
`split_view_expectations`, `split_view_markdown_compose`,
`tab_config`, `tab_drag`, `copy_buffer_path`,
`issue_1540_tab_click_focus`, `position_history*` (4 files).

**`PersistenceScenario` (~23)** — adds `VirtualFs` to the context
(an in-memory FS the editor reads/writes through a fake adapter)
and `FsState` as observable. Files: `auto_revert`, `encoding`,
`external_file_save_as_tab`, `file_permissions`, `hot_exit_*`,
`large_file_*`, `on_save_actions`, `recovery`,
`save_as_language_detection`, `server_session_lifecycle`,
`session_hot_exit`, `slow_filesystem`, `stdin_input`, `symlinks`,
`unnamed_buffer_persistence`, `workspace`, `open_folder`,
`lifecycle`, `bash_profile_editing`, `binary_file`,
`save_nonexistent_directory` (dual with Modal),
`undo_bulk_edit_after_save` (dual with Buffer).

**`LspScenario` (~29)** — adds `LspScript`, an ordered list of
expected client-to-server messages and pre-written
server-to-client responses. The fake server matches messages by
shape, replies on cue, and records traffic for assertion. Files:
`lsp` and 26 `lsp_*` files; `language_features_e2e`;
`universal_lsp`; `inline_diagnostics`; `issue_1572_inlay_hint_drift`;
`issue_1573_format_buffer`. Note that `hot_exit_recovery_lsp_sync`
is dual (Persistence + LSP).

**`StyleScenario` (~12)** — pulls a `StyledFrame` via the §15
`RenderSnapshot → StyledFrame` projection (theme + role table) and
asserts on cell roles + colors via `Inspect::{Cell, Row, Column,
Region, FullFrame}`. Subsumes today's `theme_screenshots`-style
golden tests with a diffable JSON form. Files: `theme`,
`theme_screenshots`, `blog_showcases`, `cursor_style_rendering`,
`crlf_rendering`, `syntax_highlighting_coverage`,
`syntax_highlighting_embedded_offset`, `syntax_language_case`,
`glob_language_detection`, `config_language_selector`,
`csharp_language_coherence`, `warning_indicators`,
`issue_1554_scrollbar_theme_color`, `issue_1577_unicode_width`,
`issue_1598_shebang_detection`, `issue_779_after_eof_shade`.

**`InputScenario` (~7)** — extends the `Action` alphabet with
`InputEvent::{Mouse(MouseEvent), Compose(ComposeSeq), KeyChord(...)}`.
Mouse coordinates project to (line, byte) via the current
`RenderSnapshot`. Files: `mouse`, `capslock_shortcuts`, `altgr_shift`,
`csi_u_session_input`, `issue_1620_split_terminal_click_panic`,
`locale`, `tab_drag` (dual with Workspace).

**`TemporalScenario` (~3)** — adds a `MockClock` and an
`InputEvent::AdvanceClock(Duration)` action. Expectation is a
`Vec<RenderSnapshot>` taken after each clock tick. Files:
`animation`, `flash`, `status_bar_config` (timing aspects).

**`TerminalIoScenario` (~7)** — projects `StyledFrame` through the
real escape-sequence emitter, then through `vt100` back to a
typed grid; asserts on the round-trip grid. This catches escape
emission bugs without committing to specific byte sequences. Files:
`ansi_cursor`, `terminal`, `terminal_close`, `terminal_resize`,
`terminal_split_focus_live`, `rendering`, `redraw_screen` (dual).
The harness already does most of this through `render_real` /
`render_real_incremental`; the realignment formalizes it into a
scenario type.

**`PluginScenario` (~5)** — adds a `PluginScript` (the JS plugin
source as a string + the messages it's expected to emit). Plugin
actions are dispatched through the existing `process_async_messages`
path; the runner asserts on the plugin's effect on `BufferState` and
on the message log. Files: anything under `tests/e2e/plugins/`.

**`GuiScenario` (~1)** — `gui.rs`. The wgpu/winit front-end shares
the `Editor` core but has its own input layer (raw mouse, text-input
IME) and its own output layer (rasterized cells). Most editor-level
behavior in `gui.rs` is already covered by `BufferScenario` /
`LayoutScenario`; what remains is a thin layer of GUI-specific
asserts (font fallback, sub-pixel positioning). Lowest priority;
may stay imperative.

### 13.3 Cross-cutting observables

Some files exercise more than one subsystem. Examples:

| File | Categories | How it composes |
|---|---|---|
| `lsp_code_action_modal.rs` | `LspScenario` + `ModalScenario` | context carries `LspScript`; expectation includes `ModalState` |
| `hot_exit_recovery_lsp_sync.rs` | `PersistenceScenario` + `LspScenario` | context carries `VirtualFs` + `LspScript` |
| `tab_drag.rs` | `WorkspaceScenario` + `InputScenario` | context carries `WorkspaceState`; actions include `Mouse::Drag` |
| `issue_1554_scrollbar_theme_color.rs` | `LayoutScenario` + `StyleScenario` | observable is `(RenderSnapshot, StyledFrame)` |

Composition is direct, not "convert to one type or the other." See
§14 for the runner's type-level handling.

## §14. Composable scenario architecture

The Part-I runner shape (`assert_buffer_theorem(t: BufferTheorem)`)
generalizes to:

```rust
pub struct Scenario<Obs: Observable> {
    pub description: String,            // String, not &'static str (data form)
    pub context:     ScenarioContext,
    pub actions:     Vec<InputEvent>,   // superset of Action
    pub expectation: Obs,
}

pub struct ScenarioContext {
    pub buffer:    BufferContext,                  // initial_text, behavior, language, terminal
    pub workspace: Option<WorkspaceContext>,
    pub fs:        Option<VirtualFs>,
    pub lsp:       Option<LspScript>,
    pub plugins:   Option<PluginScript>,
    pub theme:     Option<Theme>,
    pub clock:     Option<MockClock>,
}

/// Anything the runner can extract from a live editor and assert on.
pub trait Observable: Serialize + DeserializeOwned + PartialEq {
    fn extract(api: &mut dyn EditorTestApi) -> Self;
}

pub fn check_scenario<Obs: Observable>(s: Scenario<Obs>)
    -> Result<(), TheoremFailure>;
```

`InputEvent` is the new top-level alphabet:

```rust
pub enum InputEvent {
    Action(Action),                  // existing 600-variant editor alphabet
    Mouse(MouseEvent),               // Click(x,y), Drag(start,end), Wheel(dx,dy)
    Compose(ComposeSeq),             // dead keys / IME
    OpenPrompt(PromptKind),          // for ModalScenario
    FilterPrompt(String),
    ConfirmPrompt,
    CancelPrompt,
    AdvanceClock(Duration),          // for TemporalScenario
    LspMessage(LspIncoming),         // server → client injection
    FsExternalEdit(PathBuf, String), // for auto_revert tests
    Wait(WaitCondition),             // semantic wait, never wall-clock sleep
}
```

The seven new variants beyond `Action` are the price of full
coverage. Each one is a typed event the runner knows how to
dispatch deterministically. Crucially, **no variant is a `KeyCode`**
— even mouse events project through the layout, not through
`crossterm`.

Each scenario type from §13.1 is a type alias / specialization:

```rust
pub type BufferScenario       = Scenario<BufferState>;
pub type LayoutScenario       = Scenario<RenderSnapshot>;
pub type ModalScenario        = Scenario<(BufferState, ModalState)>;
pub type WorkspaceScenario    = Scenario<(BufferState, WorkspaceState)>;
pub type PersistenceScenario  = Scenario<(BufferState, FsState)>;
pub type LspScenario          = Scenario<(BufferState, LspTraffic)>;
pub type StyleScenario        = Scenario<StyledFrame>;
pub type InputScenario        = Scenario<RenderSnapshot>;     // mouse asserts on cursor row/col
pub type TemporalScenario     = Scenario<Vec<RenderSnapshot>>;
pub type TerminalIoScenario   = Scenario<RoundTripGrid>;
pub type PluginScenario       = Scenario<(BufferState, PluginLog)>;
pub type GuiScenario          = Scenario<GuiSnapshot>;
```

The runner is a single entry point parameterized by `Obs`; the
specializations exist for ergonomic constructors and for
proptest-strategy specialization, not because the runner branches.

`Observable` is the interface shadow models also implement (§16).

## §15. Rendering inside the framework

This supersedes §8. The original §8 carved rendering out as "stays
imperative, forever" because the editor's render pass collapses
several conceptual stages into one `terminal.draw` call. The
realigned plan factors that pass into named layers and gives each
layer its own scenario type. Tests target the *highest* layer they
care about and stop there.

### 15.1 The four rendering layers

```
        EditorState
             │  layout(width, height)
             ▼
       RenderSnapshot       ← Class B  (LayoutScenario, theme-free)
             │  style(Theme, RoleTable)
             ▼
        StyledFrame         ← Class C  (StyleScenario, role-tagged cells)
             │  emit(Capabilities, EmitState)
             ▼
        AnsiStream          ← (rarely tested directly)
             │  vt100 round-trip
             ▼
       RoundTripGrid        ← Class D  (TerminalIoScenario)
```

Each arrow is a function. None of these layers exists as a named
public type today; building them is the bulk of the rendering-side
work. Proposed locations:

| Type | Where | Approx LOC |
|---|---|---|
| `RenderSnapshot` | `crates/fresh-editor/src/test_api.rs` (new) | 300 |
| `StyledFrame` | same | 80 |
| `RoundTripGrid` | same | 60 |
| Layer functions | `src/view/render_layers.rs` (refactored from existing render code) | ~500 net |

The refactor does **not** rewrite the renderer. It splits the
existing `render()` body into three named functions:

```rust
fn layout(state: &EditorState, dim: TerminalDim) -> RenderSnapshot;
fn style(snapshot: &RenderSnapshot, theme: &Theme, roles: &RoleTable) -> StyledFrame;
fn emit(frame: &StyledFrame, caps: &Capabilities) -> AnsiStream;
```

Today's `render()` is the composition. Production stays unchanged
(it still composes them in one call); tests call them
individually.

### 15.2 What each scenario type catches

| Type | Catches | Doesn't catch |
|---|---|---|
| `LayoutScenario` | viewport reconciliation, wrap math, gutter widths, hw cursor row/col, popup placement, scrollbar geometry | colors, glyph choice, escape correctness |
| `StyleScenario` | theme contrast, role-to-color mapping, modifier flags, syntax-highlight color regressions | terminal-emulator quirks |
| `TerminalIoScenario` | escape emission bugs, optimization regressions (e.g., redundant SGR resets), incremental redraw correctness | terminal-side bugs (xterm vs kitty) |
| `TemporalScenario` | animation frame correctness, fade/flash duration, blink phase, scroll smoothing | wall-clock drift |
| `GuiScenario` | font fallback, sub-pixel positioning, IME interaction | wgpu driver bugs |

Together these cover everything Part-I §8.3 listed as "stays
imperative, forever" except for actual terminal-emulator and
GPU-driver bugs. Those are correctly outside the editor's
responsibility.

### 15.3 Visual regression as a `StyleScenario`

Today's `tests/e2e/theme_screenshots.rs` saves PNG-ish snapshots and
compares byte-for-byte. Diff failures are uninspectable.

Realigned: a `StyleScenario` with `Inspect::FullFrame` and
`expected: StyledFrame` loaded from a JSON snapshot file. Diffs are
structural (cell `(x,y)` changed role from `Selection` to `Normal`,
fg `#abc` to `#def`). Snapshot regeneration is a CLI flag on the
test runner. Today's PNG pipeline can be deleted.

### 15.4 Animations as `TemporalScenario`

```rust
TemporalScenario {
    description: "Flash banner fades over 250ms".into(),
    context: ScenarioContext {
        buffer: BufferContext::default(),
        clock: Some(MockClock::epoch()),
        ..Default::default()
    },
    actions: vec![
        InputEvent::Action(Action::ShowFlash("saved".into())),
        InputEvent::AdvanceClock(Duration::from_millis(50)),
        InputEvent::AdvanceClock(Duration::from_millis(50)),
        InputEvent::AdvanceClock(Duration::from_millis(150)),
    ],
    expectation: vec![
        snapshot_t0_with_banner,
        snapshot_t50_partially_faded,
        snapshot_t100_more_faded,
        snapshot_t250_no_banner,
    ],
}
```

Requires a single hook: `Editor` reads time through a
`Clock` trait, default-impl uses the system clock, test-impl uses
`MockClock`. ~30 LOC of production change, gated like the existing
test API.

### 15.5 Layered shadows

Each layer admits its own shadow:

| Layer | Shadow | Catches |
|---|---|---|
| `step` | reference editor (already discussed) | logic bugs |
| `layout` | naive wrap algorithm in pure Rust | wrap regressions, viewport drift |
| `style` | role-table-driven projection | theme regressions, role-to-color mismatches |
| `emit` | minimal escape emitter | redundant escapes, incorrect cursor positioning |

Each shadow runs on every applicable scenario in the corpus. The
naive wrap shadow alone would have caught `issue_1502` and likely
several `line_wrap_*` regressions before they shipped — uniform
proptest never finds them because the failing inputs are specific
(double-width chars at exactly column `width-1`); the shadow finds
them on the first scenario that hits them.

## §16. Shadow model framework

One trait, multiple impls, every scenario auto-checked.

```rust
pub trait ShadowModel {
    /// Subset of `EditorTestApi` this shadow can simulate. The
    /// runner skips scenarios whose context references subsystems
    /// the shadow doesn't claim to handle.
    fn supports(&self) -> ShadowCapabilities;

    fn dispatch(&mut self, event: &InputEvent);

    fn extract<O: Observable>(&self) -> O;
}

pub struct ShadowCapabilities {
    pub buffer:    bool,
    pub workspace: bool,
    pub fs:        bool,
    pub lsp:       bool,
    pub layout:    bool,   // can produce RenderSnapshot
    pub style:     bool,   // can produce StyledFrame
}
```

The differential test:

```rust
#[test]
fn corpus_agrees_with_buffer_shadow() {
    let shadow = BufferShadow::new();
    for scenario in corpus::iter().filter(|s| BufferShadow::handles(s)) {
        check_scenario_against_shadow(&scenario, &shadow)
            .expect("shadow disagreement");
    }
}
```

Adding a new shadow:

1. Implement `ShadowModel` for the alternate semantics (or
   alternate algorithm).
2. Declare which scenario types it supports via `ShadowCapabilities`.
3. The corpus-wide differential test picks it up automatically.

Shadows live in `tests/common/shadows/`:

| Shadow | Supports | Purpose |
|---|---|---|
| `BufferShadow` | buffer | reference for editor-as-state-machine; catches actions.rs / state.rs class bugs |
| `LayoutShadow` | buffer, layout | naive wrap algorithm; catches §15.5 wrap-table regressions |
| `StyleShadow` | layout, style | role-driven projection from RenderSnapshot to StyledFrame |
| `RopeShadow` | buffer | text stored in `Vec<u8>` not the production rope; catches rope-implementation bugs |
| `MultiCursorShadow` | buffer | naive cursor merge; cross-checks the production merge |
| `UndoShadow` | buffer | snapshot-stack undo; cross-checks the action-trace undo |

Today's `tests/shadow_model_*.rs` files are a starting point — they
become `ShadowModel` impls and are deleted from the bespoke
test files (the corpus loop subsumes them).

## §17. Implementation roadmap (revised)

Phase numbers continue from Part-I. Each phase includes the
framework extension *and* the e2e files migrated under it, so
landing a phase is observable in the test count.

### Phase E — data-model lockdown (small, mechanical)

Prerequisite for everything else. Ships as one PR.

- Derive `Serialize`/`Deserialize` on `BufferTheorem`, `TraceTheorem`,
  `LayoutTheorem`.
- Replace `&'static str` with `String` (or `Cow<'static, str>`) on
  the three theorem structs.
- Lift `BehaviorFlags`, filename, `TerminalSize` into the struct.
  Delete the runner overloads (`assert_buffer_theorem_with_*`).
- Promote `EvaluatedState` (`property.rs:23`) to the canonical
  `BufferState` type.
- Add a `ShadowModel` trait skeleton + `BufferShadow` impl that
  delegates to the live editor (no-op differential).
- CI job: dump the corpus to JSON, fail on schema-breaking diffs.

Acceptance: every existing semantic test continues to pass; corpus
JSON exists; `BufferShadow` runs the corpus and reports zero
disagreements (it's the same editor; the harness is what's being
validated).

### Phase F — RenderSnapshot + LayoutScenario expansion

- Land `RenderSnapshot` per §9.1 / §15.
- Refactor `render()` into `layout` / `style` / `emit` (additive;
  production keeps composing them).
- Build `LayoutShadow` (naive wrap, ~200 LOC).
- Migrate the 32 Class B files.

Acceptance: 32 LayoutScenarios in `tests/semantic/layout/`, the
naive-wrap differential passes.

### Phase G — ModalScenario

- Add `ModalState` observable to `EditorTestApi`.
- Add `OpenPrompt`/`FilterPrompt`/`ConfirmPrompt`/`CancelPrompt` /
  `MenuSelect` to `InputEvent`.
- Migrate the 43 modal files.

Acceptance: 43 ModalScenarios; the existing palette/picker/settings
e2e files retired (or kept thin as redundant terminal-side proofs
per §7).

### Phase H — WorkspaceScenario

- Add `WorkspaceState` observable.
- Address splits/tabs/docks via `SplitId`/`TabId` handles.
- Migrate 19 files.

### Phase I — PersistenceScenario + VirtualFs

- Build `VirtualFs` (in-memory, with adapter into the existing
  filesystem trait).
- Add `FsExternalEdit` to `InputEvent` (for `auto_revert` etc.).
- Migrate 23 files.

### Phase J — LspScenario

- Build `LspScript` + fake LSP server adapter.
- Add `LspMessage` to `InputEvent`.
- Migrate 29 files.

### Phase K — StyleScenario + visual regression migration

- Land `StyledFrame` + `style()` projection.
- `StyleShadow` (alternate role-to-color).
- Replace `tests/e2e/theme_screenshots.rs` byte-snapshot tests with
  `StyleScenario` JSON snapshots.
- Migrate 12 style-coupled files.

### Phase L — InputScenario + mouse projection

- Add `MouseEvent` projection through `RenderSnapshot` (cell → byte).
- Add `Compose`, `KeyChord` to `InputEvent`.
- Migrate 7 files.

### Phase M — TemporalScenario + MockClock

- Inject `Clock` trait (~30 LOC production change).
- Add `AdvanceClock` to `InputEvent`.
- Migrate 3 files.

### Phase N — TerminalIoScenario + RoundTripGrid

- Formalize the existing `render_real` / `vt100` flow into a
  `TerminalIoScenario`.
- `EmitShadow` (alternate escape generator).
- Migrate 7 files.

### Phase O — PluginScenario

- `PluginScript` carries plugin source + expected message log.
- Migrate 5 files.

### Phase P — GuiScenario (best-effort)

- Decide whether `gui.rs` justifies its own scenario type or
  whether its editor-level content is already covered by §H/§G/§F
  and the GUI-specific bits stay imperative. Defer until last.

### Cross-phase: corpus-guided proptest

Once the corpus exists (Phase E), build a proptest strategy that
samples scenario prefixes from the corpus and generates random
tails. Run as a soak job in CI. Counterexamples write
`tests/semantic/regressions/` JSON files. This thread runs in
parallel with the migration phases and starts paying off
immediately — it doesn't block any phase.

### Sequencing

Phase E is on the critical path. Phases F–O are independent and
can run in parallel by different owners, ordered by ROI:

1. F (LayoutScenario) — biggest test count after BufferScenario
2. G (ModalScenario) — biggest absolute count
3. K (StyleScenario) — kills the PNG pipeline; high signal on theme bugs
4. J (LspScenario) — most flake-prone today
5. I (PersistenceScenario)
6. H (WorkspaceScenario)
7. N (TerminalIoScenario)
8. L (InputScenario)
9. M (TemporalScenario)
10. O (PluginScenario)
11. P (GuiScenario, if pursued)

Estimated effort: each phase is 2–4 weeks for one engineer once
Phase E lands; total ≈ 6 person-months for the framework + 3 for
migrations, parallelizable.

## §18. Updated risks & non-goals

### Risks specific to the realignment

- **Scenario context becomes a god object.** Mitigation:
  `ScenarioContext` fields are `Option<...>` so a buffer-only
  scenario carries only `BufferContext`. JSON schema enforces
  presence iff the runner needs it.
- **Fake LSP / VirtualFs drift from real subsystems.** Mitigation:
  the existing imperative e2e files for those subsystems stay
  for one release after each phase ships. Differential testing
  between fake and real catches drift before retirement.
- **`InputEvent` enum grows unmaintainable.** Mitigation: keep
  `Action` separate; only add new variants when a scenario type
  legitimately needs them. The seven non-`Action` variants in
  §14 are believed to be the ceiling, not a starting point.
- **Snapshot churn on `RenderSnapshot` schema changes.**
  Mitigation: snapshot files are `serde_json` with `#[serde(default)]`
  on additive fields; schema changes are reviewed as data-model
  changes, not as test churn.
- **Corpus-guided proptest finds bugs that aren't in the migrated
  scenario but block CI.** Mitigation: the soak job is non-blocking;
  found bugs become regression JSON files and a separate gating
  test.

### Non-goals (revised, slimmer)

- **Theorem-prover export.** Considered and rejected; data form is
  for proptest + shadow leverage, not Lean. Removing this constraint
  drops several requirements (formal `step` semantics, encoded
  unicode tables, etc.).
- **Replacing the rope buffer with a verified one.** Out of scope;
  the rope is the production subject.
- **GPU/driver-level GUI tests.** wgpu rendering quality is wgpu's
  problem.
- **Terminal-emulator-level tests** (xterm vs kitty vs alacritty).
  We test the editor's *output*, not its consumers.

## §19. Acceptance criteria for the realignment

The realignment is "done" when:

- [ ] `tests/e2e/` either contains zero files or contains only the
      handful kept as redundant terminal-side proofs (per §7) and
      the GUI-specific subset (per §13.1, GuiScenario).
- [ ] `tests/semantic/` contains all twelve scenario types with at
      least one example per type.
- [ ] The corpus dumps to a JSON directory in CI artifacts on every
      run.
- [ ] At least three shadow models are wired into the
      corpus-differential CI job.
- [ ] Corpus-guided proptest runs as a soak job; counterexamples
      produce regression JSON files automatically.
- [ ] `theme_screenshots.rs` PNG pipeline is deleted.
- [ ] The rendering-test split between `tests/e2e/` (renders) and
      `tests/semantic/` (doesn't) no longer exists; rendering is
      tested via §15's layered scenarios within the same framework.
- [ ] Documentation: `CONTRIBUTING.md` rule #2 is rewritten to
      describe the scenario-type taxonomy as the primary test idiom,
      with terminal-side e2es as an explicit redundant proof
      where needed.
