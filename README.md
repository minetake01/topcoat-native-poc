# Topcoat Native Explorer

A file explorer proof of concept designed with Topcoat's `view!` syntax and powered by a Rust backend and native WinUI 3 controls—without a DOM or WebView.

The interface follows the Windows 11 File Explorer layout, including a Mica backdrop, navigation controls, command bar, left sidebar, details view, and status bar. English is the default and only interface language in this PoC.

## Implemented

- Real file-system directory and file enumeration
- Back, Forward, Up, and Refresh navigation
- Clickable breadcrumb navigation
- Sidebar with Home, known folders, and available drives
- Incremental search within the current folder
- Four-column table with Name, Date modified, Type, and Size
- Folder-first ascending and descending sorting
- Single selection and double-click folder navigation
- Open button for the selected item
- Files opened with their Windows-associated applications
- Modification timestamps shown in local time
- File type, size, and generic icon presentation
- Mica backdrop with OS theme integration
- UI Automation names and native ListView selection

This version is intentionally read-only. New, Cut, Copy, Paste, Rename, and Delete are represented visually but explicitly disabled.

## Topcoat source

The complete interface is declared with Topcoat syntax in [`app/src/main.rs`](app/src/main.rs). Its state and event model retain the original PoC's Topcoat-style authoring experience.

```rust
view! { cx =>
    signal explorer = ExplorerState::initial();
    let snapshot = explorer.get();

    <main class="p-3 space-y-2">
        <input
            type="text"
            :value=$(snapshot.query())
            @input=$(|event: Event| {
                explorer.set(explorer.get().with_query(event.target.value))
            })
        >

        <table
            :key=$(snapshot.current_display())
            :rows=$(snapshot.rows())
            :selected=$(snapshot.selected_key())
            :width=$(table_width)
            :height=$(table_height)
            @select=$(|event: Event| explorer.set(explorer.get().select(event.target.value)))
            @activate=$(|event: Event| explorer.set(explorer.get().activate(event.target.value)))
            @sort=$(|event: Event| explorer.set(explorer.get().sort_by(event.target.value)))
        ></table>
    </main>
}
```

`<table>` is a native element added by this project. Its `:key` identifies the native ListView by location so that a folder change cannot carry an old selection into the new directory. The official Topcoat parser produces an AST that is lowered into this WinUI 3 tree:

```text
<table>
  -> StackPanel
       -> Grid (column header buttons)
       -> ListView (virtualization and selection)
            -> Grid (four cells per row)
```

Table data uses the renderer-neutral `TableRow` type. This establishes a native boundary that preserves virtualization, selection, and accessibility instead of imitating DOM `table`, `tr`, and `td` elements.

## Project structure

| Path | Responsibility |
|---|---|
| `app/src/main.rs` | Explorer UI written with Topcoat syntax |
| `app/src/explorer.rs` | Rust file system, history, search, sorting, and ShellExecute integration |
| `crates/topcoat-native-macro` | Topcoat AST to WinUI 3 lowering |
| `crates/topcoat-native` | Signal and event vocabulary plus the native table runtime |

Upstream implementations are pinned to commits for reproducibility:

- Topcoat: `a2bd596af2a149f38fcf49570481f356a6cb1069`
- windows-rs / windows-reactor: `845e42a4328ec5b54b97f965798589d997cad177`

## Run

Requirements: Windows 10 version 1809 or later (Windows 11 recommended), Rust 1.95 or later, and Git. The full Mica appearance is available on Windows 11.

```powershell
cargo run -p topcoat-native-demo
```

To launch the debug executable directly:

```powershell
target\debug\topcoat-native-demo.exe
```

At startup, the application displays the directory containing its own executable.

## Verified

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --all -- --check`
- Display and scrolling of a real directory containing 133 items
- Search narrowing the list from 133 items to 2
- Row selection and Open button activation
- Double-click navigation into a child folder
- Back navigation to the original folder
- Mica backdrop, four-column layout, and English modification timestamps

## Current boundaries

This PoC does not replace the operating-system shell. It demonstrates that an Explorer-style native UI and Rust backend can be driven from Topcoat. A production file manager would still need:

1. Copy, move, delete, rename, create, and progress UI
2. A safe mutation model covering the Recycle Bin, Undo, conflicts, and elevation
3. Tabs, context menus, previews, and thumbnails
4. `shell:` namespaces, networking, OneDrive, WSL, and library integration
5. Windows Search, change notifications, and asynchronous enumeration of large folders
6. Windows system icons, file properties, and shell extensions

Unsupported features do not silently fall back to an alternate UI path.

## License

This PoC is licensed under the MIT License. Dependencies remain subject to their respective licenses.
