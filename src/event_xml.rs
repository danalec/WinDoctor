use std::collections::HashMap;
use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

pub fn event_data_pairs(xml: &str) -> HashMap<String, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_event_data = false;
    let mut cur_name: Option<String> = None;
    let mut out: HashMap<String, String> = HashMap::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) => {
                let en = e.name();
                let name = String::from_utf8_lossy(en.as_ref()).into_owned();
                if name == "EventData" { in_event_data = true; }
                else if in_event_data && name == "Data" {
                    cur_name = None;
                    for a in e.attributes().flatten() {
                        let k = String::from_utf8_lossy(a.key.as_ref());
                        if k == "Name" && let Ok(val) = a.unescape_value() {
                            cur_name = Some(val.to_string());
                        }
                    }
                }
            }
            Ok(XmlEvent::End(e)) => {
                let en = e.name();
                let name = String::from_utf8_lossy(en.as_ref()).into_owned();
                if name == "EventData" { in_event_data = false; }
                if name == "Data" { cur_name = None; }
            }
            Ok(XmlEvent::Text(t)) => {
                if in_event_data && let Some(n) = cur_name.as_ref() {
                    let v = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                    if !v.is_empty() { out.insert(n.clone(), v); }
                }
            }
            Ok(XmlEvent::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

pub fn event_data_pairs_fallback(xml: &str) -> HashMap<String, String> {
    let mut res = HashMap::new();
    let mut rest = xml;
    while let Some(i) = rest.find("<Data ") {
        rest = &rest[i + 6..];
        if let Some(ns) = rest.find("Name=\"") {
            let after = &rest[ns + 6..];
            if let Some(ne) = after.find('"') {
                let name = &after[..ne];
                if let Some(gt) = after[ne..].find('>') {
                    let val_part = &after[ne + gt + 1..];
                    if let Some(ve) = val_part.find("</Data>") {
                        res.insert(name.to_string(), val_part[..ve].to_string());
                        rest = &val_part[ve + 7..];
                        continue;
                    }
                }
            }
        }
        break;
    }
    res
}

pub fn event_data_pairs_or_fallback(xml: &str) -> HashMap<String, String> {
    let m = event_data_pairs(xml);
    if m.is_empty() { event_data_pairs_fallback(xml) } else { m }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_eventdata_pairs() {
        let xml = "<Event><EventData><Data Name=\"DeviceName\">\\\\.\\PHYSICALDRIVE0</Data><Data Name=\"QueryName\">example.com</Data></EventData></Event>";
        let m = event_data_pairs_or_fallback(xml);
        assert_eq!(m.get("DeviceName").unwrap(), "\\\\.\\PHYSICALDRIVE0");
        assert_eq!(m.get("QueryName").unwrap(), "example.com");
    }
    #[test]
    fn fallback_parses_data_outside_eventdata() {
        let xml = "<Event><System><Data Name=\"Odd\">Value</Data></System></Event>";
        let m = event_data_pairs_or_fallback(xml);
        assert_eq!(m.get("Odd").unwrap(), "Value");
    }
}
