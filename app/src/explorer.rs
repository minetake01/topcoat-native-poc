use std::ffi::{OsStr, c_void};
use std::fs::{self, Metadata};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use topcoat_native::TableRow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub label: String,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortColumn {
    Name,
    Modified,
    Kind,
    Size,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileEntry {
    path: PathBuf,
    name: String,
    modified_ticks: u64,
    modified: String,
    kind: String,
    size: u64,
    size_display: String,
    icon: String,
    is_dir: bool,
}

impl FileEntry {
    fn table_row(&self) -> TableRow {
        TableRow::new(
            path_key(&self.path),
            &self.name,
            &self.modified,
            &self.kind,
            &self.size_display,
            &self.icon,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorerState {
    current: PathBuf,
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
    entries: Vec<FileEntry>,
    query: String,
    selected_key: String,
    sort: SortColumn,
    ascending: bool,
    notice: String,
}

impl ExplorerState {
    pub fn initial() -> Self {
        let path = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .expect("the Explorer PoC requires an executable directory");
        let entries = read_directory(&path).expect("the executable directory must be readable");
        Self {
            current: path,
            back: Vec::new(),
            forward: Vec::new(),
            entries,
            query: String::new(),
            selected_key: String::new(),
            sort: SortColumn::Name,
            ascending: true,
            notice: "Read-only mode".to_owned(),
        }
    }

    pub fn title(&self) -> String {
        self.current
            .file_name()
            .and_then(OsStr::to_str)
            .filter(|name| !name.is_empty())
            .map_or_else(
                || self.current.to_string_lossy().into_owned(),
                str::to_owned,
            )
    }

    pub fn current_display(&self) -> String {
        self.current.to_string_lossy().into_owned()
    }

    pub fn search_placeholder(&self) -> String {
        format!("Search {}", self.title())
    }

    pub fn query(&self) -> String {
        self.query.clone()
    }

    pub fn selected_key(&self) -> String {
        self.selected_key.clone()
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn can_up(&self) -> bool {
        self.current.parent().is_some()
    }

    pub fn breadcrumbs(&self) -> Vec<Location> {
        let mut result = Vec::new();
        let mut built = PathBuf::new();
        for component in self.current.components() {
            built.push(component.as_os_str());
            match component {
                Component::Prefix(prefix) => result.push(Location {
                    label: prefix.as_os_str().to_string_lossy().into_owned(),
                    path: built.clone(),
                }),
                Component::Normal(name) => result.push(Location {
                    label: name.to_string_lossy().into_owned(),
                    path: built.clone(),
                }),
                Component::RootDir | Component::CurDir | Component::ParentDir => {}
            }
        }
        if result.len() > 6 {
            let tail_start = result.len() - 5;
            let mut compact = vec![result[0].clone()];
            compact.extend(result.into_iter().skip(tail_start));
            compact
        } else {
            result
        }
    }

    pub fn sidebar_locations(&self) -> Vec<Location> {
        let mut locations = Vec::new();
        if let Some(profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            for (label, child) in [
                ("🏠 Home", None),
                ("🖼 Pictures", Some("Pictures")),
                ("📄 Documents", Some("Documents")),
                ("⬇ Downloads", Some("Downloads")),
            ] {
                let path = child.map_or_else(|| profile.clone(), |child| profile.join(child));
                if path.is_dir() {
                    locations.push(Location {
                        label: label.to_owned(),
                        path,
                    });
                }
            }
        }

        locations.push(Location {
            label: "💻 PC".to_owned(),
            path: self.current.clone(),
        });
        for letter in b'A'..=b'Z' {
            let path = PathBuf::from(format!("{}:\\", char::from(letter)));
            if path.is_dir() {
                locations.push(Location {
                    label: format!("▣ {}:", char::from(letter)),
                    path,
                });
            }
        }
        locations
    }

    pub fn rows(&self) -> Vec<TableRow> {
        let query = self.query.to_lowercase();
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| query.is_empty() || entry.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        entries.sort_by(|left, right| {
            let folders_first = right.is_dir.cmp(&left.is_dir);
            if folders_first != std::cmp::Ordering::Equal {
                return folders_first;
            }
            let ordering = match self.sort {
                SortColumn::Name => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
                SortColumn::Modified => left.modified_ticks.cmp(&right.modified_ticks),
                SortColumn::Kind => left.kind.cmp(&right.kind),
                SortColumn::Size => left.size.cmp(&right.size),
            };
            if self.ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
        entries.iter().map(FileEntry::table_row).collect()
    }

    pub fn status(&self) -> String {
        let visible = self.rows().len();
        let selection = if self.selected_key.is_empty() {
            String::new()
        } else {
            "  |  1 item selected".to_owned()
        };
        format!("{visible} items{selection}  |  {}", self.notice)
    }

    pub fn with_query(mut self, query: String) -> Self {
        if self.query == query {
            return self;
        }
        self.query = query;
        self.selected_key.clear();
        self.notice = "Showing search results".to_owned();
        self
    }

    pub fn select(mut self, key: String) -> Self {
        if !self
            .entries
            .iter()
            .any(|entry| path_key(&entry.path) == key)
        {
            return self;
        }
        self.selected_key = key;
        self.notice = "Item selected".to_owned();
        self
    }

    pub fn navigate(mut self, path: PathBuf) -> Self {
        self.change_directory(path, true);
        self
    }

    pub fn go_back(mut self) -> Self {
        if let Some(path) = self.back.pop() {
            self.forward.push(self.current.clone());
            self.change_directory(path, false);
        }
        self
    }

    pub fn go_forward(mut self) -> Self {
        if let Some(path) = self.forward.pop() {
            self.back.push(self.current.clone());
            self.change_directory(path, false);
        }
        self
    }

    pub fn go_up(mut self) -> Self {
        if let Some(parent) = self.current.parent().map(Path::to_path_buf) {
            self.change_directory(parent, true);
        }
        self
    }

    pub fn refresh(mut self) -> Self {
        match read_directory(&self.current) {
            Ok(entries) => {
                self.entries = entries;
                self.selected_key.clear();
                self.notice = "Refreshed".to_owned();
            }
            Err(error) => self.notice = error,
        }
        self
    }

    pub fn sort_by(mut self, key: String) -> Self {
        let next = match key.as_str() {
            "name" => SortColumn::Name,
            "modified" => SortColumn::Modified,
            "kind" => SortColumn::Kind,
            "size" => SortColumn::Size,
            _ => return self,
        };
        if self.sort == next {
            self.ascending = !self.ascending;
        } else {
            self.sort = next;
            self.ascending = true;
        }
        self.notice = if self.ascending {
            "Sorted ascending"
        } else {
            "Sorted descending"
        }
        .to_owned();
        self
    }

    pub fn activate(mut self, key: String) -> Self {
        let path = PathBuf::from(&key);
        if path.is_dir() {
            self.change_directory(path, true);
        } else if path.is_file() {
            self.notice = match open_with_shell(&path) {
                Ok(()) => format!("Opened {}", path.display()),
                Err(error) => error,
            };
        } else {
            self.notice = "The selected item no longer exists".to_owned();
        }
        self
    }

    pub fn open_selected(self) -> Self {
        if self.selected_key.is_empty() {
            let mut next = self;
            next.notice = "Select an item to open".to_owned();
            next
        } else {
            let key = self.selected_key.clone();
            self.activate(key)
        }
    }

    fn change_directory(&mut self, path: PathBuf, remember: bool) {
        let canonical = match fs::canonicalize(&path) {
            Ok(path) => user_facing_path(path),
            Err(error) => {
                self.notice = format!("Cannot open {}: {error}", path.display());
                return;
            }
        };
        match read_directory(&canonical) {
            Ok(entries) => {
                if remember && canonical != self.current {
                    self.back.push(self.current.clone());
                    self.forward.clear();
                }
                self.current = canonical;
                self.entries = entries;
                self.query.clear();
                self.selected_key.clear();
                self.notice = "Folder opened".to_owned();
            }
            Err(error) => self.notice = error,
        }
    }
}

fn user_facing_path(path: PathBuf) -> PathBuf {
    let display = path.to_string_lossy();
    if let Some(rest) = display.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = display.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

fn read_directory(path: &Path) -> Result<Vec<FileEntry>, String> {
    let reader =
        fs::read_dir(path).map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
    let mut entries = Vec::new();
    for item in reader {
        let Ok(item) = item else { continue };
        let Ok(metadata) = item.metadata() else {
            continue;
        };
        let path = item.path();
        let name = item.file_name().to_string_lossy().into_owned();
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        entries.push(FileEntry {
            modified_ticks: metadata.last_write_time(),
            modified: format_modified(&metadata),
            kind: file_kind(&path, is_dir),
            size_display: if is_dir {
                String::new()
            } else {
                format_size(size)
            },
            icon: file_icon(&path, is_dir),
            path,
            name,
            size,
            is_dir,
        });
    }
    Ok(entries)
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn file_kind(path: &Path, is_dir: bool) -> String {
    if is_dir {
        return "File folder".to_owned();
    }
    match path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "exe" => "Application",
        "dll" => "Application extension",
        "rs" => "Rust source file",
        "toml" => "TOML file",
        "md" => "Markdown file",
        "txt" => "Text document",
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "Image file",
        "zip" | "7z" | "rar" => "Compressed archive",
        "json" => "JSON file",
        _ => "File",
    }
    .to_owned()
}

fn file_icon(path: &Path, is_dir: bool) -> String {
    if is_dir {
        return "📁".to_owned();
    }
    match path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "🖼",
        "zip" | "7z" | "rar" => "🗜",
        "exe" => "▣",
        _ => "📄",
    }
    .to_owned()
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{} KB", bytes.div_ceil(KB))
    } else {
        format!("{bytes} B")
    }
}

#[repr(C)]
struct FileTime {
    low_date_time: u32,
    high_date_time: u32,
}

#[repr(C)]
#[derive(Default)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn FileTimeToLocalFileTime(file_time: *const FileTime, local_file_time: *mut FileTime) -> i32;
    fn FileTimeToSystemTime(file_time: *const FileTime, system_time: *mut SystemTime) -> i32;
}

fn format_modified(metadata: &Metadata) -> String {
    let ticks = metadata.last_write_time();
    let utc = FileTime {
        low_date_time: ticks as u32,
        high_date_time: (ticks >> 32) as u32,
    };
    let mut local = FileTime {
        low_date_time: 0,
        high_date_time: 0,
    };
    let mut value = SystemTime::default();
    // SAFETY: both functions receive valid pointers to correctly laid out Win32 structures.
    let converted = unsafe {
        FileTimeToLocalFileTime(&utc, &mut local) != 0
            && FileTimeToSystemTime(&local, &mut value) != 0
    };
    if converted {
        format!(
            "{:04}/{:02}/{:02} {:02}:{:02}",
            value.year, value.month, value.day, value.hour, value.minute
        )
    } else {
        String::new()
    }
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        window: *mut c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show_command: i32,
    ) -> isize;
}

fn open_with_shell(path: &Path) -> Result<(), String> {
    let operation: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: the string buffers are NUL-terminated and live for the duration of the call.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    if result > 32 {
        Ok(())
    } else {
        Err(format!(
            "Cannot open {} with its associated application",
            path.display()
        ))
    }
}
