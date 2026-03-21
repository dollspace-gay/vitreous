use std::path::PathBuf;

use vitreous::{
    App, Dimension, FontWeight, Node, Theme, button, container, create_signal, divider, for_each,
    h_stack, scroll_view, show, spacer, text, theme, v_stack,
};

#[derive(Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

fn root() -> Node {
    let t = theme();
    let current_path = create_signal(home_dir());
    let entries = create_signal(list_directory(&home_dir()));
    let selected = create_signal(Option::<PathBuf>::None);

    v_stack((
        // Breadcrumb bar
        breadcrumbs(current_path, entries),
        divider(),
        // Main content
        h_stack((
            // File list
            scroll_view(for_each(
                entries.get(),
                |entry| entry.path.to_string_lossy().to_string(),
                move |entry| {
                    let path = entry.path.clone();
                    let is_dir = entry.is_dir;
                    let is_selected = selected.get().as_ref() == Some(&path);

                    file_row(entry, is_selected).on_click(move || {
                        if is_dir {
                            current_path.set(path.clone());
                            entries.set(list_directory(&path));
                            selected.set(None);
                        } else {
                            selected.set(Some(path.clone()));
                        }
                    })
                },
            ))
            .flex_grow(1.0),
            // Detail sidebar
            show(selected.get().is_some(), move || {
                detail_panel(selected.get().unwrap(), &entries.get())
            }),
        ))
        .flex_grow(1.0),
        // Status bar
        divider(),
        status_bar(entries),
    ))
    .background(t.background)
}

fn breadcrumbs(
    current_path: vitreous::Signal<PathBuf>,
    entries: vitreous::Signal<Vec<FileEntry>>,
) -> Node {
    let t = theme();
    let path = current_path.get();
    let components: Vec<(String, PathBuf)> = path
        .ancestors()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "/".to_owned());
            (name, p.to_path_buf())
        })
        .collect();

    h_stack(for_each(
        components,
        |c| c.1.to_string_lossy().to_string(),
        move |c| {
            let crumb_path = c.1.clone();
            h_stack((
                button(c.0.as_str())
                    .on_click(move || {
                        current_path.set(crumb_path.clone());
                        entries.set(list_directory(&crumb_path));
                    })
                    .font_size(t.font_size_sm)
                    .padding(t.spacing_xs),
                text("/").font_size(t.font_size_sm),
            ))
            .gap(t.spacing_xs)
        },
    ))
    .padding(t.spacing_sm)
    .background(t.surface)
}

fn file_row(entry: &FileEntry, is_selected: bool) -> Node {
    let t = theme();
    let icon = if entry.is_dir { "📁" } else { "📄" };
    let size_str = if entry.is_dir {
        "--".to_owned()
    } else {
        format_size(entry.size)
    };

    h_stack((
        text(icon).width(Dimension::Px(24.0)),
        text(entry.name.as_str())
            .flex_grow(1.0)
            .font_weight(if entry.is_dir {
                FontWeight::Bold
            } else {
                FontWeight::Regular
            }),
        text(size_str).font_size(t.font_size_sm),
    ))
    .gap(t.spacing_sm)
    .padding(t.spacing_sm)
    .background(if is_selected {
        t.primary_variant
    } else {
        t.background
    })
}

fn detail_panel(path: PathBuf, entries: &[FileEntry]) -> Node {
    let t = theme();
    let entry = entries.iter().find(|e| e.path == path);

    match entry {
        Some(e) => v_stack((
            text(e.name.as_str())
                .font_size(t.font_size_lg)
                .font_weight(FontWeight::Bold),
            divider(),
            detail_row("Type", if e.is_dir { "Directory" } else { "File" }),
            detail_row("Size", &format_size(e.size)),
            detail_row("Path", &e.path.to_string_lossy()),
        ))
        .gap(t.spacing_sm)
        .padding(t.spacing_md)
        .background(t.surface)
        .width(Dimension::Px(250.0)),
        None => container(text("Select a file")).padding(t.spacing_md),
    }
}

fn detail_row(label: &str, value: &str) -> Node {
    let t = theme();
    v_stack((
        text(label)
            .font_size(t.font_size_xs)
            .font_weight(FontWeight::Bold),
        text(value).font_size(t.font_size_sm),
    ))
    .gap(t.spacing_xs)
}

fn status_bar(entries: vitreous::Signal<Vec<FileEntry>>) -> Node {
    let t = theme();
    let items = entries.get();
    let dirs = items.iter().filter(|e| e.is_dir).count();
    let files = items.iter().filter(|e| !e.is_dir).count();
    let total_size: u64 = items.iter().map(|e| e.size).sum();

    h_stack((
        text(format!("{dirs} folders, {files} files")),
        spacer(),
        text(format!("Total: {}", format_size(total_size))),
    ))
    .padding(t.spacing_sm)
    .background(t.surface)
    .font_size(t.font_size_xs)
}

// --- Helpers ---

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

fn list_directory(path: &PathBuf) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let meta = entry.metadata().ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files
            if name.starts_with('.') {
                continue;
            }

            entries.push(FileEntry {
                name,
                path: entry.path(),
                is_dir,
                size,
            });
        }
    }

    // Directories first, then alphabetically
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    entries
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn main() {
    App::new()
        .title("Vitreous File Explorer")
        .size(800, 600)
        .min_size(600, 400)
        .theme(Theme::light())
        .run(root);
}
