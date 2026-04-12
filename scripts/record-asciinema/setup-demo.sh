#!/usr/bin/env bash
# Create the demo workspace used by record.py.
#
# Writes a small Rust project with a local (uncommitted) edit so the
# git gutter and "Review Diff" view have something to show.
#
# Usage: setup-demo.sh [DEMO_DIR]     (default: /tmp/fresh-demo-workspace)

set -euo pipefail

DEMO_DIR="${1:-/tmp/fresh-demo-workspace}"

rm -rf "$DEMO_DIR"
mkdir -p "$DEMO_DIR"

cat > "$DEMO_DIR/main.rs" <<'RS'
// Fresh — a terminal editor with IDE features.

use std::collections::HashMap;

/// A simple user management system.
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
}

impl User {
    pub fn new(id: u64, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
            is_active: true,
        }
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

pub struct UserStore {
    users: HashMap<u64, User>,
}

impl UserStore {
    pub fn new() -> Self {
        Self { users: HashMap::new() }
    }

    pub fn insert(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    pub fn active_users(&self) -> impl Iterator<Item = &User> {
        self.users.values().filter(|u| u.is_active)
    }
}

fn main() {
    let mut store = UserStore::new();
    store.insert(User::new(1, "Alice", "alice@example.com"));
    store.insert(User::new(2, "Bob",   "bob@example.com"));
    store.insert(User::new(3, "Carol", "carol@example.com"));

    for user in store.active_users() {
        println!("{}: {}", user.name, user.email);
    }
}
RS

cat > "$DEMO_DIR/notes.md" <<'MD'
# Notes

Demo project used to showcase Fresh.
MD

cat > "$DEMO_DIR/Cargo.toml" <<'TOML'
[package]
name = "demo"
version = "0.1.0"
edition = "2021"
TOML

(
    cd "$DEMO_DIR"
    GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
      git -c init.defaultBranch=main \
          -c user.email=demo@local \
          -c user.name=Demo \
          -c commit.gpgsign=false \
          init -q
    GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
      git -c user.email=demo@local -c user.name=Demo -c commit.gpgsign=false \
          add . >/dev/null
    GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null \
      git -c user.email=demo@local -c user.name=Demo -c commit.gpgsign=false \
          commit -qm "initial"

    # Introduce a local change so the git gutter and Review Diff show hunks.
    sed -i 's|pub fn new(id: u64|pub fn new(id: u64 /* FIXME */|' main.rs
    echo '// TODO: add error handling for duplicate IDs' >> main.rs
)

echo "demo workspace ready at $DEMO_DIR"
