//! Shared hwmon sensor discovery helpers used by the fan speed and system
//! temperature sources.
//!
//! Identity rules (both sources follow these):
//! - A sensor is identified in saved configs by its **label**. Labels are made
//!   unique at discovery; when two sensors share a label (two identical NVMe
//!   drives, two USB controllers of the same chipset) a suffix is added, built
//!   from progressively more specific device identity: the PCI address, then
//!   the leaf device (distinguishes drives behind one controller), then the
//!   channel, e.g. `prom21_xhci temp1 (0000:11:00.0)`.
//! - The discovered list is sorted by (driver name, device, channel) so
//!   dropdown order and indices are deterministic. sysfs enumeration order is
//!   not: `read_dir` returns hash order and hwmon numbering changes per boot.
//! - `sensor_index` in a config is only consulted when the config has no label
//!   (saved before labels existed).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One `{prefix}N_input` sensor found under /sys/class/hwmon
#[derive(Debug, Clone)]
pub(crate) struct HwmonSensor {
    /// Driver name from the hwmon `name` file (falls back to the directory name)
    pub name: String,
    /// Stable identity of the device behind the hwmon directory: its PCI
    /// address, else its sysfs device name (`it87.2656`); empty when unknown
    pub device_id: String,
    /// Last component of the device path when it is more specific than
    /// `device_id` (`0:0:0:0` for a SATA drive behind an AHCI controller,
    /// `3-1:1.0` for a USB sensor); empty otherwise
    pub device_leaf: String,
    /// `temp1` / `fan2`
    pub channel: String,
    /// The N in `tempN_input` / `fanN_input`
    pub number: u32,
    /// Human-readable label; unique within a finalized list
    pub label: String,
    /// Label before collision disambiguation (equal to `label` unless it collided)
    pub base_label: String,
    /// `*_input` file to read (None for sensors read through sysinfo on non-Linux)
    pub path: Option<PathBuf>,
    /// Position in the raw discovery order, assigned by `finalize`
    pub raw_pos: usize,
}

/// Something that can be looked up by label (`FanInfo` / `SensorInfo`)
pub(crate) trait SensorIdentity {
    fn label(&self) -> &str;
    fn base_label(&self) -> &str;
}

/// One /sys/class/hwmon/hwmonN directory, with the per-directory files read once
#[cfg(target_os = "linux")]
pub(crate) struct HwmonDir {
    pub path: PathBuf,
    /// Driver name (`name` file), falling back to the directory name
    pub name: String,
    /// `device/model` when present (drives expose it)
    pub model: Option<String>,
}

/// Read a sysfs text file with trailing whitespace removed. This matches how
/// sysinfo reads `name`, `*_label` and `device/model`, so temperature labels
/// stay byte-identical to the ones older configs saved.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn read_trimmed(path: &Path) -> Option<String> {
    let s = std::fs::read_to_string(path).ok()?;
    Some(s.trim_end().to_string())
}

/// Read an integer sysfs value (`fan1_input`, `temp1_input`)
pub(crate) fn read_i64(path: &Path) -> Option<i64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Parse N out of `"{prefix}N_input"`; None for anything else
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn sensor_number(filename: &str, prefix: &str) -> Option<u32> {
    let rest = filename.strip_prefix(prefix)?;
    let num = rest.strip_suffix("_input")?;
    num.parse().ok()
}

/// `0000:02:00.0` style PCI bus/device/function
pub(crate) fn is_pci_bdf(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 12
        && b[4] == b':'
        && b[7] == b':'
        && b[10] == b'.'
        && [0, 1, 2, 3, 5, 6, 8, 9]
            .iter()
            .all(|&i| b[i].is_ascii_hexdigit())
        && b[11].is_ascii_digit()
}

/// Split a resolved sysfs device path into (device id, leaf).
///
/// The id is the last PCI address in the path (stable across boots for fixed
/// hardware, unlike `nvme0`/`phy0`/`hwmonN` numbering); the leaf is the final
/// component when the path continues past that address (a drive behind an
/// AHCI controller, a USB device behind an xHCI controller). Without a PCI
/// address the id is the final component itself (`it87.2656`).
pub(crate) fn parse_device_path(real: &Path) -> (String, String) {
    let parts: Vec<&str> = real.iter().filter_map(|c| c.to_str()).collect();
    match parts.iter().rposition(|p| is_pci_bdf(p)) {
        Some(pos) => {
            let leaf = if pos + 1 < parts.len() {
                parts[parts.len() - 1]
            } else {
                ""
            };
            (parts[pos].to_string(), leaf.to_string())
        }
        None => (
            parts.last().copied().unwrap_or("").to_string(),
            String::new(),
        ),
    }
}

/// Device identity of an hwmon directory; both parts empty when the
/// directory has no `device` link (virtual drivers). The hwmon directory
/// name itself is never used: it is renumbered every boot.
#[cfg(target_os = "linux")]
fn device_identity(hwmon_dir: &Path) -> (String, String) {
    match std::fs::canonicalize(hwmon_dir.join("device")) {
        Ok(real) => parse_device_path(&real),
        Err(_) => (String::new(), String::new()),
    }
}

#[cfg(target_os = "linux")]
fn dir_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Scan /sys/class/hwmon for `{prefix}N_input` files, in raw sysfs order.
/// `label_for(dir, N)` builds the human-readable label.
#[cfg(target_os = "linux")]
pub(crate) fn scan<F>(prefix: &str, label_for: F) -> Vec<HwmonSensor>
where
    F: Fn(&HwmonDir, u32) -> String,
{
    let mut sensors = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return sensors;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(files) = std::fs::read_dir(&path) else {
            continue;
        };
        let channels: Vec<(u32, PathBuf)> = files
            .flatten()
            .filter_map(|f| {
                let number = sensor_number(&f.file_name().to_string_lossy(), prefix)?;
                Some((number, f.path()))
            })
            .collect();
        if channels.is_empty() {
            continue;
        }

        // Per-directory files are read once, not once per channel
        let name = read_trimmed(&path.join("name"))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| dir_name(&path));
        let model = read_trimmed(&path.join("device/model"));
        let (device_id, device_leaf) = device_identity(&path);
        let dir = HwmonDir { path, name, model };

        for (number, input) in channels {
            let label = label_for(&dir, number);
            sensors.push(HwmonSensor {
                name: dir.name.clone(),
                device_id: device_id.clone(),
                device_leaf: device_leaf.clone(),
                channel: format!("{}{}", prefix, number),
                number,
                base_label: label.clone(),
                label,
                path: Some(input),
                raw_pos: 0,
            });
        }
    }
    sensors
}

/// Fallback for boards without hwmon temperature sensors (e.g. Raspberry Pi):
/// one sensor per /sys/class/thermal/thermal_zoneN, labelled by its `type`.
#[cfg(target_os = "linux")]
pub(crate) fn scan_thermal_zones() -> Vec<HwmonSensor> {
    let mut sensors = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/thermal") else {
        return sensors;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let zone = dir_name(&dir);
        let Some(number) = zone
            .strip_prefix("thermal_zone")
            .and_then(|n| n.parse().ok())
        else {
            continue;
        };
        let temp = dir.join("temp");
        if !temp.exists() {
            continue;
        }
        let name = read_trimmed(&dir.join("type"))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| zone.clone());
        sensors.push(HwmonSensor {
            name: name.clone(),
            device_id: zone.clone(),
            device_leaf: String::new(),
            channel: zone,
            number,
            base_label: name.clone(),
            label: name,
            path: Some(temp),
            raw_pos: 0,
        });
    }
    sensors
}

/// Sort by (driver name, device, channel number), make labels unique and
/// return the sorted list plus a map from raw discovery position to sorted
/// index (used to interpret index-only configs saved before labels existed).
pub(crate) fn finalize(mut sensors: Vec<HwmonSensor>) -> (Vec<HwmonSensor>, Vec<usize>) {
    for (i, s) in sensors.iter_mut().enumerate() {
        s.raw_pos = i;
    }
    sensors.sort_by(|a, b| {
        (
            a.name.as_str(),
            a.device_id.as_str(),
            a.device_leaf.as_str(),
            a.number,
        )
            .cmp(&(
                b.name.as_str(),
                b.device_id.as_str(),
                b.device_leaf.as_str(),
                b.number,
            ))
    });
    let mut raw_order = vec![0; sensors.len()];
    for (sorted_idx, s) in sensors.iter().enumerate() {
        raw_order[s.raw_pos] = sorted_idx;
    }
    disambiguate_labels(&mut sensors);
    (sensors, raw_order)
}

/// Make every label unique. Labels that occur more than once get a suffix of
/// progressively more specific identity parts: device id, then device leaf,
/// then channel, each added only while the label still collides and only when
/// the part is non-empty. Sensors that are indistinguishable even then get
/// their raw discovery position, which is not stable across boots; such a
/// saved label is matched back through `strip_raw_suffix`.
pub(crate) fn disambiguate_labels(sensors: &mut [HwmonSensor]) {
    fn colliding(sensors: &[HwmonSensor]) -> Vec<bool> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for s in sensors {
            *counts.entry(s.label.as_str()).or_insert(0) += 1;
        }
        sensors
            .iter()
            .map(|s| counts[s.label.as_str()] > 1)
            .collect()
    }

    let tiers: [fn(&HwmonSensor) -> &str; 3] =
        [|s| &s.device_id, |s| &s.device_leaf, |s| &s.channel];
    let mut suffix: Vec<Vec<String>> = vec![Vec::new(); sensors.len()];
    for tier in tiers {
        let collides = colliding(sensors);
        for (i, s) in sensors.iter_mut().enumerate() {
            let part = tier(s);
            if collides[i] && !part.is_empty() {
                suffix[i].push(part.to_string());
                s.label = format!("{} ({})", s.base_label, suffix[i].join(" "));
            }
        }
    }

    let collides = colliding(sensors);
    for (i, s) in sensors.iter_mut().enumerate() {
        if collides[i] {
            log::warn!(
                "Sensors '{}' cannot be told apart by device; using discovery position",
                s.label
            );
            s.label = format!("{} [raw {}]", s.label, s.raw_pos);
        }
    }
}

/// Remove the ` [raw N]` last-resort suffix, if present
pub(crate) fn strip_raw_suffix(label: &str) -> &str {
    match label.rfind(" [raw ") {
        Some(pos) if label.ends_with(']') => &label[..pos],
        _ => label,
    }
}

/// Find the sensor a saved config refers to.
///
/// 1. Exact label match (labels are unique after `finalize`).
/// 2. Base-label match, first in sorted order: configs saved before
///    disambiguation existed stored the bare colliding label.
/// 3. For a label carrying the ` [raw N]` last-resort suffix, match ignoring
///    that suffix: those sensors were indistinguishable, so "first" is the
///    best available answer.
/// 4. No label (config saved before labels existed): the index, mapped through
///    `legacy_order` when that raw discovery order is what the index meant.
///
/// A label that is set but matches nothing yields `None`. Callers show
/// "not found" rather than silently reading a different sensor.
pub(crate) fn resolve_sensor<T: SensorIdentity>(
    sensors: &[T],
    label: Option<&str>,
    index: usize,
    legacy_order: Option<&[usize]>,
) -> Option<usize> {
    match label.filter(|l| !l.is_empty()) {
        Some(label) => sensors
            .iter()
            .position(|s| s.label() == label)
            .or_else(|| sensors.iter().position(|s| s.base_label() == label))
            .or_else(|| {
                let stem = strip_raw_suffix(label);
                (stem != label)
                    .then(|| {
                        sensors
                            .iter()
                            .position(|s| strip_raw_suffix(s.label()) == stem)
                    })
                    .flatten()
            }),
        None => {
            let idx = match legacy_order {
                Some(order) => *order.get(index)?,
                None => index,
            };
            (idx < sensors.len()).then_some(idx)
        }
    }
}

/// "'label'" or "#index" for log/UI messages about a saved selection
pub fn describe_selection(label: Option<&str>, index: usize) -> String {
    match label {
        Some(label) if !label.is_empty() => format!("'{}'", label),
        _ => format!("#{}", index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str, dev: &str, n: u32, label: &str) -> HwmonSensor {
        sl(name, dev, "", n, label)
    }

    fn sl(name: &str, dev: &str, leaf: &str, n: u32, label: &str) -> HwmonSensor {
        HwmonSensor {
            name: name.into(),
            device_id: dev.into(),
            device_leaf: leaf.into(),
            channel: format!("temp{}", n),
            number: n,
            label: label.into(),
            base_label: label.into(),
            path: None,
            raw_pos: 0,
        }
    }

    fn labels(sensors: &[HwmonSensor]) -> Vec<&str> {
        sensors.iter().map(|s| s.label.as_str()).collect()
    }

    struct L(&'static str, &'static str);
    impl SensorIdentity for L {
        fn label(&self) -> &str {
            self.0
        }
        fn base_label(&self) -> &str {
            self.1
        }
    }

    #[test]
    fn sensor_number_parses_input_files_only() {
        assert_eq!(sensor_number("temp3_input", "temp"), Some(3));
        assert_eq!(sensor_number("fan12_input", "fan"), Some(12));
        assert_eq!(sensor_number("temp3_label", "temp"), None);
        assert_eq!(sensor_number("fan1_input", "temp"), None);
        assert_eq!(sensor_number("tempX_input", "temp"), None);
    }

    #[test]
    fn pci_bdf_detection() {
        assert!(is_pci_bdf("0000:02:00.0"));
        assert!(is_pci_bdf("0000:0b:0c.0"));
        assert!(!is_pci_bdf("nvme0"));
        assert!(!is_pci_bdf("0000:02:00"));
        assert!(!is_pci_bdf("it87.2656"));
    }

    #[test]
    fn device_path_parsing_on_real_layouts() {
        let p = |s: &str| parse_device_path(Path::new(s));
        let own = |id: &str| (id.to_string(), String::new());
        // Platform device: no PCI address, the device name is the id
        assert_eq!(p("/sys/devices/platform/it87.2656"), own("it87.2656"));
        // PCI function is the device itself: no leaf
        assert_eq!(
            p("/sys/devices/pci0000:00/0000:00:08.1/0000:79:00.0"),
            own("0000:79:00.0")
        );
        assert_eq!(
            p("/sys/devices/pci0000:00/0000:00:18.3"),
            own("0000:00:18.3")
        );
        // NVMe: each drive is its own PCI function; nvme0 is the leaf
        assert_eq!(
            p("/sys/devices/pci0000:00/0000:00:01.2/0000:02:00.0/nvme/nvme0"),
            ("0000:02:00.0".to_string(), "nvme0".to_string())
        );
        // SATA drives share the AHCI controller; the SCSI address is the leaf
        assert_eq!(
            p("/sys/devices/pci0000:00/0000:00:17.0/ata1/host0/target0:0:0/0:0:0:0"),
            ("0000:00:17.0".to_string(), "0:0:0:0".to_string())
        );
        assert_eq!(
            p("/sys/devices/pci0000:00/0000:00:02.1/0000:03:00.0/0000:04:08.0/0000:0a:00.0/0000:0b:02.0/0000:0d:00.0/ieee80211/phy0"),
            ("0000:0d:00.0".to_string(), "phy0".to_string())
        );
        assert_eq!(p(""), (String::new(), String::new()));
    }

    #[test]
    fn finalize_sorts_and_maps_raw_order() {
        // Raw sysfs order seen on a real board: fan3, fan1, fan2
        let (sorted, raw) = finalize(vec![
            s("it87952", "it87.2656", 3, "it87952 Fan 3"),
            s("it87952", "it87.2656", 1, "it87952 Fan 1"),
            s("it87952", "it87.2656", 2, "it87952 Fan 2"),
        ]);
        assert_eq!(
            labels(&sorted),
            ["it87952 Fan 1", "it87952 Fan 2", "it87952 Fan 3"]
        );
        // raw position -> sorted index
        assert_eq!(raw, [2, 0, 1]);
        assert_eq!(
            sorted.iter().map(|s| s.raw_pos).collect::<Vec<_>>(),
            [1, 2, 0]
        );
    }

    #[test]
    fn finalize_groups_by_device_then_channel() {
        let (sorted, _) = finalize(vec![
            s("nvme", "0000:03:00.0", 2, "nvme Sensor 1 B"),
            s("nvme", "0000:02:00.0", 2, "nvme Sensor 1 A"),
            s("nvme", "0000:03:00.0", 1, "nvme Composite B"),
            s("k10temp", "0000:00:18.3", 1, "k10temp Tctl"),
            s("nvme", "0000:02:00.0", 1, "nvme Composite A"),
        ]);
        assert_eq!(
            labels(&sorted),
            [
                "k10temp Tctl",
                "nvme Composite A",
                "nvme Sensor 1 A",
                "nvme Composite B",
                "nvme Sensor 1 B",
            ]
        );
    }

    #[test]
    fn colliding_labels_get_device_suffix() {
        let (sorted, _) = finalize(vec![
            s("prom21_xhci", "0000:13:00.0", 1, "prom21_xhci temp1"),
            s("k10temp", "0000:00:18.3", 1, "k10temp Tctl"),
            s("prom21_xhci", "0000:11:00.0", 1, "prom21_xhci temp1"),
        ]);
        assert_eq!(
            labels(&sorted),
            [
                "k10temp Tctl",
                "prom21_xhci temp1 (0000:11:00.0)",
                "prom21_xhci temp1 (0000:13:00.0)",
            ]
        );
        assert_eq!(sorted[1].base_label, "prom21_xhci temp1");
        assert_eq!(sorted[0].base_label, "k10temp Tctl");
    }

    #[test]
    fn siblings_behind_one_controller_use_the_leaf() {
        // Two identical SATA drives on one AHCI controller: same driver,
        // same PCI address, same model label; only the SCSI address differs
        let (sorted, _) = finalize(vec![
            sl(
                "drivetemp",
                "0000:00:17.0",
                "1:0:0:0",
                1,
                "drivetemp WDC WD40EFRX",
            ),
            sl(
                "drivetemp",
                "0000:00:17.0",
                "0:0:0:0",
                1,
                "drivetemp WDC WD40EFRX",
            ),
        ]);
        assert_eq!(
            labels(&sorted),
            [
                "drivetemp WDC WD40EFRX (0000:00:17.0 0:0:0:0)",
                "drivetemp WDC WD40EFRX (0000:00:17.0 1:0:0:0)",
            ]
        );
    }

    #[test]
    fn empty_identity_parts_are_skipped() {
        // Only one of the two has a known device: it gets the suffix, the
        // other keeps its bare label, and nothing emits "()"
        let (sorted, _) = finalize(vec![
            s("foo", "0000:01:00.0", 1, "foo temp1"),
            s("foo", "", 1, "foo temp1"),
        ]);
        assert_eq!(labels(&sorted), ["foo temp1", "foo temp1 (0000:01:00.0)"]);
    }

    #[test]
    fn indistinguishable_sensors_fall_back_to_raw_position() {
        let (sorted, _) = finalize(vec![
            s("foo", "", 1, "foo temp1"),
            s("foo", "", 1, "foo temp1"),
        ]);
        assert_eq!(
            labels(&sorted),
            ["foo temp1 (temp1) [raw 0]", "foo temp1 (temp1) [raw 1]"]
        );
        assert_eq!(strip_raw_suffix(&sorted[1].label), "foo temp1 (temp1)");
        assert_eq!(strip_raw_suffix("CPU Fan (rear)"), "CPU Fan (rear)");
        assert_eq!(strip_raw_suffix("x [raw 3"), "x [raw 3");
        // Every finalized list has unique labels
        let mut seen = std::collections::HashSet::new();
        assert!(sorted.iter().all(|s| seen.insert(s.label.clone())));
    }

    #[test]
    fn resolve_prefers_exact_then_base_label() {
        let list = [
            L("k10temp Tctl", "k10temp Tctl"),
            L("prom21_xhci temp1 (0000:11:00.0)", "prom21_xhci temp1"),
            L("prom21_xhci temp1 (0000:13:00.0)", "prom21_xhci temp1"),
        ];
        assert_eq!(
            resolve_sensor(&list, Some("prom21_xhci temp1 (0000:13:00.0)"), 0, None),
            Some(2)
        );
        // Config saved before disambiguation: first match by base label
        assert_eq!(
            resolve_sensor(&list, Some("prom21_xhci temp1"), 2, None),
            Some(1)
        );
        assert_eq!(
            resolve_sensor(&list, Some("k10temp Tctl"), 7, None),
            Some(0)
        );
    }

    #[test]
    fn resolve_missing_label_is_none_not_index() {
        let list = [L("a", "a"), L("b", "b")];
        assert_eq!(resolve_sensor(&list, Some("gone"), 0, None), None);
        assert_eq!(resolve_sensor(&list, Some("gone"), 1, Some(&[1, 0])), None);
        // A real label that merely looks suffixed is not stripped
        let list = [
            L("CPU Fan", "CPU Fan"),
            L("CPU Fan (front)", "CPU Fan (front)"),
        ];
        assert_eq!(resolve_sensor(&list, Some("CPU Fan (rear)"), 0, None), None);
    }

    #[test]
    fn resolve_raw_suffix_matches_ignoring_position() {
        let list = [
            L("foo temp1 (temp1) [raw 4]", "foo temp1"),
            L("foo temp1 (temp1) [raw 9]", "foo temp1"),
        ];
        // Saved on a boot where the positions were different
        assert_eq!(
            resolve_sensor(&list, Some("foo temp1 (temp1) [raw 2]"), 0, None),
            Some(0)
        );
        assert_eq!(
            resolve_sensor(&list, Some("foo temp1 (temp1) [raw 9]"), 0, None),
            Some(1)
        );
    }

    #[test]
    fn resolve_legacy_index_uses_raw_order_when_given() {
        let list = [L("fan1", "fan1"), L("fan2", "fan2"), L("fan3", "fan3")];
        // raw order was fan3, fan1, fan2 -> raw index 1 meant fan1 (sorted 0)
        let raw = [2usize, 0, 1];
        assert_eq!(resolve_sensor(&list, None, 1, Some(&raw)), Some(0));
        assert_eq!(resolve_sensor(&list, None, 0, Some(&raw)), Some(2));
        assert_eq!(resolve_sensor(&list, None, 5, Some(&raw)), None);
        // Without a legacy order the index is taken as-is
        assert_eq!(resolve_sensor(&list, None, 1, None), Some(1));
        assert_eq!(resolve_sensor(&list, None, 3, None), None);
        // An empty label counts as no label
        assert_eq!(resolve_sensor(&list, Some(""), 1, None), Some(1));
    }

    #[test]
    fn describe_selection_prefers_label() {
        assert_eq!(describe_selection(None, 4), "#4");
        assert_eq!(describe_selection(Some(""), 4), "#4");
        assert_eq!(
            describe_selection(Some("k10temp Tctl"), 4),
            "'k10temp Tctl'"
        );
    }
}
