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
                    title="戻る"
                    aria-label="戻る"
                    disabled=(!snapshot.can_back())
                    @click=$(|_event: Event| explorer.set(explorer.get().go_back()))
                >"←"</button>
                <button
                    class="toolbar"
                    title="進む"
                    aria-label="進む"
                    disabled=(!snapshot.can_forward())
                    @click=$(|_event: Event| explorer.set(explorer.get().go_forward()))
                >"→"</button>
                <button
                    class="toolbar"
                    title="上へ"
                    aria-label="上へ"
                    disabled=(!snapshot.can_up())
                    @click=$(|_event: Event| explorer.set(explorer.get().go_up()))
                >"↑"</button>
                <button
                    class="toolbar"
                    title="最新の情報に更新"
                    aria-label="最新の情報に更新"
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
                    aria-label="現在のフォルダーを検索"
                    @input=$(|event: Event| explorer.set(explorer.get().with_query(event.target.value)))
                >
            </header>

            <section class="flex gap-1">
                <button class="toolbar" disabled="true" title="読み取り専用PoCでは無効">"⊕  新規作成"</button>
                <button class="toolbar" disabled="true" title="読み取り専用PoCでは無効">"✂  切り取り"</button>
                <button class="toolbar" disabled="true" title="読み取り専用PoCでは無効">"▣  コピー"</button>
                <button class="toolbar" disabled="true" title="読み取り専用PoCでは無効">"▤  貼り付け"</button>
                <button class="toolbar" disabled="true" title="読み取り専用PoCでは無効">"✎  名前の変更"</button>
                <button class="toolbar" disabled="true" title="読み取り専用PoCでは無効">"⌫  削除"</button>
                <button
                    class="toolbar"
                    disabled=(snapshot.selected_key().is_empty())
                    title="選択した項目を開く"
                    @click=$(|_event: Event| explorer.set(explorer.get().open_selected()))
                >"↗  開く"</button>
                <button class="toolbar" title="列見出しでも並べ替えできます">"⇅  並べ替え"</button>
                <button class="toolbar" title="詳細表示">"☷  表示"</button>
                <button class="toolbar" title="その他">"•••"</button>
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
