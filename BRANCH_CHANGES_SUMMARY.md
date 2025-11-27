# Architecture Migration Branch Summary

**Branch:** `claude/complete-architecture-migration-01EWrn6cxZoWCPbiAc4rZVGp`
**Base:** `origin/master`
**Date:** November 2025

## Overview

This branch completes a significant architecture migration to remove duplicate state from `EditorState` and establish `SplitViewState` as the authoritative source of truth for per-view state (cursors and viewport).

## Problem Statement

The codebase had duplicate state stored in two places:
- **EditorState** (per buffer): Had `cursors` and `viewport` fields
- **SplitViewState** (per split): Also had `cursors` and `viewport` fields

This caused several bugs:
1. **Sync loops**: Changes to one viewport would sync to the other, then sync back, causing flickering
2. **Stale state**: After cursor movement, scroll position would reset because sync copied old values
3. **Wrong dimensions**: EditorState.viewport dimensions were used before split_rendering resized them

## Solution: Single Source of Truth

The migration established **SplitViewState as authoritative** while keeping EditorState as a working copy during event processing:

```
Before (problematic):
  EditorState.cursors  ←sync→  SplitViewState.cursors
  EditorState.viewport ←sync→  SplitViewState.viewport

After (clean):
  SplitViewState.cursors   = authoritative
  SplitViewState.viewport  = authoritative
  EditorState             = working copy during event processing
```

## Changes by Commit

### 1. Complete view state architecture migration and cleanup (5c6d28f)

- Removed redundant `save_current_split_view_state()` function
- Added comprehensive documentation for view state synchronization
- Documented `temporary_split_state()` pattern in split_rendering.rs
- Updated ARCHITECTURE.md to reflect completed migration status

**Files:** `docs/ARCHITECTURE.md`, `src/app/mod.rs`, `src/view/ui/split_rendering.rs`

### 2. Remove cursors/viewport from EditorState - Option A migration (58d1c9d)

- Removed `cursors` and `viewport` fields from `EditorState` struct
- Updated `apply()` signature to take cursors/viewport as parameters
- Updated `new()` and `from_file()` to not initialize cursors/viewport
- Removed helper methods that depended on self.cursors/viewport
- Updated `prepare_for_render()` to take viewport as parameter

**Files:** `src/state.rs`

### 3. Continue migration - remove cursors/viewport from EditorState (36e1909)

- Updated `EditorState` constructors to remove width/height parameters
- Updated `action_to_events` to take cursors/viewport as parameters
- Fixed input.rs: scrollbar functions, mouse handling, completion
- Fixed app/mod.rs: buffer creation, position history, navigation
- Updated `apply()` calls to pass cursors/viewport from split_view_states

**Files:** `src/app/input.rs`, `src/app/mod.rs`, `src/app/render.rs`, `src/input/actions.rs`

### 4. Continue removing sync functions and updating apply() calls (0e5b624)

- Removed `sync_editor_state_to_split_view_state` function (no longer needed)
- Removed `sync_split_view_state_to_editor_state` function (no longer needed)
- Removed all calls to sync functions throughout the codebase
- Updated `apply_event_to_active_buffer` to pass cursors/viewport from split_view_states
- Fixed plugin command handlers to use split_view_states for cursor operations
- Fixed navigation functions (navigate_back/forward)
- Fixed split functions (next_split/prev_split)

**Files:** `src/app/mod.rs`

### 5. Fix clipboard functions to use split_view_states (ea20aab)

- Updated `copy_selection` to get cursor ranges from split_view_states
- Updated `cut_selection` to get deletion ranges and cursor_id from split_view_states
- Updated `paste` to get cursor position from split_view_states

**Files:** `src/app/mod.rs`

## Files Modified Summary

| File | Lines Changed | Description |
|------|---------------|-------------|
| `src/app/mod.rs` | +537/-521 | Core event handling, state management |
| `src/app/input.rs` | +241 changes | Input handling with new state model |
| `src/input/actions.rs` | +177 changes | Action handling with new signatures |
| `src/state.rs` | +39/-81 | EditorState struct simplification |
| `src/app/render.rs` | +33 changes | Rendering adjustments |
| `src/view/ui/split_rendering.rs` | +14 | Documentation for temporary_split_state() |
| `docs/ARCHITECTURE.md` | +34 changes | Architecture documentation updates |

**Total:** 7 files changed, ~635 insertions, ~521 deletions

## New Data Flow

After this migration, the data flow is:

```
1. User switches to split B → SplitViewState[B] provides cursors/viewport
2. User types/navigates → Events applied with cursors/viewport from SplitViewState
3. apply() returns updated cursors/viewport → stored back to SplitViewState[B]
4. Render split A (inactive) → use SplitViewState[A] directly
5. Render split B (active) → use SplitViewState[B] directly
```

## Key Architectural Decisions

1. **`apply()` takes cursors/viewport as parameters** - Instead of reading from `self`, the apply function now receives and returns cursor/viewport state explicitly.

2. **No sync functions needed** - The old bidirectional sync between EditorState and SplitViewState has been eliminated.

3. **SplitViewState is always used** - All rendering and cursor operations now read directly from SplitViewState.

4. **EditorState simplified** - EditorState now focuses solely on buffer content (PieceTree, undo/redo, syntax highlighting, overlays).

## Status

This is a **work in progress** branch. The last commit (ea20aab) reports 162 compilation errors remaining. The migration approach is sound but needs continued work to resolve all call sites.

## Testing Recommendations

After completing the migration:
1. Test split views with same buffer - each should have independent scroll/cursor
2. Test cursor movement triggers correct scrolling
3. Test view transforms (git blame) work correctly
4. Test clipboard operations across splits
5. Run full test suite to catch regressions
