# Review Diff — Open Defects

**Feature:** `fresh` Review Diff mode (launched via `Ctrl+P → "Review Diff"`).
**Branch:** `claude/tui-editor-usability-eval-0LHgo` · **Editor:** 0.2.23 debug build.
**Evidence:** Screen captures under `/tmp/validate/c_*.txt` (pass 3, interactive),
`/tmp/eval-workspace/pass2/p2_*.txt` (pass 2, scenario sweep), and
`/tmp/eval-workspace/screen_*.txt` (pass 1, initial walk-through).

All items below are capture-proven open defects. Everything that already
works is in the appendix, as a sanity check on what the feature is for.

---

## P0 — Ship-blockers

### 1. Terminal resize is unrecoverable

Shrink 160×45 → 80×24 → grow back leaves menu, toolbar, and tab row
hidden. `r` refresh doesn't fix it; resize-bump doesn't fix it;
close-and-reopen of the review tab leaves stale rendering on the right
pane. Only killing and relaunching the editor recovers.

*Evidence:* `c_40_at_80.txt`, `c_41_back.txt`, `c_42_after_r.txt`,
`c_44_bump.txt`, `c_46_reopen.txt`.

### 2. Side-by-side `n` / `p` leave status bar stale

`n` / `p` move the viewport but do NOT update the status-bar `Ln` /
`Col`. Arrow keys update it correctly; `n` / `p` leave it stale.

*Evidence:* `c_18_sxs.txt` (Ln 1) → `c_21_sxs_n3.txt` (viewport at
`L054` but `Ln 8`) → `c_19_sxs_down1.txt` (Down → `Ln 7`, immediate
update).

### 3. No "Hunk N of M" indicator

Status bar shows only the *total* count (`Review Diff: 35 hunks`) and
never the current index, in either the unified or side-by-side pane.

*Evidence:* `c_03`–`c_09` — the counter is unchanged across every hunk
jump.

### 4. Empty state is ambiguous

Non-git directory and clean git repo render *byte-identically*: empty
`GIT STATUS` pane, `DIFF` header with no filename, `Review Diff: 0
hunks`. The i18n keys `status.not_git_repo` and `panel.no_changes`
exist but are never displayed.

*Evidence:* `c_22_nogit.txt`, `c_23_clean.txt`.

---

## P1 — High-impact UX gaps

### 5. Unified pane has no per-keyword syntax highlighting

Side-by-side does (`def` → fg 207, `return` → fg 51); unified pane uses
one foreground color per `+` / `-` line, no language tokens.

*Evidence:* `c_33_unified_syntax.txt` shows `[1m[38;5;51mdef add(a:
int, b: int) -> int:[0m` — single color for the whole keyword-rich
line. `c_34_sxs_syntax.txt` shows `[38;5;207mdef`, `[38;5;51mreturn`
with per-token colors.

### 6. `n` / `p` are dead in the files pane

Pressing `n` while focus is on the files pane neither advances the
file selection nor moves the diff cursor.

*Evidence:* `c_36_n_filespane.txt`.

### 7. `n` / `p` do not cross file boundaries

From the last hunk of `a.py`, further `n` presses don't jump to the
first hunk of `b.py`; cursor clamps just past the last hunk header of
the current file.

*Evidence:* `c_37_n_pastend.txt`.

### 8. `n` / `p` hints appear in the toolbar only after `Tab`

A user who never Tabs into the diff pane never learns hunk navigation
exists.

*Evidence:* `c_01_review.txt` (files-pane toolbar, no `n` / `p`) vs
`c_13_diff_start.txt` (diff-pane toolbar shows `n Next  p Prev`).

### 9. Whitespace-only changes have no per-character highlight

Trailing-space and double-space edits look identical on the `-` and `+`
lines; only the leading marker differs.

*Evidence:* `screen_13_whitespace_ansi.txt` — full-line bg, no
intra-line spans.

---

## P2 — Standards & discoverability

### 10. Non-standard hunk header

Renders as `@@ L006 @@` (a context-line preview) instead of git-standard
`@@ -X,Y +X,Y @@ <signature>`. Breaks muscle memory from `git diff` /
GitHub / `vimdiff` and prevents counting added/removed lines per hunk.

*Evidence:* `c_15_start.txt`.

### 11. Review Diff is in zero top-level menus

Walked every menu (File / Edit / View / Selection / Go / LSP / Help) —
the feature is not present. Only `Go → Command Palette…` exists, which
delegates back to `Ctrl+P`.

*Evidence:* `c_26_menu_file.txt`–`c_32_menu_help.txt`.

### 12. F1 in-app Manual lacks the feature

Searching the Manual for "review diff" returns `No matches found`.

*Evidence:* `screen_25_help_search.txt`.

### 13. Fuzzy palette is subsequence-only, no typo tolerance

"revw difff" returns `Markdown: Toggle Compose/Preview` instead of
`Review Diff`.

*Evidence:* `screen_22_typo.txt`.

### 14. `\ No newline at end of file` marker is dropped

A file stripped of its trailing newline shows only the normal
`+modified` line with no marker — reviewers will miss newline
regressions in shell scripts / fixtures.

*Evidence:* `p2_09_nonl.txt`.

### 15. Merge-conflict files appear twice

`UU conf.txt` shows in both `Staged` and `Changes` sections with `(no
diff available)` and no resolution affordance.

*Evidence:* `p2_43_conflict.txt`.

---

## P3 — Polish & edge cases

### 16. `N` / `n` key collision

Lowercase `n` = next hunk; capital `N` = open Note prompt. Distinct but
easy to mis-fire; no other toolbar key pairs rely on case sensitivity.

*Evidence:* `p2_22_lower_n.txt` vs `p2_23_upper_N.txt`.

### 17. Chrome vanishes at 80 × 24

Menu, tab row, and toolbar all disappear at narrow widths. No graceful
degradation to a single-glyph legend.

*Evidence:* `c_40_at_80.txt`.

### 18. No overflow indicator for truncated lines

`End` scrolls horizontally but nothing signals that a line continues.

*Evidence:* `screen_04_review_plain.txt`, `p2_37_end.txt`.

### 19. Files list sorts alphabetically, not naturally

`many/f10.txt` precedes `many/f2.txt`.

*Evidence:* `p2_02_review_open.txt`.

### 20. No line numbers in the unified diff gutter

Side-by-side has them; unified does not. Makes "which line is that?"
conversations awkward.

*Evidence:* `c_15_start.txt`.

### 21. No "reopen last review" command

After `q`, every re-entry requires the 4-keystroke palette round-trip.

---

## Suggested sprint bundle

- **A — stabilise:** 1, 2, 3, 4.
- **B — navigation ergonomics:** 5, 6, 7, 8.
- **C — standards & a11y:** 9, 10, 14.
- **D — discoverability & polish:** 11, 12, 13, 15, 16–21.

---

## Appendix A — What Review Diff is trying to be

A single keyboard-driven review surface that lets a developer do a PR-
style read-through of local changes without leaving the editor:

- List every changed file, grouped by `Staged` / `Changes` / `Untracked`.
- Show a unified colour diff on the right; drill into side-by-side with
  `Enter`.
- Stage / unstage / discard hunks with `s` / `u` / `d`.
- Attach inline `c`omments and a per-session `N`ote.
- Export the whole session to Markdown or JSON for sharing.

## Appendix B — Verified working (reference)

These behaviours were explicitly tested and do not need changes:

- `n` / `p` hunk navigation in the unified pane (jumps between hunk
  headers correctly).
- Viewport auto-scrolls to follow the cursor in the unified pane.
- Current hunk header is highlighted (bg `256:17`, dark blue).
- `NO_COLOR=1` env var is honoured (only `[0m` / `[4m` emitted).
- Cursor position is preserved across `Tab` between panes.
- Inline comments render as `» [hunk] …` and persist across close /
  reopen (backed by `.review/session.md`).
- `d` discard shows a confirmation dialog with an "undone" warning.
- Rename detection renders as `R old → new`.
- Unicode and emoji align correctly in both panes.
- Deleted-file drill-down into side-by-side works (no hang).
- `q` cleanly closes the review; the editor survives all tested keys
  including `Ctrl+C`.
- Debug-build input responsiveness is adequate (50 `PageDown` presses
  in ~3 s with no lag).
