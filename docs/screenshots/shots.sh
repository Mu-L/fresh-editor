#!/bin/bash
# Shared screenshot definitions and sample files
# Sourced by screenshot.sh and screenshot-x11.sh

# ============================================================================
# Screenshot definitions
# Format: "name:file:width:height:key_sequence"
# ============================================================================

# Key sequence format: keys:delay:keys:delay:...
# Special keys: C-x (Ctrl+x), M-x (Alt+x), S-x (Shift+x), ENTER, ESC, TAB, UP, DOWN, LEFT, RIGHT
# Menu navigation: M-v (View menu), M-f (File menu), etc.
SHOTS=(
    # Hero: file explorer + code + horizontal split with terminal at bottom
    # Open explorer, focus editor, split horizontal, open terminal in bottom, run top, focus top
    "hero:main.rs:140:40:1.0:C-e:0.3:C-e:0.3:C-p:0.3:split horizontal:0.3:ENTER:0.5:C-p:0.3:open terminal:0.3:ENTER:1.0:top:0.3:ENTER:0.8:M-[:0.3"

    # Multi-cursor: select word and add cursors at next matches
    # Select word under cursor, then add cursors at next matches
    "multicursor:main.rs:120:35:1.0:C-w:0.3:C-d:0.3:C-d:0.3:C-d:0.5"

    # LSP Completion popup - go to end, new line, trigger completion
    "completion:main.rs:120:35:1.0:C-End:0.3:ENTER:0.3:    users.:0.3:C-Space:0.5"

    # Terminal with command running
    "terminal:main.rs:120:35:1.0:C-p:0.5:open terminal:0.5:ENTER:1.0:cargo --version:0.3:ENTER:0.5"

    # Git log plugin
    "gitlog:main.rs:120:40:1.0:C-p:0.3:git log:0.3:ENTER:0.8"

    # Find in files (project-wide search)
    "search:main.rs:120:35:1.0:C-p:0.3:find in files:0.3:ENTER:0.5:user:0.5"

    # Command palette showcase
    "palette:main.rs:120:35:1.0:C-p:0.5"

    # Menu with submenu open (View menu -> Theme submenu)
    "menu:main.rs:120:35:1.0:M-v:0.3:DOWN:0.1:DOWN:0.1:DOWN:0.1:RIGHT:0.3"

    # Diagnostics (need a file with errors)
    "diagnostics:error.rs:120:35:1.5"
)

# ============================================================================
# Sample files
# ============================================================================

create_samples() {
    local dir="$1"
    mkdir -p "$dir"

    cat > "$dir/main.rs" << 'RUST'
use std::collections::HashMap;

/// A simple user management system
struct User {
    name: String,
    email: String,
    is_active: bool,
}

impl User {
    fn new(name: &str, email: &str) -> Self {
        User {
            name: name.to_string(),
            email: email.to_string(),
            is_active: true,
        }
    }

    fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

fn main() {
    let mut users: HashMap<u32, User> = HashMap::new();

    users.insert(1, User::new("Alice", "alice@example.com"));
    users.insert(2, User::new("Bob", "bob@example.com"));

    for (id, user) in &users {
        println!("[{}] {}", id, user.greet());
    }
}
RUST

    cat > "$dir/plugin.ts" << 'TS'
import { Editor, Command } from "fresh";

interface PluginConfig {
    greeting: string;
    showNotification: boolean;
}

export function activate(editor: Editor) {
    const config: PluginConfig = {
        greeting: "Hello from Fresh!",
        showNotification: true,
    };

    editor.registerCommand("hello", () => {
        if (config.showNotification) {
            editor.notify(config.greeting);
        }
    });

    editor.registerCommand("insert-date", () => {
        const date = new Date().toISOString();
        editor.insertText(date);
    });
}
TS

    cat > "$dir/app.py" << 'PYTHON'
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class Task:
    title: str
    completed: bool = False
    priority: int = 0

class TodoList:
    def __init__(self):
        self.tasks: List[Task] = []

    def add(self, title: str, priority: int = 0) -> Task:
        task = Task(title=title, priority=priority)
        self.tasks.append(task)
        return task

    def complete(self, index: int) -> Optional[Task]:
        if 0 <= index < len(self.tasks):
            self.tasks[index].completed = True
            return self.tasks[index]
        return None

if __name__ == "__main__":
    todo = TodoList()
    todo.add("Learn Fresh editor", priority=1)
    todo.add("Write some code")
    print(f"Tasks: {len(todo.tasks)}")
PYTHON

    cat > "$dir/config.json" << 'JSON'
{
    "editor": {
        "theme": "dark",
        "fontSize": 14,
        "fontFamily": "JetBrains Mono",
        "tabSize": 4,
        "wordWrap": true
    },
    "keybindings": {
        "save": "Ctrl+S",
        "find": "Ctrl+F",
        "palette": "Ctrl+Shift+P"
    },
    "plugins": [
        "color-highlighter",
        "todo-highlighter",
        "path-complete"
    ]
}
JSON

    # File with intentional errors for diagnostics screenshot
    cat > "$dir/error.rs" << 'RUST'
use std::collections::HashMap;

fn main() {
    let x: i32 = "hello";  // Type mismatch error

    let unused_var = 42;   // Unused variable warning

    println!("{}", undefined_var);  // Undefined variable

    let result = divide(10, 0);
}

fn divide(a: i32, b: i32) -> i32 {
    a / b  // Missing return for error case
}

struct User {
    name: String,
    age: u32
}

impl User {
    fn greet(&self) {
        println!("Hello, {}!", self.name);
    }
}
RUST
}
