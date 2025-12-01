use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::device_map;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoviceHint {
    pub category: String,
    pub severity: String,
    pub message: String,
    pub evidence: Vec<String>,
    pub count: usize,
    pub probability: u8,
}

fn extract_data_pairs(xml: &str) -> HashMap<String, String> { crate::event_xml::event_data_pairs_or_fallback(xml) }

fn push_hint(acc: &mut HashMap<(String, String, String), NoviceHint>, category: &str, severity: &str, message: &str, evidence: Option<String>) {
    let key = (category.to_string(), severity.to_string(), message.to_string());
    let entry = acc.entry(key.clone()).or_insert(NoviceHint {
        category: key.0.clone(),
        severity: key.1.clone(),
        message: key.2.clone(),
        evidence: Vec::new(),
        count: 0,
        probability: 0,
    });
    entry.count += 1;
    if let Some(ev) = evidence
        && entry.evidence.len() < 3 && !ev.is_empty() {
        entry.evidence.push(ev);
    }
}

pub fn generate_hints(events: &[crate::EventItem]) -> Vec<NoviceHint> {
    let mut acc: HashMap<(String, String, String), NoviceHint> = HashMap::new();
    for e in events {
        let m = extract_data_pairs(&e.content);
        let content_lower = e.content.to_lowercase();
        match e.provider.as_str() {
            "Application Error" => {
                if e.event_id == 1000 {
                    let app = m.get("FaultingApplicationName").cloned().unwrap_or_default();
                    let module = m.get("FaultingModuleName").cloned().unwrap_or_default();
                    let ev = if !app.is_empty() { app } else { module };
                    push_hint(&mut acc, "Application", "high", "Application crash detected", if ev.is_empty() { None } else { Some(ev) });
                }
            }
            "Microsoft-Windows-Kernel-Acpi" | "Microsoft-Windows-ACPI" | "ACPI" | "Microsoft-Windows-Thermal" => {
                let inst = m.get("DeviceInstanceId").cloned().unwrap_or_default();
                if content_lower.contains("fan") {
                    if content_lower.contains("fail") || content_lower.contains("stalled") || content_lower.contains("not detected") {
                        let mut msg = "CPU/Chassis fan failure detected".to_string();
                        if let Some(cls) = if inst.is_empty() { None } else { crate::device_map::classify_instance_id(&inst) } { msg = format!("{} [{}]", msg, cls); }
                        push_hint(&mut acc, "Cooling", "high", &msg, if inst.is_empty() { None } else { Some(inst.clone()) });
                    } else if content_lower.contains("rpm") || content_lower.contains("tachometer") {
                        push_hint(&mut acc, "Cooling", "medium", "Fan speed low or unstable", if inst.is_empty() { None } else { Some(inst.clone()) });
                    } else {
                        push_hint(&mut acc, "Cooling", "medium", "Fan-related event reported", if inst.is_empty() { None } else { Some(inst.clone()) });
                    }
                }
                if content_lower.contains("thermal zone") || content_lower.contains("temperature") || content_lower.contains("overheat") || content_lower.contains("critical") {
                    let temp = m.get("CurrentTemperature").cloned().unwrap_or_default();
                    let ev = if temp.is_empty() { inst.clone() } else { temp };
                    push_hint(&mut acc, "Thermal", "medium", "Thermal zone or sensor reports high temperature", if ev.is_empty() { None } else { Some(ev) });
                }
            }
            "Microsoft-Windows-DNS-Client" => {
                if e.event_id == 1014 || content_lower.contains("name resolution") || content_lower.contains("dns") {
                    let q = m.get("QueryName").cloned().unwrap_or_default();
                    push_hint(&mut acc, "Network", "medium", "DNS name resolution failure", if q.is_empty() { None } else { Some(q) });
                }
            }
            "Microsoft-Windows-Time-Service" | "W32Time" => {
                if content_lower.contains("failed") || content_lower.contains("no response") || content_lower.contains("synchronize") {
                    let src = m.get("SourceType").cloned().unwrap_or_default();
                    push_hint(&mut acc, "System", "medium", "System time synchronization failed", if src.is_empty() { None } else { Some(src) });
                }
            }
            "Microsoft-Windows-GroupPolicy" => {
                if content_lower.contains("failed") || content_lower.contains("could not apply") || content_lower.contains("processing aborted") {
                    let dc = m.get("DCName").cloned().unwrap_or_default();
                    let gpo = m.get("GPOID").cloned().unwrap_or_default();
                    let evidence = if !gpo.is_empty() { gpo } else { dc };
                    push_hint(&mut acc, "Policy", "medium", "Group Policy processing failure", if evidence.is_empty() { None } else { Some(evidence) });
                }
            }
            "Microsoft-Windows-WHEA-Logger" => {
                match e.event_id {
                    18 => {
                        let src = m.get("ErrorSource").cloned().unwrap_or_default();
                        let apic = m.get("ApicId").or_else(|| m.get("ProcessorAPICID")).cloned().unwrap_or_default();
                        let ev = if apic.is_empty() { src } else { format!("{} APIC {}", src, apic) };
                        push_hint(&mut acc, "Hardware", "high", "Uncorrected hardware error detected (machine check)", Some(ev));
                    }
                    17 => {
                        let comp = m.get("Component").cloned().unwrap_or_default();
                        let dev = m.get("DeviceId").cloned().unwrap_or_default();
                        let ev = if comp.is_empty() { dev } else { comp };
                        push_hint(&mut acc, "Hardware", "medium", "Corrected hardware error reported", Some(ev));
                    }
                    19 | 20 => {
                        let src = m.get("ErrorSource").cloned().unwrap_or_default();
                        push_hint(&mut acc, "Hardware", "medium", "Hardware error reported by WHEA", Some(src));
                    }
                    _ => {}
                }
                let bus = m.get("Bus").cloned();
                let dev = m.get("Device").cloned();
                let func = m.get("Function").cloned();
                if let Some(cls) = device_map::classify_bdf_platform(bus.as_deref(), dev.as_deref(), func.as_deref()) {
                    let bdf = format!("B:{} D:{} F:{}", bus.unwrap_or_default(), dev.unwrap_or_default(), func.unwrap_or_default());
                    push_hint(&mut acc, "Hardware", "medium", &format!("{} ({} )", cls, bdf), None);
                }
            }
            "Service Control Manager" | "Microsoft-Windows-Services" => {
                if content_lower.contains("failed to start") || content_lower.contains("start pending timed out") || content_lower.contains("terminated unexpectedly") {
                    let svc = m.get("ServiceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
                    let msg = if svc.is_empty() { "Service start/termination failure".to_string() } else { format!("Service failure: {}", svc) };
                    let sev = if content_lower.contains("failed") || content_lower.contains("terminated") { "high" } else { "medium" };
                    push_hint(&mut acc, "Services", sev, &msg, if svc.is_empty() { None } else { Some(svc) });
                }
            }
            "Disk" => {
                match e.event_id {
                    7 => {
                        let dev = m.get("DeviceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
                        push_hint(&mut acc, "Storage", "high", "Bad block detected on disk", Some(dev));
                    }
                    11 => {
                        let dev = m.get("DeviceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
                        push_hint(&mut acc, "Storage", "high", "Disk or controller error", Some(dev));
                    }
                    51 => {
                        push_hint(&mut acc, "Storage", "medium", "Paging I/O error indicates unstable storage path", None);
                    }
                    157 => {
                        let dev = m.get("DeviceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
                        push_hint(&mut acc, "Storage", "high", "Disk was surprise removed (connection/port)", Some(dev));
                    }
                    _ => {}
                }
            }
            "Microsoft-Windows-Ntfs" => {
                match e.event_id {
                    55 => push_hint(&mut acc, "Storage", "high", "File system corruption detected (NTFS)", None),
                    57 => push_hint(&mut acc, "Storage", "high", "Delayed write failed", None),
                    140 => push_hint(&mut acc, "Storage", "high", "Failed to flush data to transaction log (NTFS)", None),
                    _ => {}
                }
            }
            "Storport" => {
                match e.event_id {
                    129 => push_hint(&mut acc, "Storage", "medium", "Reset to device implies storage connectivity issue", None),
                    153 => push_hint(&mut acc, "Storage", "medium", "I/O operation retried by Storport", None),
                    _ => {}
                }
            }
            "volmgr" => {
                if content_lower.contains("failed to flush data to the transaction log") {
                    push_hint(&mut acc, "Storage", "high", "Volume manager flush failure – potential corruption", None);
                }
            }
            "volsnap" => {
                if content_lower.contains("shadow copies of volume") && content_lower.contains("were aborted") {
                    push_hint(&mut acc, "Storage", "medium", "Shadow copies aborted – may indicate underlying disk issues", None);
                }
            }
            "Microsoft-Windows-DiskDiagnostic" | "Microsoft-Windows-DiskDiagnosticDataCollector" => {
                let reason = m.get("Reason").cloned().unwrap_or_default();
                let degraded = m.get("PercentPerformanceDegraded").cloned().unwrap_or_default();
                let ev = if !reason.is_empty() { reason } else { degraded };
                push_hint(&mut acc, "Storage", "high", "Windows detected disk reliability issue", if ev.is_empty() { None } else { Some(ev) });
            }
            "Microsoft-Windows-Kernel-PnP" => {
                if e.event_id == 219 {
                    let dev = m.get("DeviceInstanceId").cloned().unwrap_or_default();
                    let mut msg = "Driver failed to load for a device (Kernel-PnP 219)".to_string();
                    if let Some(cls) = device_map::classify_instance_id(&dev) { msg = format!("{} [{}]", msg, cls); }
                    push_hint(&mut acc, "Peripheral", "medium", &msg, if dev.is_empty() { None } else { Some(dev) });
                }
            }
            "Microsoft-Windows-UserPnp" => {
                if e.event_id == 2003 || content_lower.contains("driver install failed") || content_lower.contains("device install failed") {
                    let dev = m.get("DeviceInstanceID").or_else(|| m.get("DeviceInstanceId")).cloned().unwrap_or_default();
                    let mut msg = "Device installation failed".to_string();
                    if let Some(cls) = if dev.is_empty() { None } else { device_map::classify_instance_id(&dev) } { msg = format!("{} [{}]", msg, cls); }
                    push_hint(&mut acc, "Peripheral", "medium", &msg, if dev.is_empty() { None } else { Some(dev) });
                }
            }
            "Microsoft-Windows-Kernel-Power" => {
                if e.event_id == 41 {
                    push_hint(&mut acc, "Power", "high", "Unexpected shutdown or power loss detected", None);
                }
            }
            "Microsoft-Windows-EventLog" | "EventLog" => {
                if e.event_id == 6008 {
                    push_hint(&mut acc, "Power", "high", "Previous system shutdown was unexpected", None);
                }
            }
            "Microsoft-Windows-Kernel-Processor-Power" => {
                if e.event_id == 37 {
                    push_hint(&mut acc, "Thermal", "medium", "CPU frequency limited by firmware (thermal/power)", None);
                }
            }
            "Display" => {
                if e.event_id == 4101 {
                    push_hint(&mut acc, "GPU", "medium", "Display driver stopped responding and recovered", None);
                }
            }
            "Microsoft-Windows-DxgKrnl" => {
                if e.event_id == 2 || e.event_id == 3 {
                    push_hint(&mut acc, "GPU", "medium", "Video scheduler or graphics kernel reported a fault", None);
                }
            }
            "nvlddmkm" | "amdkmdag" => {
                push_hint(&mut acc, "GPU", "medium", "GPU driver timeout or reset detected", None);
            }
            "USBHUB" | "USBHUB3" | "USBXHCI" | "usbhub" | "usbstor" | "USB" => {
                if content_lower.contains("enumeration failed") || content_lower.contains("descriptor request failed") || content_lower.contains("port reset failed") {
                    push_hint(&mut acc, "Peripheral", "medium", "USB device enumeration or port failure", None);
                }
            }
            "cdrom" => {
                if e.event_id == 11 || content_lower.contains("controller error") {
                    push_hint(&mut acc, "Storage", "medium", "CD/DVD device or controller error", None);
                }
            }
            "Netlogon" | "NETLOGON" => {
                if content_lower.contains("domain controller") || content_lower.contains("logon failure") || content_lower.contains("could not establish a secure connection") {
                    let dc = m.get("DnsHostName").or_else(|| m.get("DCName")).cloned().unwrap_or_default();
                    push_hint(&mut acc, "Network", "medium", "Domain logon or secure channel issue", if dc.is_empty() { None } else { Some(dc) });
                }
            }
            "Microsoft-Windows-MemoryDiagnostics-Results" => {
                let errs = m.get("TestResult").or_else(|| m.get("FailureCount")).cloned().unwrap_or_default();
                if !errs.is_empty() && errs != "0" {
                    push_hint(&mut acc, "Memory", "high", "Memory diagnostics reported errors", Some(errs));
                }
            }
            _ => {}
        }
        if content_lower.contains("access denied") || content_lower.contains("permission") || content_lower.contains("privilege") {
            push_hint(&mut acc, "Permissions", "medium", "Access denied or insufficient permissions detected", None);
        }
        if e.provider == "DistributedCOM" && content_lower.contains("do not grant") && content_lower.contains("permission settings") {
            push_hint(&mut acc, "Permissions", "medium", "DCOM permission misconfiguration", None);
        }
        if content_lower.contains("dns") || content_lower.contains("name resolution") || content_lower.contains("tcp") || content_lower.contains("connection timed out") || content_lower.contains("reset by peer") || content_lower.contains("dhcp") || content_lower.contains("media disconnected") {
            push_hint(&mut acc, "Network", "medium", "Network connectivity or name resolution issue", None);
        }
        if content_lower.contains("windows update") || content_lower.contains("wuau") || content_lower.contains("failed to install update") || content_lower.contains("download error") {
            push_hint(&mut acc, "Updates", "medium", "Windows Update reported a failure", None);
        }
        if content_lower.contains("low disk space") || content_lower.contains("not enough space") || content_lower.contains("quota exceeded") {
            push_hint(&mut acc, "Storage", "medium", "Low disk space or quota exceeded", None);
        }
        if content_lower.contains("bugcheck") || content_lower.contains("stop code") {
            push_hint(&mut acc, "Power", "high", "System crash (BugCheck) indicated", None);
        }
        if (e.provider.to_lowercase().contains("iastor") || e.provider.to_lowercase().contains("storahci") || e.provider.to_lowercase().contains("nvme"))
            && (content_lower.contains("reset to device") || content_lower.contains("i/o was retried")) {
            push_hint(&mut acc, "Storage", "medium", "Storage controller reported resets/retries (path instability)", None);
        }
        if e.provider.to_lowercase().contains("cdrom")
            && (e.event_id == 11 || content_lower.contains("controller error") || content_lower.contains("device not ready")) {
            push_hint(&mut acc, "Storage", "medium", "Optical drive or controller error", None);
        }
        if e.provider == "Microsoft-Windows-Diagnostics-Performance" {
            match e.event_id {
                100 => push_hint(&mut acc, "Performance", "medium", "Slow startup detected (Diagnostics-Performance 100)", None),
                200 => push_hint(&mut acc, "Performance", "medium", "Slow logon detected (Diagnostics-Performance 200)", None),
                400 => push_hint(&mut acc, "Performance", "medium", "Slow resume from standby detected (Diagnostics-Performance 400)", None),
                _ => {}
            }
        }
        if content_lower.contains("retry") || content_lower.contains("reset") || content_lower.contains("corrupt") || content_lower.contains("degraded") || content_lower.contains("unexpected") {
            push_hint(&mut acc, "General", "medium", "System reported error patterns indicating instability", None);
        }
        if let Some((sev, msg)) = device_map::smart_hint_from_text(&content_lower) {
            push_hint(&mut acc, "Storage", sev, msg, None);
        }
    }
    let mut out: Vec<NoviceHint> = acc.into_values().collect();
    for h in &mut out {
        let base = match h.severity.as_str() { "high" => 75u8, "medium" => 50u8, _ => 25u8 };
        let bump = if h.count >= 5 { 15 } else if h.count >= 3 { 10 } else if h.count >= 2 { 5 } else { 0 };
        let evb = if h.evidence.is_empty() { 0 } else { 5 };
        let p = base.saturating_add(bump).saturating_add(evb);
        h.probability = p.clamp(5, 95);
    }
    let has_volsnap_abort = events.iter().any(|e| e.provider.eq_ignore_ascii_case("volsnap") && e.content.to_lowercase().contains("aborted"));
    let has_ntfs_55 = events.iter().any(|e| e.provider.eq_ignore_ascii_case("Microsoft-Windows-Ntfs") && e.event_id == 55);
    if has_volsnap_abort && has_ntfs_55 {
        push_hint(&mut acc, "Storage", "high", "Shadow copies aborted and NTFS corruption detected (sequence)", None);
    }
    out.sort_by(|a, b| b.count.cmp(&a.count));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, TimeZone};

    #[test]
    fn hints_from_disk_event() {
        let e = crate::EventItem {
            time: Utc.with_ymd_and_hms(2025, 11, 30, 12, 0, 0).unwrap(),
            level: 2,
            channel: "System".to_string(),
            provider: "Disk".to_string(),
            event_id: 7,
            content: "<EventData><Data Name=\"DeviceName\">\\\\.\\PHYSICALDRIVE2</Data></EventData>".to_string(),
            raw_xml: None,
        };
        let out = generate_hints(&[e]);
        assert!(out.iter().any(|h| h.category == "Storage" && h.severity == "high"));
    }
}
