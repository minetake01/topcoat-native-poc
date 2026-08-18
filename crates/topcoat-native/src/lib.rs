//! Minimal native runtime for the Topcoat-to-WinUI proof of concept.
//!
//! Parsing belongs to Topcoat itself (`topcoat-view-grammar`). This crate only
//! supplies the state and event vocabulary needed by the WinUI lowering.

pub use topcoat_native_macro::view;
pub use windows_reactor;

use windows_reactor::SetState;

/// Renderer-neutral row data consumed by the native `<table>` element.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableRow {
    pub key: String,
    pub name: String,
    pub modified: String,
    pub kind: String,
    pub size: String,
    pub icon: String,
}

impl TableRow {
    pub fn new(
        key: impl Into<String>,
        name: impl Into<String>,
        modified: impl Into<String>,
        kind: impl Into<String>,
        size: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            modified: modified.into(),
            kind: kind.into(),
            size: size.into(),
            icon: icon.into(),
        }
    }
}

/// Builds the WinUI details view used by Topcoat's native `<table>` mapping.
#[doc(hidden)]
pub struct NativeTableProps {
    pub key: String,
    pub rows: Vec<TableRow>,
    pub selected_key: String,
    pub width: f64,
    pub height: f64,
}

/// Builds the WinUI details view used by Topcoat's native `<table>` mapping.
#[doc(hidden)]
pub fn native_table(
    props: NativeTableProps,
    on_select: impl windows_reactor::IntoCallback<String>,
    on_activate: impl windows_reactor::IntoCallback<String>,
    on_sort: impl windows_reactor::IntoCallback<String>,
) -> windows_reactor::Element {
    use std::cell::RefCell;
    use std::time::{Duration, Instant};
    use windows_reactor::{
        AccessibilityExt as _, BackgroundExt as _, GridChildExt as _, GridLength,
        HorizontalAlignment, InputExt as _, LayoutExt as _, PaddingExt as _, SelectionMode,
        TextTrimming, ThemeRef, VerticalAlignment, button, grid, list_view, text_block, vstack,
    };

    let on_select = on_select.into_callback();
    let on_activate = on_activate.into_callback();
    let on_sort = on_sort.into_callback();
    let NativeTableProps {
        key: table_key,
        rows,
        selected_key,
        width,
        height,
    } = props;

    thread_local! {
        static LAST_ACTIVATION_TAP: RefCell<Option<(String, Instant)>> = const { RefCell::new(None) };
    }

    fn header_cell(
        label: &'static str,
        key: &'static str,
        column: i32,
        on_sort: windows_reactor::Callback<String>,
    ) -> windows_reactor::Element {
        button(label)
            .subtle()
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch)
            .grid_column(column)
            .on_click(move || on_sort.invoke(key.to_owned()))
            .automation_name(format!("Sort by {label}"))
            .into()
    }

    let columns = [
        GridLength::Star(1.0),
        GridLength::Pixel(170.0),
        GridLength::Pixel(160.0),
        GridLength::Pixel(110.0),
    ];

    let header = grid([
        header_cell("Name", "name", 0, on_sort.clone()),
        header_cell("Date modified", "modified", 1, on_sort.clone()),
        header_cell("Type", "kind", 2, on_sort.clone()),
        header_cell("Size", "size", 3, on_sort),
    ])
    .columns(columns)
    .height(36.0)
    .background(ThemeRef::CardBackground);

    let selected_index = rows
        .iter()
        .position(|row| row.key == selected_key)
        .map_or(-1, |index| index as i32);
    let selection_rows = rows.clone();
    let selection_callback = on_select.clone();
    let activation_callback = on_activate.clone();

    let list = list_view(rows, move |row: &TableRow, _index| {
        let key = row.key.clone();
        let activate = activation_callback.clone();
        let name = format!("{}  {}", row.icon, row.name);
        let automation_name = format!("{}、{}、{}、{}", row.name, row.modified, row.kind, row.size);

        grid([
            windows_reactor::Element::from(
                text_block(name)
                    .text_trimming(TextTrimming::CharacterEllipsis)
                    .vertical_alignment(VerticalAlignment::Center)
                    .grid_column(0),
            ),
            windows_reactor::Element::from(
                text_block(row.modified.clone())
                    .vertical_alignment(VerticalAlignment::Center)
                    .grid_column(1),
            ),
            windows_reactor::Element::from(
                text_block(row.kind.clone())
                    .text_trimming(TextTrimming::CharacterEllipsis)
                    .vertical_alignment(VerticalAlignment::Center)
                    .grid_column(2),
            ),
            windows_reactor::Element::from(
                text_block(row.size.clone())
                    .vertical_alignment(VerticalAlignment::Center)
                    .grid_column(3),
            ),
        ])
        .columns(columns)
        .height(34.0)
        .padding(windows_reactor::Thickness::xy(8.0, 2.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .automation_name(automation_name)
        .on_pointer_pressed(move |_pointer| {
            let should_activate = LAST_ACTIVATION_TAP.with(|last| {
                let mut last = last.borrow_mut();
                let now = Instant::now();
                let activate = last.as_ref().is_some_and(|(last_key, at)| {
                    last_key == &key && now.duration_since(*at) <= Duration::from_millis(650)
                });
                *last = if activate {
                    None
                } else {
                    Some((key.clone(), now))
                };
                activate
            });
            if should_activate {
                activate.invoke(key.clone());
            }
        })
    })
    .with_key_selector(|row| row.key.clone())
    .with_key(table_key)
    .selected_index(selected_index)
    .selection_mode(SelectionMode::Single)
    .on_selection_changed(move |index| {
        if let Some(row) = usize::try_from(index)
            .ok()
            .and_then(|index| selection_rows.get(index))
        {
            selection_callback.invoke(row.key.clone());
        }
    })
    .width(width.max(480.0))
    .height((height - 36.0).max(180.0));

    vstack([
        windows_reactor::Element::from(header),
        windows_reactor::Element::from(list),
    ])
    .width(width.max(480.0))
    .height(height.max(216.0))
    .automation_name("File list")
    .into()
}

/// A Topcoat-shaped reactive signal backed by a `windows-reactor` state slot.
#[derive(Clone)]
pub struct Signal<T>
where
    T: Clone + PartialEq + 'static,
{
    current: T,
    setter: SetState<T>,
}

impl<T> Signal<T>
where
    T: Clone + PartialEq + 'static,
{
    #[doc(hidden)]
    pub fn new(current: T, setter: SetState<T>) -> Self {
        Self { current, setter }
    }

    /// Returns the value from the current render pass.
    pub fn get(&self) -> T {
        self.current.clone()
    }

    /// Schedules a state update and a WinUI tree reconciliation.
    pub fn set(&self, value: T) {
        self.setter.call(value);
    }
}

/// Event value supplied to Topcoat-style event closures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub target: EventTarget,
}

impl Event {
    #[doc(hidden)]
    pub fn click() -> Self {
        Self::default()
    }

    #[doc(hidden)]
    pub fn input(value: String) -> Self {
        Self {
            target: EventTarget { value },
        }
    }
}

/// The portable subset of a DOM event target used by this PoC.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventTarget {
    pub value: String,
}

pub mod prelude {
    pub use crate::{Event, Signal, TableRow, view};
    pub use windows_reactor::{App, Backdrop, Element, RenderCx, Result};
}
