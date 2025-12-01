pub fn classify_instance_id(id: &str) -> Option<String> {
    let id_lower = id.to_lowercase();
    if id_lower.starts_with("nvme\\") { return Some("NVMe drive".to_string()); }
    if id_lower.starts_with("scsi\\disk") { return Some("SATA/SAS disk".to_string()); }
    if id_lower.starts_with("usb\\vid_") { return Some("USB device".to_string()); }
    if id_lower.starts_with("acpi\\pnp0c0b") { return Some("ACPI fan".to_string()); }
    if id_lower.starts_with("acpi\\pnp0c0a") { return Some("ACPI thermal zone".to_string()); }
    if let (Some(vendor), dev_opt) = parse_pci_ven_dev(&id_lower) {
        let base = classify_vendor_hex(&vendor).unwrap_or("PCI device");
        if let Some(dev) = dev_opt { return Some(format!("{} device 0x{}", base, dev)); }
        return Some(base.to_string());
    }
    None
}

pub fn classify_vendor_hex(vendor_hex: &str) -> Option<&'static str> {
    match vendor_hex {
        "10de" => Some("NVIDIA GPU"),
        "1002" => Some("AMD GPU"),
        "8086" => Some("Intel controller/device"),
        "144d" => Some("Samsung NVMe"),
        "1bb1" => Some("Western Digital NVMe"),
        "1987" => Some("Phison NVMe"),
        "1c5c" => Some("SK hynix NVMe"),
        "1344" => Some("Micron NVMe"),
        "10ec" => Some("Realtek controller/device"),
        "14e4" => Some("Broadcom controller/device"),
        "1b21" => Some("ASMedia controller/device"),
        "197b" => Some("JMicron controller/device"),
        "126f" => Some("Silicon Motion NVMe"),
        "15b7" => Some("SanDisk NVMe"),
        "1e0f" => Some("KIOXIA NVMe"),
        "1e49" => Some("Solidigm NVMe"),
        "1d97" => Some("ADATA NVMe"),
        "1022" => Some("AMD controller/device"),
        _ => None,
    }
}

pub fn classify_bdf(bus: Option<&str>, dev: Option<&str>, func: Option<&str>) -> Option<String> {
    let b = bus.and_then(|s| s.parse::<u32>().ok());
    let d = dev.and_then(|s| s.parse::<u32>().ok());
    let f = func.and_then(|s| s.parse::<u32>().ok());
    if let (Some(b), Some(d)) = (b, d) {
        if b == 1 && d == 0 { return Some("Likely discrete GPU (PEG root path)".to_string()); }
        if b >= 1 && d <= 3 && f == Some(0) { return Some("Device on CPU PCIe lanes (GPU/NVMe)".to_string()); }
        if (16..=31).contains(&d) { return Some("PCIe root/downstream port".to_string()); }
        if f == Some(0) && d <= 7 { return Some("Onboard controller/device".to_string()); }
    }
    None
}

pub fn smart_hint_from_text(text: &str) -> Option<(&'static str, &'static str)> {
    let t = text.to_lowercase();
    if t.contains("smart") && (t.contains("pred fail") || t.contains("failed") || t.contains("bad")) {
        return Some(("high", "SMART indicates predicted disk failure"));
    }
    if t.contains("reallocated") || t.contains("pending sector") || t.contains("uncorrectable") {
        return Some(("medium", "SMART attributes suggest media degradation"));
    }
    if t.contains("temperature") && (t.contains("high") || t.contains("overheat") || t.contains("critical")) {
        return Some(("medium", "SMART reports high temperature"));
    }
    None
}

#[cfg(target_os = "windows")]
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
static PNP_CACHE: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();
#[cfg(target_os = "windows")]
static DISK_CACHE: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();

#[cfg(target_os = "windows")]
fn build_pnp_cache() -> std::collections::HashMap<String, String> {
    use wmi::WMIConnection;
    #[derive(Debug, serde::Deserialize)]
    struct Row {
        #[serde(rename = "PNPDeviceID")] pnp_device_id: Option<String>,
        #[serde(rename = "Name")] name: Option<String>,
    }
    let mut map = std::collections::HashMap::new();
    if let Ok(wmi) = WMIConnection::new() {
        if let Ok(rows) = wmi.raw_query::<Row>("SELECT PNPDeviceID, Name FROM Win32_PnPEntity") {
            for r in rows { if let (Some(id), Some(name)) = (r.pnp_device_id, r.name) { map.insert(id.to_uppercase(), name); } }
        }
    }
    map
}

#[cfg(target_os = "windows")]
fn build_disk_cache() -> std::collections::HashMap<String, String> {
    use wmi::WMIConnection;
    #[derive(Debug, serde::Deserialize)]
    struct Row {
        #[serde(rename = "PNPDeviceID")] pnp_device_id: Option<String>,
        #[serde(rename = "Model")] model: Option<String>,
    }
    let mut map = std::collections::HashMap::new();
    if let Ok(wmi) = WMIConnection::new() {
        if let Ok(rows) = wmi.raw_query::<Row>("SELECT PNPDeviceID, Model FROM Win32_DiskDrive") {
            for r in rows { if let (Some(id), Some(model)) = (r.pnp_device_id, r.model) { map.insert(id.to_uppercase(), model); } }
        }
    }
    map
}

#[cfg(target_os = "windows")]
pub fn friendly_device(id_or_name: &str) -> Option<String> {
    let id = id_or_name.trim().to_uppercase();
    let pnp = PNP_CACHE.get_or_init(build_pnp_cache);
    if let Some(name) = pnp.get(&id) { return Some(name.clone()); }
    let disk = DISK_CACHE.get_or_init(build_disk_cache);
    if let Some(model) = disk.get(&id) { return Some(model.clone()); }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn friendly_device(_id_or_name: &str) -> Option<String> { None }

fn parse_pci_ven_dev(id_lower: &str) -> (Option<String>, Option<String>) {
    fn take_hex4(s: &str, start: usize) -> Option<String> {
        if s.len() < start + 4 { return None; }
        let sub = &s[start..start+4];
        if sub.chars().all(|c| c.is_ascii_hexdigit()) { Some(sub.to_string()) } else { None }
    }
    let ven = if let Some(p) = id_lower.find("ven_") { take_hex4(id_lower, p + 4) } else { None };
    let dev = if let Some(p) = id_lower.find("dev_") { take_hex4(id_lower, p + 4) } else { None };
    (ven, dev)
}

pub fn classify_bdf_platform(bus: Option<&str>, dev: Option<&str>, func: Option<&str>) -> Option<String> {
    let spec = std::env::var("WINDOCTOR_BDF_HINTS")
        .or_else(|_| std::env::var("WINREPORT_BDF_HINTS"));
    if let Ok(spec) = spec {
        for entry in spec.split(';') {
            if let Some(eq) = entry.find('=') {
                let (key, val) = entry.split_at(eq);
                let val = &val[1..];
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 3
                    && bus == Some(parts[0]) && dev == Some(parts[1]) && func == Some(parts[2]) {
                    return Some(val.to_string());
                }
            }
        }
    }
    classify_bdf(bus, dev, func)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_instance_basic() {
        assert_eq!(classify_instance_id("NVME\\DEV123").as_deref(), Some("NVMe drive"));
        assert_eq!(classify_instance_id("USB\\VID_1234").as_deref(), Some("USB device"));
    }

    #[test]
    fn classify_vendor_hex_known() {
        assert_eq!(classify_vendor_hex("10de"), Some("NVIDIA GPU"));
        assert_eq!(classify_vendor_hex("8086"), Some("Intel controller/device"));
        assert_eq!(classify_vendor_hex("1e49"), Some("Solidigm NVMe"));
    }

    #[test]
    fn classify_bdf_platform_env_override() {
        unsafe { std::env::set_var("WINDOCTOR_BDF_HINTS", "1:0:0=Discrete GPU"); }
        let r = classify_bdf_platform(Some("1"), Some("0"), Some("0"));
        assert_eq!(r.as_deref(), Some("Discrete GPU"));
        unsafe { std::env::remove_var("WINDOCTOR_BDF_HINTS"); }
    }
}
    #[test]
    fn parse_pci_ven_dev_validation() {
        let (v1, d1) = parse_pci_ven_dev("pci\\ven_zzzz&dev_12g4");
        assert_eq!(v1, None);
        assert_eq!(d1, None);
        let (v2, d2) = parse_pci_ven_dev("pci\\ven_8086&dev_abcd");
        assert_eq!(v2.as_deref(), Some("8086"));
        assert_eq!(d2.as_deref(), Some("abcd"));
    }
