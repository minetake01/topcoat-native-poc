#![windows_subsystem = "windows"]

mod explorer;

use explorer::ExplorerState;
use topcoat_native::prelude::*;

fn app(cx: &mut RenderCx) -> Element {
    let window = cx.use_inner_size();

    view! { cx =>
        signal explorer = ExplorerState::initial();
        let snapshot = explorer.get();
        let table_width = (window.width - 224.0).max(640.0);
        let table_height = (window.height - 174.0).max(320.0);

        <main class="p-3 space-y-2">
            <header class="flex gap-2">
                <button
                    class="toolbar"
                    title="Back"
                    aria-label="Back"
                    disabled=(!snapshot.can_back())
                    @click=$(|_event: Event| explorer.set(explorer.get().go_back()))
                >"←"</button>
                <button
                    class="toolbar"
                    title="Forward"
                    aria-label="Forward"
                    disabled=(!snapshot.can_forward())
                    @click=$(|_event: Event| explorer.set(explorer.get().go_forward()))
                >"→"</button>
                <button
                    class="toolbar"
                    title="Up"
                    aria-label="Up"
                    disabled=(!snapshot.can_up())
                    @click=$(|_event: Event| explorer.set(explorer.get().go_up()))
                >"↑"</button>
                <button
                    class="toolbar"
                    title="Refresh"
                    aria-label="Refresh"
                    @click=$(|_event: Event| explorer.set(explorer.get().refresh()))
                >"↻"</button>

                <nav class="flex gap-1">
                    for location in snapshot.breadcrumbs() {
                        let location_path = location.path.clone();
                        let location_title = location.path.to_string_lossy().into_owned();
                        <button
                            class="nav"
                            title=(location_title)
                            @click=$(|_event: Event| explorer.set(explorer.get().navigate(location_path.clone())))
                        >
                            (format!("{}  ›", location.label))
                        </button>
                    }
                </nav>

                <input
                    type="text"
                    class="w-60"
                    :value=$(snapshot.query())
                    placeholder=(snapshot.search_placeholder())
                    aria-label="Search the current folder"
                    @input=$(|event: Event| explorer.set(explorer.get().with_query(event.target.value)))
                >
            </header>

            <section class="flex gap-1">
                <button class="toolbar" disabled="true" title="Unavailable in this read-only PoC">"⊕  New"</button>
                <button class="toolbar" disabled="true" title="Unavailable in this read-only PoC">"✂  Cut"</button>
                <button class="toolbar" disabled="true" title="Unavailable in this read-only PoC">"▣  Copy"</button>
                <button class="toolbar" disabled="true" title="Unavailable in this read-only PoC">"▤  Paste"</button>
                <button class="toolbar" disabled="true" title="Unavailable in this read-only PoC">"✎  Rename"</button>
                <button class="toolbar" disabled="true" title="Unavailable in this read-only PoC">"⌫  Delete"</button>
                <button
                    class="toolbar"
                    disabled=(snapshot.selected_key().is_empty())
                    title="Open the selected item"
                    @click=$(|_event: Event| explorer.set(explorer.get().open_selected()))
                >"↗  Open"</button>
                <button class="toolbar" title="You can also sort with the column headers">"⇅  Sort"</button>
                <button class="toolbar" title="Details view">"☷  View"</button>
                <button class="toolbar" title="More">"•••"</button>
            </section>

            <section class="flex gap-3">
                <nav class="w-48 space-y-1">
                    for location in snapshot.sidebar_locations() {
                        let location_path = location.path.clone();
                        let location_title = location.path.to_string_lossy().into_owned();
                        <button
                            class="nav w-44"
                            title=(location_title)
                            @click=$(|_event: Event| explorer.set(explorer.get().navigate(location_path.clone())))
                        >
                            (location.label)
                        </button>
                    }
                </nav>

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
            </section>

            <footer class="flex gap-3">
                <p>(snapshot.status())</p>
                <p>(snapshot.current_display())</p>
            </footer>
        </main>
    }
}

fn main() -> Result<()> {
    App::new()
        .title("Topcoat Explorer")
        .backdrop(Backdrop::Mica)
        .inner_size(1120.0, 700.0)
        .render(app)
}
