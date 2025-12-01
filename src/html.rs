use crate::{ReportSummary, EventItem, TimeZone};

pub fn render_html(rep: &ReportSummary, theme: crate::Theme, use_emoji: bool, tz: TimeZone, tfmt: Option<&str>) -> String {
    let mut s = String::new();
    s.push_str("<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>WinDoctor Report</title><style>");
    match theme {
        crate::Theme::Dark => s.push_str(":root{--bg:#0a0e13;--fg:#ffffff;--muted:#c0c4cc;--card:#0d131a;--border:#243041;--accent:#3b82f6;--ok:#22c55e;--warn:#f59e0b;--err:#ef4444;--chip:#0f172a} body{margin:0;background:var(--bg);color:var(--fg);font-family:Segoe UI,system-ui,-apple-system,Arial,sans-serif} .container{max-width:1200px;margin:0 auto;padding:24px} .header{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:16px} .title{font-size:20px;font-weight:600;letter-spacing:.2px} .sub{color:var(--muted);font-size:13px} .grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px} .card{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:14px;box-shadow:0 1px 0 rgba(255,255,255,.03) inset} .metric{display:flex;align-items:center;justify-content:space-between} .metric .label{color:var(--muted);font-size:12px} .metric .value{font-size:22px;font-weight:700} .value.err{color:var(--err)} .value.warn{color:var(--warn)} .value.ok{color:var(--ok)} .section{margin-top:18px} .section h3{margin:0 0 10px 0;font-size:16px;font-weight:600} .table{width:100%;border-collapse:separate;border-spacing:0;background:var(--card);border:1px solid var(--border);border-radius:10px;overflow:hidden} .table th{position:sticky;top:0;background:#0c1118;color:#ffffff;text-align:left;font-weight:600;padding:10px;border-bottom:1px solid var(--border)} .table td{padding:10px;border-bottom:1px solid var(--border);vertical-align:top} .table tr:nth-child(odd) td{background:#0b0f14} .chip{display:inline-flex;align-items:center;gap:6px;background:var(--chip);color:#ffffff;border:1px solid var(--border);border-radius:999px;padding:4px 10px;font-size:12px} .sev-high{background:#3a0f12;color:#ffffff;border-color:#7f1d1d} .sev-medium{background:#3a2b0d;color:#ffffff;border-color:#854d0e} .sev-low{background:#0f1a2b;color:#ffffff;border-color:#1e3a8a} .pill{display:inline-block;background:#0c1118;color:#ffffff;padding:6px 10px;border-radius:999px;border:1px solid var(--border);font-size:12px;margin:4px 6px 0 0} .code{font-family:Consolas,Monaco,monospace;background:#091017;border:1px solid var(--border);border-radius:8px;padding:10px;margin-top:8px;white-space:pre-wrap} .ellipsis{display:block;max-width:900px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis} details summary{cursor:pointer;color:var(--accent)} .split{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px} .footer{margin-top:22px;color:var(--muted);font-size:12px} @media (max-width:900px){.grid{grid-template-columns:repeat(2,minmax(0,1fr))}.split{grid-template-columns:1fr}} @media (max-width:600px){.grid{grid-template-columns:1fr}.header{flex-direction:column;align-items:flex-start}}"),
        crate::Theme::Light => s.push_str(":root{--bg:#f7fafc;--fg:#111827;--muted:#6b7280;--card:#ffffff;--border:#e5e7eb;--accent:#2563eb;--ok:#16a34a;--warn:#d97706;--err:#dc2626;--chip:#eef2f7} body{margin:0;background:var(--bg);color:var(--fg);font-family:Segoe UI,system-ui,-apple-system,Arial,sans-serif} .container{max-width:1200px;margin:0 auto;padding:24px} .header{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:16px} .title{font-size:20px;font-weight:600;letter-spacing:.2px} .sub{color:var(--muted);font-size:13px} .grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px} .card{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:14px;box-shadow:0 1px 0 rgba(0,0,0,.04)} .metric{display:flex;align-items:center;justify-content:space-between} .metric .label{color:var(--muted);font-size:12px} .metric .value{font-size:22px;font-weight:700} .value.err{color:var(--err)} .value.warn{color:var(--warn)} .value.ok{color:var(--ok)} .section{margin-top:18px} .section h3{margin:0 0 10px 0;font-size:16px;font-weight:600} .table{width:100%;border-collapse:separate;border-spacing:0;background:var(--card);border:1px solid var(--border);border-radius:10px;overflow:hidden} .table th{position:sticky;top:0;background:#f3f4f6;color:var(--fg);text-align:left;font-weight:600;padding:10px;border-bottom:1px solid var(--border)} .table td{padding:10px;border-bottom:1px solid var(--border);vertical-align:top} .table tr:nth-child(odd) td{background:#fbfdff} .chip{display:inline-flex;align-items:center;gap:6px;background:var(--chip);color:var(--fg);border:1px solid var(--border);border-radius:999px;padding:4px 10px;font-size:12px} .sev-high{background:#fee2e2;color:#7f1d1d;border-color:#fecaca} .sev-medium{background:#fde68a;color:#854d0e;border-color:#fef3c7} .sev-low{background:#dbeafe;color:#1e3a8a;border-color:#bfdbfe} .pill{display:inline-block;background:#eef2f7;color:var(--fg);padding:6px 10px;border-radius:999px;border:1px solid var(--border);font-size:12px;margin:4px 6px 0 0} .code{font-family:Consolas,Monaco,monospace;background:#f3f4f6;border:1px solid var(--border);border-radius:8px;padding:10px;margin-top:8px;white-space:pre-wrap} .ellipsis{display:block;max-width:900px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis} details summary{cursor:pointer;color:var(--accent)} .split{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px} .footer{margin-top:22px;color:var(--muted);font-size:12px} @media (max-width:900px){.grid{grid-template-columns:repeat(2,minmax(0,1fr))}.split{grid-template-columns:1fr}} @media (max-width:600px){.grid{grid-template-columns:1fr}.header{flex-direction:column;align-items:flex-start}}"),
    }
    s.push_str("</style><script>(function(){const light={bg:'#f7fafc',fg:'#111827',muted:'#6b7280',card:'#ffffff',border:'#e5e7eb',accent:'#2563eb',ok:'#16a34a',warn:'#d97706',err:'#dc2626',chip:'#eef2f7'};const dark={bg:'#0f1216',fg:'#e5e7eb',muted:'#9aa0a6',card:'#141820',border:'#1f2430',accent:'#3b82f6',ok:'#22c55e',warn:'#f59e0b',err:'#ef4444',chip:'#1f2937'};function apply(vars){const r=document.documentElement.style;Object.entries(vars).forEach(([k,v])=>r.setProperty('--'+k,v));document.body.style.background='var(--bg)';document.body.style.color='var(--fg)';}window.__wdTheme=window.__wdTheme||'';window.toggleTheme=function(){const curr=window.__wdTheme==='light'?'dark':'light';window.__wdTheme=curr;apply(curr==='light'?light:dark);const btn=document.getElementById('themeToggle');if(btn){btn.textContent=curr==='light'?'Dark Mode':'Light Mode';}};window.copyRowMessage=function(btn){const tr=btn.closest('tr');if(!tr)return;const el=tr.querySelector('.full-msg');if(!el)return;const txt=el.textContent||'';if(navigator.clipboard){navigator.clipboard.writeText(txt).then(()=>{btn.textContent='Copied!';setTimeout(()=>btn.textContent='Copy',1500);});}};})();</script></head><body><div class=\"container\">");
    s.push_str("<div class=\"header\"><div class=\"title\">WinDoctor Report</div>");
    let start_s = match (tz, tfmt) { (TimeZone::Local, Some(f)) => rep.window_start.with_timezone(&chrono::Local).format(f).to_string(), (TimeZone::Utc, Some(f)) => rep.window_start.format(f).to_string(), (TimeZone::Local, None) => rep.window_start.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(), (TimeZone::Utc, None) => rep.window_start.format("%Y-%m-%d %H:%M").to_string() };
    let end_s = match (tz, tfmt) { (TimeZone::Local, Some(f)) => rep.window_end.with_timezone(&chrono::Local).format(f).to_string(), (TimeZone::Utc, Some(f)) => rep.window_end.format(f).to_string(), (TimeZone::Local, None) => rep.window_end.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(), (TimeZone::Utc, None) => rep.window_end.format("%Y-%m-%d %H:%M").to_string() };
    s.push_str(&format!("<div class=\"sub\">{} → {}{} <span class=\"pill\">Risk · {}</span></div>", start_s, end_s, match rep.mode.as_ref(){Some(m)=>format!(" | {}", m),None=>String::new()}, html_escape(&rep.risk_grade)));
    s.push_str(&format!("<button id=\"themeToggle\" class=\"pill\" onclick=\"toggleTheme()\">{}</button>", match theme { crate::Theme::Light => "Dark Mode", _ => "Light Mode" }));
    s.push_str("</div>");
    if !rep.by_category.is_empty() {
        s.push_str("<div class=\"section\"><h3>Impact Assessment</h3><div class=\"card\">");
        for (cat,cnt) in &rep.by_category { s.push_str(&format!("<span class=\"pill\">{} · {}</span>", html_escape(cat), cnt)); }
        s.push_str("</div></div>");
    }
    if !rep.likely_causes.is_empty() {
        s.push_str("<div class=\"section\"><h3>Likely Root Causes</h3><div class=\"card\">");
        for c in &rep.likely_causes { s.push_str(&format!("<div class=\"pill\">{}</div>", html_escape(c))); }
        s.push_str("</div></div>");
    }
    s.push_str("<div class=\"grid\">");
    s.push_str(&format!("<div class=\"card metric\"><div class=\"label\">Total Events</div><div class=\"value\">{}</div></div>", rep.total));
    s.push_str(&format!("<div class=\"card metric\"><div class=\"label\">Errors</div><div class=\"value err\">{}</div></div>", rep.errors));
    s.push_str(&format!("<div class=\"card metric\"><div class=\"label\">Warnings</div><div class=\"value warn\">{}</div></div>", rep.warnings));
    s.push_str(&format!("<div class=\"card metric\"><div class=\"label\">Providers</div><div class=\"value\">{}</div></div>", rep.by_provider.len()));
    s.push_str(&format!("<div class=\"card metric\"><div class=\"label\">Performance Score</div><div class=\"value\">{}</div></div>", rep.performance_score));
    let risk_cls = match rep.risk_grade.as_str(){"Critical"=>"value err","High"=>"value err","Medium"=>"value warn",_=>"value ok"};
    s.push_str(&format!("<div class=\"card metric\"><div class=\"label\">Risk</div><div class=\"{}\">{}</div></div>", risk_cls, rep.risk_grade));
    s.push_str("</div>");
    if !rep.novice_hints.is_empty() {
        s.push_str("<div class=\"section\"><h3>Diagnostics</h3><table class=\"table\"><thead><tr><th>Category</th><th>Severity</th><th>Probability</th><th>Message</th><th>Occurrences</th><th>Examples</th></tr></thead><tbody>");
        for h in &rep.novice_hints {
            let sev_cls = match h.severity.as_str(){"high"=>"sev-high","medium"=>"sev-medium",_=>"sev-low"}.to_string();
            let sev_emoji = if use_emoji { match h.severity.as_str(){"high"=>"⛔","medium"=>"⚠️",_=>"🛈"} } else { "" };
            let ex = if h.evidence.is_empty(){String::new()} else { h.evidence.join(", ") };
            s.push_str(&format!("<tr><td>{}</td><td><span class=\"chip {}\">{} {}</span></td><td>{}%</td><td>{}</td><td>{}</td><td>{}</td></tr>", h.category, sev_cls, sev_emoji, h.severity, h.probability, html_escape(&h.message), h.count, html_escape(&ex)));
        }
        s.push_str("</tbody></table></div>");
    }
    if let Some(pc) = &rep.perf_counters {
        s.push_str("<div class=\"section\"><h3>Live Performance</h3><div class=\"card\">");
        if let Some(v) = pc.cpu_percent { s.push_str(&format!("<span class=\"pill\">CPU · {}%</span>", v)); }
        if let Some(v) = pc.avg_disk_ms_per_transfer { s.push_str(&format!("<span class=\"pill\">Avg Disk Transfer · {:.2} ms</span>", v)); }
        if let Some(v) = pc.disk_reads_per_sec { s.push_str(&format!("<span class=\"pill\">Reads/s · {}</span>", v)); }
        if let Some(v) = pc.disk_writes_per_sec { s.push_str(&format!("<span class=\"pill\">Writes/s · {}</span>", v)); }
        s.push_str("</div></div>");
    }
    if let Some(pred) = rep.smart_failure_predicted && pred { s.push_str("<div class=\"section\"><div class=\"card\"><div class=\"value err\">SMART predicts failure on one or more drives</div></div></div>"); }
    if !rep.perf_metrics.is_empty() {
        s.push_str("<div class=\"section\"><h3>Performance Details</h3><table class=\"table\"><thead><tr><th>Metric</th><th>Average (ms)</th><th>Max (ms)</th><th>Samples</th></tr></thead><tbody>");
        for (name, avg, max, cnt) in &rep.perf_metrics { s.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", html_escape(name), avg, max, cnt)); }
        s.push_str("</tbody></table></div>");
    }
    s.push_str("<div class=\"section split\">");
    s.push_str("<div class=\"card\"><h3>Top Providers</h3><table class=\"table\"><thead><tr><th>Provider</th><th>Count</th></tr></thead><tbody>");
    for (p,c) in &rep.by_provider { s.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>", html_escape(p), c)); }
    s.push_str("</tbody></table></div>");
    s.push_str("<div class=\"card\"><h3>Top Domains</h3><table class=\"table\"><thead><tr><th>Domain</th><th>Count</th></tr></thead><tbody>");
    for (d,c) in &rep.by_domain { s.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>", html_escape(d), c)); }
    s.push_str("</tbody></table></div>");
    s.push_str("<div class=\"card\"><h3>Top Devices</h3><table class=\"table\"><thead><tr><th>Device</th><th>Count</th></tr></thead><tbody>");
    for (d,c) in &rep.by_device { s.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>", html_escape(d), c)); }
    s.push_str("</tbody></table></div>");
    s.push_str("<div class=\"card\"><h3>Top Event IDs</h3><table class=\"table\"><thead><tr><th>Event ID</th><th>Count</th></tr></thead><tbody>");
    for (id,c) in &rep.by_event_id { s.push_str(&format!("<tr><td>{}</td><td>{}</td></tr>", id, c)); }
    s.push_str("</tbody></table></div>");
    s.push_str("</div>");
    if !rep.perf_metrics.is_empty() {
        s.push_str("<div class=\"section\"><h3>Performance Metrics</h3><table class=\"table\"><thead><tr><th>Phase</th><th>Avg (ms)</th><th>Max (ms)</th><th>Count</th></tr></thead><tbody>");
        for (name, avg, max, count) in &rep.perf_metrics { s.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", html_escape(name), avg, max, count)); }
        s.push_str("</tbody></table></div>");
    }
    if !rep.degradation_signals.is_empty() {
        s.push_str("<div class=\"section\"><h3>Degradation Signals</h3><div class=\"card\">");
        for (n,w) in &rep.degradation_signals { s.push_str(&format!("<span class=\"pill\">{} · weight {}</span>", html_escape(n), w)); }
        s.push_str("</div></div>");
    }
    if !rep.matched_terms.is_empty() {
        s.push_str("<div class=\"section\"><h3>Matched Keywords</h3><div class=\"card\">");
        for (t,c) in &rep.matched_terms { s.push_str(&format!("<span class=\"pill\">{} · {}</span>", html_escape(t), c)); }
        s.push_str("</div></div>");
    }
    s.push_str("<div class=\"section\"><h3>Recent Samples</h3><table class=\"table\"><thead><tr><th>Time</th><th>Channel</th><th>Provider</th><th>Device</th><th>Event ID</th><th>Cause</th><th>Data</th><th>Message</th><th>Actions</th></tr></thead><tbody>");
    for e in &rep.samples {
        let ts = match (tz, tfmt) { (TimeZone::Local, Some(f)) => e.time.with_timezone(&chrono::Local).format(f).to_string(), (TimeZone::Utc, Some(f)) => e.time.format(f).to_string(), (TimeZone::Local, None) => e.time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(), (TimeZone::Utc, None) => e.time.format("%Y-%m-%d %H:%M").to_string() };
        let msg = &e.content;
        let truncated = truncate_chars(msg, 240);
        let dev_raw = device_from(e).unwrap_or_default();
        let dev = crate::device_map::friendly_device(&dev_raw).unwrap_or(dev_raw);
        let sel = selected_data_from(e);
        let mut data_cell = String::new();
        if sel.is_empty() { data_cell.push_str("<span class=\"sub\">None</span>"); } else {
            for (k,v) in sel.into_iter().take(3) { data_cell.push_str(&format!("<span class=\"pill\">{} · {}</span> ", html_escape(&k), html_escape(&v))); }
        }
        if msg.chars().count() > 240 {
            s.push_str(&format!("<tr><td class=\"sub\">{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><span class=\"ellipsis\">{}</span><details><summary>Show full</summary><div class=\"code\">{}</div></details><span class=\"full-msg\" style=\"display:none\">{}</span></td><td><button class=\"pill\" onclick=\"copyRowMessage(this)\">Copy</button></td></tr>", ts, html_escape(&e.channel), html_escape(&e.provider), html_escape(&dev), e.event_id, html_escape(&cause_from(e)), data_cell, html_escape(&truncated), html_escape(msg), html_escape(msg)));
        } else {
            s.push_str(&format!("<tr><td class=\"sub\">{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><button class=\"pill\" onclick=\"copyRowMessage(this)\">Copy</button><span class=\"full-msg\" style=\"display:none\">{}</span></td></tr>", ts, html_escape(&e.channel), html_escape(&e.provider), html_escape(&dev), e.event_id, html_escape(&cause_from(e)), data_cell, html_escape(msg), html_escape(msg)));
        }
    }
    s.push_str("</tbody></table></div>");
    if !rep.recommendations.is_empty() {
        s.push_str("<div class=\"section\"><h3>Recommendations</h3><div class=\"card\">");
        for r in &rep.recommendations { s.push_str(&format!("<div class=\"pill\">{}</div>", html_escape(r))); }
        s.push_str("</div></div>");
    }
    s.push_str("<div class=\"section\"><h3>Tools & References</h3><div class=\"card\">");
    s.push_str("<a href=\"https://support.microsoft.com/en-us/windows/check-your-drive-for-errors-in-windows-10-9a7773b1-1f89-2df4-6f48-49706027fea8\" class=\"pill\">chkdsk</a> ");
    s.push_str("<a href=\"https://learn.microsoft.com/en-us/windows-hardware/test/wpt/\" class=\"pill\">Windows Performance Toolkit</a> ");
    s.push_str("<a href=\"https://crystalmark.info/en/software/crystaldiskinfo/\" class=\"pill\">CrystalDiskInfo</a> ");
    s.push_str("<a href=\"https://learn.microsoft.com/en-us/windows/win32/wmisdk/wmi-start-page\" class=\"pill\">WMI</a> ");
    s.push_str("</div></div>");
    if !rep.recommendations.is_empty() {
        s.push_str("<div class=\"section\"><h3>Checklist</h3><div class=\"card\">");
        for r in &rep.recommendations { s.push_str(&format!("<div><input type=\"checkbox\"/> {}</div>", html_escape(r))); }
        s.push_str("</div></div>");
    }
    if !rep.timeline.is_empty() {
        let max_e = rep.timeline.iter().map(|(_,e,_)| *e).max().unwrap_or(1);
        let max_w = rep.timeline.iter().map(|(_,_,w)| *w).max().unwrap_or(1);
        s.push_str("<div class=\"section\"><h3>Timeline</h3><div class=\"card\">");
        for (t,e,w) in &rep.timeline {
            let ew = if max_e == 0 { 0.0 } else { (*e as f64 / max_e as f64) * 100.0 };
            let ww = if max_w == 0 { 0.0 } else { (*w as f64 / max_w as f64) * 100.0 };
            s.push_str(&format!("<div style=\"display:flex;align-items:center;gap:8px;margin:6px 0\"><div class=\"sub\">{}</div><div style=\"flex:1;display:flex;gap:6px\"><div style=\"height:8px;border-radius:4px;background:var(--err);width:{:.0}%\"></div><div style=\"height:8px;border-radius:4px;background:var(--warn);width:{:.0}%\"></div></div><div class=\"sub\">E:{} · W:{}</div></div>", html_escape(t), ew, ww, e, w));
        }
        s.push_str("</div></div>");
    }
    if !rep.file_matched_terms.is_empty() || !rep.file_samples.is_empty() {
        s.push_str("<div class=\"section\"><h3>Files</h3>");
        if !rep.file_matched_terms.is_empty() {
            s.push_str("<div class=\"card\"><h3>Matched Keywords</h3>");
            for (t,c) in &rep.file_matched_terms { s.push_str(&format!("<span class=\"pill\">{} · {} files</span>", html_escape(t), c)); }
            s.push_str("</div>");
        }
        if !rep.file_samples.is_empty() {
            s.push_str("<div class=\"card\"><h3>Examples</h3><table class=\"table\"><thead><tr><th>Path</th><th>Pattern</th><th>Line</th><th>Content</th></tr></thead><tbody>");
            for s2 in &rep.file_samples {
                let msg = s2.line.replace('\n', " ");
                let truncated = truncate_chars(&msg, 160);
                if msg.chars().count() > 160 {
                    s.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td><span class=\"ellipsis\">{}</span><details><summary>Show full</summary><div class=\"code\">{}</div></details></td></tr>", html_escape(&s2.path), html_escape(&s2.pattern), s2.line_no, html_escape(&truncated), html_escape(&msg)));
                } else {
                    s.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", html_escape(&s2.path), html_escape(&s2.pattern), s2.line_no, html_escape(&msg)));
                }
            }
            s.push_str("</tbody></table></div>");
        }
        s.push_str("</div>");
    }
    s.push_str("<div class=\"footer\">Generated by WinDoctor</div></div><script>(function(){var init=");
    s.push_str(match theme { crate::Theme::Light => "'light'", _ => "'dark'" });
    s.push_str("; window.__wdTheme=init; toggleTheme();})();</script></body></html>");
    s
}

fn selected_data_from(e: &EventItem) -> Vec<(String,String)> {
    let pairs = crate::event_xml::event_data_pairs_or_fallback(&e.content);
    let keys = [
        "FaultingApplicationPath","FaultingModulePath","FaultingModuleName",
        "DeviceName","TargetDevice","Device","InstancePath","PhysicalDeviceObjectName",
        "CLSID","AppID","SID","QueryName","ServiceName","param1","param2"
    ];
    let mut out: Vec<(String,String)> = Vec::new();
    for k in keys.iter() {
        if let Some(v) = pairs.get(*k) { if !v.is_empty() { out.push((k.to_string(), v.clone())); } }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn truncate_chars(s: &str, n: usize) -> String { s.chars().take(n).collect() }

fn cause_from(e: &EventItem) -> String {
    let c = e.content.trim();
    if c.starts_with('<') || c.contains("<EventData>") { format!("{} {}", e.provider, e.event_id) } else { c.to_string() }
}

fn device_from(e: &EventItem) -> Option<String> {
    let pairs = crate::event_xml::event_data_pairs_or_fallback(&e.content);
    let keys = ["DeviceName", "TargetDevice", "Device", "InstancePath", "PhysicalDeviceObjectName"];
    for k in keys.iter() {
        if let Some(v) = pairs.get(*k) && !v.is_empty() { return Some(v.clone()); }
    }
    None
}
