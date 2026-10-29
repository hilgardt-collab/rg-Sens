//! Shared hwmon sensor discovery helpers used by the fan speed and system
//! temperature sources.
//!
//! Identity rules (both sources follow these):
//! - A sensor is identified in saved configs by its **label**. Labels are made
//!   unique at discovery: when two sensors share a label (two identical NVMe
//!   drives, two USB controllers of the same chipset) each gets its device id
//!   appended, e.g. `prom21_xhci temp1 (0000:11:00.0)`.
//! - The discovered list is sorted by (driver name, device id, sensor number)
//!   so dropdown order and indices are deterministic. sysfs enumeration order
//!   is not: `read_dir` returns hash order and hwmon numbering changes per boot.
//! - `sensor_index` in a config is only consulted when the config has no label
//!   (saved before labels existed).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One `{prefix}N_input` sensor found under /sys/class/hwmon
#[derive(Debug, Clone)]
pub(crate) struct HwmonSensor {
    /// Driver name from the hwmon `name` file (falls back to the directory name)
    pub name: String,
    /// Stable identity of the device behind the hwmon directory (see `device_id`)
    pub device_id: String,
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
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
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

/// Stable identity of the device behind an hwmon directory.
///
/// Prefers the PCI address found in the resolved `device` link (stable across
/// boots for fixed hardware, unlike `nvme0`/`phy0`/`hwmonN` numbering), then
/// the device's own sysfs name (e.g. `it87.2656`), then the hwmon directory name.
#[cfg(target_os = "linux")]
pub(crate) fn device_id(hwmon_dir: &Path) -> String {
    if let Ok(real) = std::fs::canonicalize(hwmon_dir.join("device")) {
        let parts: Vec<String> = real
            .iter()
            .map(|c| c.to_string_lossy().into_owned())
            .collect();
        if let Some(bdf) = parts.iter().rev().find(|p| is_pci_bdf(p)) {
            return bdf.clone();
        }
        if let Some(last) = parts.last() {
            return last.clone();
        }
    }
    dir_name(hwmon_dir)
}

#[cfg(target_os = "linux")]
fn dir_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Scan /sys/class/hwmon for `{prefix}N_input` files, in raw sysfs order.
/// `label_for(hwmon_dir, driver_name, N)` builds the human-readable label.
#[cfg(target_os = "linux")]
pub(crate) fn scan<F>(prefix: &str, label_for: F) -> Vec<HwmonSensor>
where
    F: Fn(&Path, &str, u32) -> String,
{
    let mut sensors = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return sensors;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let Ok(files) = std::fs::read_dir(&dir) else {
            continue;
        };
        let name = read_trimmed(&dir.join("name"))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| dir_name(&dir));
        let device_id = device_id(&dir);
        for file in files.flatten() {
            let filename = file.file_name();
            let Some(number) = sensor_number(&filename.to_string_lossy(), prefix) else {
                continue;
            };
            let label = label_for(&dir, &name, number);
            sensors.push(HwmonSensor {
                name: name.clone(),
                device_id: device_id.clone(),
                number,
                base_label: label.clone(),
                label,
                path: Some(file.path()),
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
            device_id: zone,
            number,
            base_label: name.clone(),
            label: name,
            path: Some(temp),
            raw_pos: 0,
        });
    }
    sensors
}

/// Sort by (driver name, device id, sensor number), make labels unique and
/// return the sorted list plus a map from raw discovery position to sorted
/// index (used to interpret index-only configs saved before labels existed).
pub(crate) fn finalize(mut sensors: Vec<HwmonSensor>) -> (Vec<HwmonSensor>, Vec<usize>) {
    for (i, s) in sensors.iter_mut().enumerate() {
        s.raw_pos = i;
    }
    sensors.sort_by(|a, b| {
        (a.name.as_str(), a.device_id.as_str(), a.number).cmp(&(
            b.name.as_str(),
            b.device_id.as_str(),
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

/// Append the device id (and, if still ambiguous, the channel number) to every
/// label that occurs more than once. `base_label` keeps the original.
pub(crate) fn disambiguate_labels(sensors: &mut [HwmonSensor]) {
    fn counts(sensors: &[HwmonSensor]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for s in sensors {
            *counts.entry(s.label.clone()).or_insert(0usize) += 1;
        }
        counts
    }

    let by_label = counts(sensors);
    for s in sensors.iter_mut() {
        if by_label[&s.label] > 1 {
            s.label = format!("{} ({})", s.base_label, s.device_id);
        }
    }
    // Same device, same label: add the channel number as well.
    let by_label = counts(sensors);
    for s in sensors.iter_mut() {
        if by_label[&s.label] > 1 {
            s.label = format!("{} ({} #{})", s.base_label, s.device_id, s.number);
        }
    }
}

/// Find the sensor a saved config refers to.
///
/// 1. Exact label match (labels are unique after `finalize`).
/// 2. Base-label match, first in sorted order: configs saved before
///    disambiguation existed stored the bare colliding label.
/// 3. No label (config saved before labels existed): the index, mapped through
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
            .or_else(|| sensors.iter().position(|s| s.base_label() == label)),
        None => {
            let idx = match legacy_order {
                Some(order) => *order.get(index)?,
                None => index,
            };
            (idx < sensors.len()).then_some(idx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str, dev: &str, n: u32, label: &str) -> HwmonSensor {
        HwmonSensor {
            name: name.into(),
            device_id: dev.into(),
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
    fn same_device_collision_adds_channel_number() {
        let mut v = vec![
            s("dev", "0000:01:00.0", 1, "dev Composite"),
            s("dev", "0000:01:00.0", 2, "dev Composite"),
        ];
        disambiguate_labels(&mut v);
        assert_eq!(
            labels(&v),
            [
                "dev Composite (0000:01:00.0 #1)",
                "dev Composite (0000:01:00.0 #2)",
            ]
        );
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
}
