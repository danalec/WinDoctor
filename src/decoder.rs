
pub fn decode_event(provider: &str, event_id: u32, xml: &str) -> Option<String> {
    let m = crate::event_xml::event_data_pairs_or_fallback(xml);
    match provider {
        "Service Control Manager" => {
            let svc = m.get("ServiceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
            match event_id {
                7000 => Some(format!("Service failed to start: {}", svc)),
                7001 => Some(format!("Service dependent failed to start: {}", svc)),
                7009 => Some(format!("Service start timed out: {}", svc)),
                7011 => Some(format!("Service hung or timeout occurred: {}", svc)),
                7023 => Some(format!("Service terminated with error: {}", svc)),
                7031 => Some(format!("Service terminated unexpectedly: {}", svc)),
                7034 => Some(format!("Service terminated unexpectedly: {}", svc)),
                _ => if !svc.is_empty() { Some(format!("SCM {} {}", event_id, svc)) } else { None }
            }
        }
        "Disk" => {
            match event_id {
                7 => {
                    let dev = m.get("DeviceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
                    Some(format!("Bad block detected on {}", dev))
                }
                11 => {
                    let dev = m.get("DeviceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
                    Some(format!("Disk or controller error on {}", dev))
                }
                51 => Some("Paging I/O error indicates unstable storage path".to_string()),
                157 => {
                    let dev = m.get("DeviceName").or_else(|| m.get("param1")).cloned().unwrap_or_default();
                    Some(format!("Disk was surprise removed: {}", dev))
                }
                _ => m.get("DeviceName").or_else(|| m.get("param1")).map(|dev| format!("Disk {}", dev))
            }
        }
        "DistributedCOM" => {
            let clsid = m.get("CLSID").cloned().unwrap_or_default();
            let appid = m.get("APPID").cloned().unwrap_or_default();
            if !clsid.is_empty() || !appid.is_empty() { return Some(format!("DCOM CLSID={} APPID={}", clsid, appid)); }
            None
        }
        "Schannel" => {
            let code = m.get("ErrorCode").cloned().unwrap_or_default();
            if !code.is_empty() { return Some(format!("Schannel ErrorCode={}", code)); }
            None
        }
        "Microsoft-Windows-WER-SystemErrorReporting" => {
            let bug = m.get("BugcheckCode").cloned().unwrap_or_default();
            if !bug.is_empty() { return Some(format!("BugCheck {}", bug)); }
            None
        }
        "Microsoft-Windows-Ntfs" => {
            Some(match event_id {
                55 => "File system corruption detected (NTFS)".to_string(),
                57 => "Delayed write failed (NTFS)".to_string(),
                140 => "Failed to flush data to transaction log (NTFS)".to_string(),
                _ => return None,
            })
        }
        "Microsoft-Windows-Kernel-Power" => {
            if event_id == 41 { return Some("Unexpected shutdown or power loss detected".to_string()); }
            None
        }
        "EventLog" => {
            if event_id == 6008 { return Some("Previous system shutdown was unexpected".to_string()); }
            None
        }
        "Microsoft-Windows-WHEA-Logger" => {
            match event_id {
                18 => {
                    let src = m.get("ErrorSource").cloned().unwrap_or_default();
                    let apic = m.get("ApicId").or_else(|| m.get("ProcessorAPICID")).cloned().unwrap_or_default();
                    let ev = if apic.is_empty() { src } else { format!("{} APIC {}", src, apic) };
                    Some(format!("Uncorrected hardware error ({} )", ev))
                }
                17 => {
                    let comp = m.get("Component").cloned().unwrap_or_default();
                    let dev = m.get("DeviceId").cloned().unwrap_or_default();
                    let ev = if comp.is_empty() { dev } else { comp };
                    Some(format!("Corrected hardware error ({})", ev))
                }
                19 | 20 => {
                    let src = m.get("ErrorSource").cloned().unwrap_or_default();
                    Some(format!("Hardware error reported by WHEA ({})", src))
                }
                _ => None
            }
        }
        "Display" => {
            if event_id == 4101 { return Some("Display driver stopped responding and recovered".to_string()); }
            None
        }
        "volmgr" => {
            let c = xml.to_lowercase();
            if c.contains("failed to flush data to the transaction log") { return Some("Volume manager flush failure – potential corruption".to_string()); }
            None
        }
        "volsnap" => {
            let c = xml.to_lowercase();
            if c.contains("shadow copies of volume") && c.contains("were aborted") { return Some("Shadow copies aborted – may indicate underlying disk issues".to_string()); }
            None
        }
        "Microsoft-Windows-DNS-Client" => {
            if event_id == 1014 {
                let q = m.get("QueryName").cloned().unwrap_or_default();
                if q.is_empty() { return Some("DNS name resolution failure".to_string()); }
                return Some(format!("DNS name resolution failure: {}", q));
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_map_parses_device_name() {
        let xml = "<Event><EventData><Data Name=\"DeviceName\">\\\\.\\PHYSICALDRIVE3</Data></EventData></Event>";
        let m = crate::event_xml::event_data_pairs(xml);
        assert_eq!(m.get("DeviceName").unwrap(), "\\\\.\\PHYSICALDRIVE3");
    }

    #[test]
    fn scm_event_7000_maps_message() {
        let xml = "<Event><EventData><Data Name=\"ServiceName\">Spooler</Data></EventData></Event>";
        let msg = decode_event("Service Control Manager", 7000, xml).unwrap();
        assert!(msg.contains("Service failed to start"));
        assert!(msg.contains("Spooler"));
    }

    #[test]
    fn eventlog_6008_maps_message() {
        let xml = "<Event><EventData></EventData></Event>";
        let msg = decode_event("EventLog", 6008, xml).unwrap();
        assert!(msg.contains("Previous system shutdown was unexpected"));
    }

    #[test]
    fn dns_client_1014_includes_query_name() {
        let xml = "<Event><EventData><Data Name=\"QueryName\">example.com</Data></EventData></Event>";
        let msg = decode_event("Microsoft-Windows-DNS-Client", 1014, xml).unwrap();
        assert!(msg.contains("example.com"));
    }
}
