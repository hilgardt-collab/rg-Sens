//! FileEntry GObject subclass for the file picker
//!
//! This provides a GObject with properties that can be used with
//! ColumnView, GridView, and ListStore.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::Path;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FileEntry {
        pub name: RefCell<String>,
        pub path: RefCell<String>,
        pub size: Cell<u64>,
        pub size_display: RefCell<String>,
        pub file_type: RefCell<String>,
        pub modified: Cell<i64>,
        pub modified_display: RefCell<String>,
        pub is_dir: Cell<bool>,
        pub icon_name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileEntry {
        const NAME: &'static str = "RgSensFileEntry";
        type Type = super::FileEntry;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for FileEntry {
        fn properties() -> &'static [glib::ParamSpec] {
            use std::sync::OnceLock;
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("name").build(),
                    glib::ParamSpecString::builder("path").build(),
                    glib::ParamSpecUInt64::builder("size").build(),
                    glib::ParamSpecString::builder("size-display").build(),
                    glib::ParamSpecString::builder("file-type").build(),
                    glib::ParamSpecInt64::builder("modified").build(),
                    glib::ParamSpecString::builder("modified-display").build(),
                    glib::ParamSpecBoolean::builder("is-dir").build(),
                    glib::ParamSpecString::builder("icon-name").build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "name" => *self.name.borrow_mut() = value.get().unwrap_or_default(),
                "path" => *self.path.borrow_mut() = value.get().unwrap_or_default(),
                "size" => self.size.set(value.get().unwrap_or(0)),
                "size-display" => *self.size_display.borrow_mut() = value.get().unwrap_or_default(),
                "file-type" => *self.file_type.borrow_mut() = value.get().unwrap_or_default(),
                "modified" => self.modified.set(value.get().unwrap_or(0)),
                "modified-display" => *self.modified_display.borrow_mut() = value.get().unwrap_or_default(),
                "is-dir" => self.is_dir.set(value.get().unwrap_or(false)),
                "icon-name" => *self.icon_name.borrow_mut() = value.get().unwrap_or_default(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "size" => self.size.get().to_value(),
                "size-display" => self.size_display.borrow().to_value(),
                "file-type" => self.file_type.borrow().to_value(),
                "modified" => self.modified.get().to_value(),
                "modified-display" => self.modified_display.borrow().to_value(),
                "is-dir" => self.is_dir.get().to_value(),
                "icon-name" => self.icon_name.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct FileEntry(ObjectSubclass<imp::FileEntry>);
}

impl Default for FileEntry {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl FileEntry {
    pub fn new(path: &Path) -> Self {
        let entry: Self = glib::Object::new();
        let imp = entry.imp();

        let metadata = std::fs::metadata(path).ok();
        let is_dir = path.is_dir();

        // Name
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        *imp.name.borrow_mut() = name;

        // Path
        *imp.path.borrow_mut() = path.to_string_lossy().to_string();

        // Size
        let (size, size_display) = if is_dir {
            (0, String::new())
        } else {
            let s = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            (s, format_file_size(s))
        };
        imp.size.set(size);
        *imp.size_display.borrow_mut() = size_display;

        // File type
        let file_type = if is_dir {
            "Folder".to_string()
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_uppercase())
                .unwrap_or_else(|| "File".to_string())
        };
        *imp.file_type.borrow_mut() = file_type;

        // Modified time
        let (modified, modified_display) = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let secs = t
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                (secs, format_modified_time(secs))
            })
            .unwrap_or((0, String::new()));
        imp.modified.set(modified);
        *imp.modified_display.borrow_mut() = modified_display;

        // Is directory
        imp.is_dir.set(is_dir);

        // Icon name
        let icon_name = if is_dir {
            "folder-symbolic".to_string()
        } else if is_image_file(path) {
            "image-x-generic-symbolic".to_string()
        } else {
            "text-x-generic-symbolic".to_string()
        };
        *imp.icon_name.borrow_mut() = icon_name;

        entry
    }

    // Accessor methods for convenience
    pub fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub fn path_string(&self) -> String {
        self.imp().path.borrow().clone()
    }

    pub fn path_buf(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.imp().path.borrow().as_str())
    }

    pub fn size(&self) -> u64 {
        self.imp().size.get()
    }

    pub fn size_display(&self) -> String {
        self.imp().size_display.borrow().clone()
    }

    pub fn file_type(&self) -> String {
        self.imp().file_type.borrow().clone()
    }

    pub fn modified(&self) -> i64 {
        self.imp().modified.get()
    }

    pub fn modified_display(&self) -> String {
        self.imp().modified_display.borrow().clone()
    }

    pub fn is_dir(&self) -> bool {
        self.imp().is_dir.get()
    }

    pub fn icon_name(&self) -> String {
        self.imp().icon_name.borrow().clone()
    }
}

/// Check if a path is an image file based on extension
fn is_image_file(path: &Path) -> bool {
    const IMAGE_EXTENSIONS: &[&str] = &[
        "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "tiff", "tif", "ico",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Format file size in human-readable format
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size_f = size as f64;
    let mut unit_idx = 0;

    while size_f >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size_f /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size_f, UNITS[unit_idx])
    }
}

/// Format modified time as human-readable string
fn format_modified_time(timestamp: i64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let time = UNIX_EPOCH + Duration::from_secs(timestamp as u64);
    let now = SystemTime::now();

    // Calculate days ago
    if let Ok(duration) = now.duration_since(time) {
        let secs = duration.as_secs();
        let days = secs / 86400;

        if days == 0 {
            // Today - show time
            let hours = (secs % 86400) / 3600;
            let mins = (secs % 3600) / 60;
            if hours > 0 {
                format!("{} hours ago", hours)
            } else if mins > 0 {
                format!("{} min ago", mins)
            } else {
                "Just now".to_string()
            }
        } else if days == 1 {
            "Yesterday".to_string()
        } else if days < 7 {
            format!("{} days ago", days)
        } else {
            // Show date using chrono-like formatting manually
            // Convert timestamp to date components
            let total_days = timestamp / 86400;
            let (year, month, day) = days_to_ymd(total_days);
            format!("{:04}-{:02}-{:02}", year, month, day)
        }
    } else {
        // Future date (shouldn't happen for files)
        "Unknown".to_string()
    }
}

/// Convert days since epoch to year/month/day
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Simple algorithm for date conversion
    let days = days + 719468; // Days from year 0 to epoch
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}
