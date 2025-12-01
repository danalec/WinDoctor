use chrono::{DateTime, Utc};
use crate::EventItem;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerfCounters {
    pub cpu_percent: Option<u32>,
    pub avg_disk_ms_per_transfer: Option<f64>,
    pub disk_reads_per_sec: Option<u32>,
    pub disk_writes_per_sec: Option<u32>,
}

#[cfg(target_os = "windows")]
pub fn collect_perf_counters() -> PerfCounters {
    use wmi::WMIConnection;
    #[allow(non_snake_case)]
    #[derive(Debug, Deserialize)]
    struct CpuRow { #[serde(rename = "Name")] _Name: String, PercentProcessorTime: Option<u32> }
    #[allow(non_snake_case)]
    #[derive(Debug, Deserialize)]
    struct DiskRow { #[serde(rename = "Name")] _Name: String, AvgDiskSecPerTransfer: Option<f64>, DiskReadsPerSec: Option<u32>, DiskWritesPerSec: Option<u32> }
    let mut out = PerfCounters { cpu_percent: None, avg_disk_ms_per_transfer: None, disk_reads_per_sec: None, disk_writes_per_sec: None };
    if let Ok(wmi) = WMIConnection::new() {
        if let Ok(rows) = wmi.raw_query::<CpuRow>("SELECT Name, PercentProcessorTime FROM Win32_PerfFormattedData_PerfOS_Processor WHERE Name='_Total'")
            && let Some(r) = rows.into_iter().next() { out.cpu_percent = r.PercentProcessorTime; }
        if let Ok(rows) = wmi.raw_query::<DiskRow>("SELECT Name, AvgDiskSecPerTransfer, DiskReadsPerSec, DiskWritesPerSec FROM Win32_PerfFormattedData_PerfDisk_PhysicalDisk WHERE Name='_Total'")
            && let Some(r) = rows.into_iter().next() {
            out.avg_disk_ms_per_transfer = r.AvgDiskSecPerTransfer.map(|s| s * 1000.0);
            out.disk_reads_per_sec = r.DiskReadsPerSec;
            out.disk_writes_per_sec = r.DiskWritesPerSec;
        }
    }
    out
}

#[cfg(not(target_os = "windows"))]
pub fn collect_perf_counters() -> PerfCounters { PerfCounters { cpu_percent: None, avg_disk_ms_per_transfer: None, disk_reads_per_sec: None, disk_writes_per_sec: None } }

#[cfg(target_os = "windows")]
pub fn smart_predict_failure() -> Option<bool> {
    use wmi::WMIConnection;
    #[allow(non_snake_case)]
    #[derive(Debug, Deserialize)]
    struct SmartRow { PredictFailure: Option<bool> }
    if let Ok(wmi) = WMIConnection::new()
        && let Ok(rows) = wmi.raw_query::<SmartRow>("SELECT PredictFailure FROM MSStorageDriver_FailurePredictStatus") {
        let any_fail = rows.into_iter().any(|r| r.PredictFailure.unwrap_or(false));
        return Some(any_fail);
    }
    None
}

#[cfg(not(target_os = "windows"))]
pub fn smart_predict_failure() -> Option<bool> { None }

pub fn compute_perf_details(events: &[EventItem]) -> Vec<(String, u32, u32, usize)> {
    let mut boot: Vec<u32> = Vec::new();
    let mut logon: Vec<u32> = Vec::new();
    let mut resume: Vec<u32> = Vec::new();
    for e in events {
        if e.provider == "Microsoft-Windows-Diagnostics-Performance" {
            let pairs = crate::event_xml::event_data_pairs_or_fallback(&e.content);
            match e.event_id {
                100 => {
                    let dur = pairs.get("BootDuration").or_else(|| pairs.get("BootTime")).cloned().unwrap_or_default();
                    if let Ok(ms) = dur.parse::<u32>() && ms > 0 { boot.push(ms); }
                }
                200 => {
                    let dur = pairs.get("LogonDuration").cloned().unwrap_or_default();
                    if let Ok(ms) = dur.parse::<u32>() && ms > 0 { logon.push(ms); }
                }
                400 => {
                    let dur = pairs.get("ResumeDuration").or_else(|| pairs.get("ResumeTime")).cloned().unwrap_or_default();
                    if let Ok(ms) = dur.parse::<u32>() && ms > 0 { resume.push(ms); }
                }
                _ => {}
            }
        }
    }
    let mut out: Vec<(String, u32, u32, usize)> = Vec::new();
    let mut push_stats = |name: &str, v: &Vec<u32>| {
        if !v.is_empty() {
            let sum: u64 = v.iter().map(|&x| x as u64).sum();
            let avg = (sum / v.len() as u64) as u32;
            let max = *v.iter().max().unwrap_or(&0);
            out.push((name.to_string(), avg, max, v.len()));
        }
    };
    push_stats("Boot", &boot);
    push_stats("Logon", &logon);
    push_stats("Resume", &resume);
    out
}

pub fn compute_performance_metrics(events: &[EventItem]) -> (u8, Vec<(String, u8)>) {
    let mut signals: Vec<(String, u8)> = Vec::new();
    let mut score: u32 = 0;
    let mut add = |name: &str, weight: u8, count: usize| { if count > 0 { signals.push((name.to_string(), weight)); score += weight as u32 * count as u32; } };
    let c = |pred: fn(&EventItem) -> bool| -> usize { events.iter().filter(|e| pred(e)).count() };
    add("Disk bad blocks", 30, c(|e| e.provider == "Disk" && e.event_id == 7));
    add("Disk/controller errors", 25, c(|e| e.provider == "Disk" && (e.event_id == 11 || e.event_id == 51 || e.event_id == 157)));
    add("NTFS corruption", 25, c(|e| e.provider == "Microsoft-Windows-Ntfs" && (e.event_id == 55 || e.event_id == 57 || e.event_id == 140)));
    add("Storport resets/retries", 15, c(|e| e.provider == "Storport" && (e.event_id == 129 || e.event_id == 153)));
    add("Hardware machine checks", 35, c(|e| e.provider == "Microsoft-Windows-WHEA-Logger" && e.event_id == 18));
    add("CPU frequency limited", 10, c(|e| e.provider == "Microsoft-Windows-Kernel-Processor-Power" && e.event_id == 37));
    add("GPU driver timeout/reset", 10, c(|e| e.provider == "Display" && e.event_id == 4101 || e.provider == "nvlddmkm" || e.provider == "amdkmdag"));
    add("DNS failures", 5, c(|e| e.provider == "Microsoft-Windows-DNS-Client" || e.content.to_lowercase().contains("dns")));
    add("Service failures", 10, c(|e| e.provider == "Service Control Manager" || e.provider == "Microsoft-Windows-Services"));
    if let Some(e) = events.iter().find(|e| e.provider.starts_with("Microsoft-Windows-DiskDiagnostic") && e.content.contains("PercentPerformanceDegraded"))
        && let Some(re) = regex::Regex::new("(?i)PercentPerformanceDegraded\\D*(\\d+)").ok()
        && let Some(cap) = re.captures(&e.content)
        && let Some(m) = cap.get(1) {
        let v: u8 = m.as_str().parse().unwrap_or(0);
        signals.push(("Disk performance degraded".to_string(), v));
        score += v as u32;
    }
    let capped = score.min(100) as u8;
    (capped, signals)
}

pub fn generate_recommendations(hints: &[crate::hints::NoviceHint]) -> Vec<String> {
    let mut recs: Vec<String> = Vec::new();
    let any = |cat: &str| hints.iter().any(|h| h.category == cat);
    let any_msg = |contains: &str| hints.iter().any(|h| h.message.to_lowercase().contains(contains));
    if any("Storage") {
        recs.push("Back up important data immediately".to_string());
        recs.push("Run disk SMART and surface tests; replace drive if SMART shows failures".to_string());
    }
    if any("Hardware") || any_msg("machine check") {
        recs.push("Run memory diagnostics and CPU stress test; ensure adequate cooling".to_string());
    }
    if any("Cooling") || any("Thermal") {
        recs.push("Clean dust and verify fans; consider repasting CPU/GPU if temperatures remain high".to_string());
    }
    if any("Network") {
        recs.push("Check DNS settings; test with public DNS; inspect NIC drivers".to_string());
    }
    if any("Services") {
        recs.push("Review failing services; check dependencies and startup type".to_string());
    }
    if any("Policy") || any("Permissions") {
        recs.push("Review Group Policy and DCOM permissions; align with security baselines".to_string());
    }
    if any("GPU") {
        recs.push("Update GPU drivers; monitor for TDRs; consider lowering overclock".to_string());
    }
    recs.truncate(8);
    recs
}

pub fn compute_root_causes(hints: &[crate::hints::NoviceHint]) -> Vec<String> {
    let mut causes: Vec<String> = Vec::new();
    if hints.iter().any(|h| h.category == "Storage" && h.severity == "high") { causes.push("Storage subsystem instability or failing disk".to_string()); }
    if hints.iter().any(|h| h.category == "Hardware" && h.severity == "high") { causes.push("Underlying hardware fault (CPU/Memory/Bus)".to_string()); }
    if hints.iter().any(|h| h.category == "Thermal" || h.category == "Cooling") { causes.push("Thermal issues causing throttling and errors".to_string()); }
    if hints.iter().any(|h| h.category == "Network") { causes.push("Network/DNS misconfiguration or intermittent connectivity".to_string()); }
    if hints.iter().any(|h| h.category == "Policy" || h.category == "Permissions") { causes.push("Policy/permission misconfiguration impacting services".to_string()); }
    if causes.is_empty() { causes.push("General system instability indicated by error patterns".to_string()); }
    causes.truncate(5);
    causes
}

pub fn compute_timeline(events: &[EventItem], since: DateTime<Utc>, until: DateTime<Utc>) -> Vec<(String, usize, usize)> {
    let span = until - since;
    let bucket_hours = if span.num_days() >= 2 { 24 } else { 1 };
    let mut buckets: std::collections::BTreeMap<String, (usize, usize)> = std::collections::BTreeMap::new();
    for e in events {
        let dt = e.time;
        let key = if bucket_hours >= 24 {
            dt.format("%Y-%m-%d").to_string()
        } else {
            dt.format("%Y-%m-%d %H:00").to_string()
        };
        let entry = buckets.entry(key).or_insert((0, 0));
        if e.level == 2 { entry.0 += 1; } else if e.level == 3 { entry.1 += 1; }
    }
    buckets.into_iter().map(|(k,(e,w))| (k, e, w)).collect()
}

pub fn compute_by_category(hints: &[crate::hints::NoviceHint]) -> Vec<(String, usize)> {
    let mut m: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for h in hints { *m.entry(h.category.clone()).or_insert(0) += h.count.max(1); }
    let mut v: Vec<(String, usize)> = m.into_iter().collect();
    v.sort_by(|a,b| b.1.cmp(&a.1));
    v
}
