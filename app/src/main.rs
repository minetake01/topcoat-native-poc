#![windows_subsystem = "windows"]

use topcoat_native::prelude::*;

fn app(cx: &mut RenderCx) -> Element {
    view! { cx =>
        signal count = 0_i32;
        signal name = String::from("Topcoat");

        <main class="p-6 space-y-4">
            <h1>"Topcoat → native WinUI 3"</h1>
            <p>"The source keeps Topcoat's view syntax; only the renderer changed."</p>

            <input
                type="text"
                :value=$(name.get())
                placeholder="Your name"
                aria-label="Your name"
                @input=$(|event: Event| name.set(event.target.value))
            >

            <p>(format!("Hello, {}. Count: {}", name.get(), count.get()))</p>

            if count.get() >= 3 {
                <p>"The conditional branch is rendered by Rust and reconciled by WinUI."</p>
            }

            <div class="flex gap-2">
                <button
                    type="button"
                    class="primary"
                    id="increment-button"
                    @click=$(|_event: Event| count.set(count.get() + 1))
                >
                    "Increment"
                </button>
                <button
                    type="button"
                    class="secondary"
                    @click=$(|_event: Event| count.set(0))
                >
                    "Reset"
                </button>
            </div>
        </main>
    }
}

fn main() -> Result<()> {
    App::new()
        .title("Topcoat Native PoC")
        .inner_size(720.0, 480.0)
        .render(app)
}
