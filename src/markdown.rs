use crate::{ReportSummary, TimeZone};
use chrono::Local;

pub fn render_markdown(rep: &ReportSummary, tz: TimeZone, tfmt: Option<&str>) -> String {
    let (start_s, end_s) = match (tz, tfmt) {
        (TimeZone::Local, Some(f)) => (
            format!("{}", rep.window_start.with_timezone(&Local).format(f)),
            format!("{}", rep.window_end.with_timezone(&Local).format(f)),
        ),
        (TimeZone::Utc, Some(f)) => (
            format!("{}", rep.window_start.format(f)),
            format!("{}", rep.window_end.format(f)),
        ),
        (TimeZone::Local, None) => (
            format!("{}", rep.window_start.with_timezone(&Local).format("%Y-%m-%d %H:%M")),
            format!("{}", rep.window_end.with_timezone(&Local).format("%Y-%m-%d %H:%M")),
        ),
        (TimeZone::Utc, None) => (
            format!("{}", rep.window_start.format("%Y-%m-%d %H:%M")),
            format!("{}", rep.window_end.format("%Y-%m-%d %H:%M")),
        ),
    };
    let mut s = String::new();
    s.push_str("# WinDoctor Report\n\n");
    s.push_str(&format!("Time Window: {} → {}\n\n", start_s, end_s));
    if let Some(m) = rep.mode.as_ref() { s.push_str(&format!("Mode: {}\n\n", m)); }
    s.push_str(&format!("Risk: {}\n", rep.risk_grade));
    s.push_str(&format!("Performance Score: {}\n\n", rep.performance_score));

    s.push_str("## Summary\n");
    s.push_str(&format!("- Total Events: {}\n", rep.total));
    s.push_str(&format!("- Errors: {}\n", rep.errors));
    s.push_str(&format!("- Warnings: {}\n\n", rep.warnings));

    s.push_str("## Key Sources\n");
    if rep.by_provider.is_empty() { s.push_str("- Providers: None\n"); } else { for (p,c) in &rep.by_provider { s.push_str(&format!("- {} ({})\n", p, c)); } }
    if rep.by_channel.is_empty() { s.push_str("- Channels: None\n"); } else { for (ch,c) in &rep.by_channel { s.push_str(&format!("- {} ({})\n", ch, c)); } }
    if rep.by_event_id.is_empty() { s.push_str("- Common Event IDs: None\n\n"); } else { s.push_str("- Common Event IDs:\n"); for (id,c) in &rep.by_event_id { s.push_str(&format!("  - {} ({})\n", id, c)); } s.push('\n'); }

    s.push_str("## Diagnostics\n");
    if rep.novice_hints.is_empty() { s.push_str("None\n\n"); } else {
        for h in &rep.novice_hints {
            let ev = if h.evidence.is_empty() { String::new() } else { format!(" — Examples: {}", h.evidence.join(", ")) };
            s.push_str(&format!("- [{} {}%] {} ({} occurrences){}\n", h.severity, h.probability, h.message.replace('\n', " "), h.count, ev));
        }
        s.push('\n');
    }

    if !rep.degradation_signals.is_empty() {
        s.push_str("## Degradation Signals\n");
        for (n,w) in &rep.degradation_signals { s.push_str(&format!("- {} (weight {})\n", n, w)); }
        s.push('\n');
    }

    if !rep.recommendations.is_empty() {
        s.push_str("## Recommendations\n");
        for r in &rep.recommendations { s.push_str(&format!("- {}\n", r)); }
        s.push('\n');
    }

    if !rep.timeline.is_empty() {
        s.push_str("## Timeline\n");
        for (t,e,w) in &rep.timeline { s.push_str(&format!("- {}  Errors: {}  Warnings: {}\n", t, e, w)); }
        s.push('\n');
    }

    if !rep.perf_metrics.is_empty() {
        s.push_str("## Performance Metrics\n");
        for (name, avg, max, count) in &rep.perf_metrics { s.push_str(&format!("- {}: avg {} ms, max {} ms ({} samples)\n", name, avg, max, count)); }
        s.push('\n');
    }

    if let Some(pc) = &rep.perf_counters {
        s.push_str("## Live Performance\n");
        if let Some(v) = pc.cpu_percent { s.push_str(&format!("- CPU: {}%\n", v)); }
        if let Some(v) = pc.avg_disk_ms_per_transfer { s.push_str(&format!("- Avg Disk Transfer: {:.2} ms\n", v)); }
        if let Some(v) = pc.disk_reads_per_sec { s.push_str(&format!("- Reads/s: {}\n", v)); }
        if let Some(v) = pc.disk_writes_per_sec { s.push_str(&format!("- Writes/s: {}\n", v)); }
        s.push('\n');
    }

    if let Some(pred) = rep.smart_failure_predicted && pred { s.push_str("## SMART\n- Predicts failure on one or more drives\n\n"); }
    s
}

pub fn render_fix_markdown(rep: &ReportSummary, tz: TimeZone, tfmt: Option<&str>) -> String {
    let (start_s, end_s) = match (tz, tfmt) {
        (TimeZone::Local, Some(f)) => (
            format!("{}", rep.window_start.with_timezone(&Local).format(f)),
            format!("{}", rep.window_end.with_timezone(&Local).format(f)),
        ),
        (TimeZone::Utc, Some(f)) => (
            format!("{}", rep.window_start.format(f)),
            format!("{}", rep.window_end.format(f)),
        ),
        (TimeZone::Local, None) => (
            format!("{}", rep.window_start.with_timezone(&Local).format("%Y-%m-%d %H:%M")),
            format!("{}", rep.window_end.with_timezone(&Local).format("%Y-%m-%d %H:%M")),
        ),
        (TimeZone::Utc, None) => (
            format!("{}", rep.window_start.format("%Y-%m-%d %H:%M")),
            format!("{}", rep.window_end.format("%Y-%m-%d %H:%M")),
        ),
    };
    let mut s = String::new();
    s.push_str("# WinDoctor Fix-It\n\n");
    s.push_str(&format!("Time Window: {} → {}\n\n", start_s, end_s));
    s.push_str(&format!("Risk: {}\n\n", rep.risk_grade));
    s.push_str("## Likely Root Causes\n");
    if rep.likely_causes.is_empty() { s.push_str("- None detected\n\n"); } else { for c in &rep.likely_causes { s.push_str(&format!("- {}\n", c)); } s.push('\n'); }
    s.push_str("## Recommendations\n");
    if rep.recommendations.is_empty() { s.push_str("- No specific actions\n\n"); } else { for r in &rep.recommendations { s.push_str(&format!("- [ ] {}\n", r)); } s.push('\n'); }
    s.push_str("## Performance\n");
    s.push_str(&format!("- Score: {}\n", rep.performance_score));
    if !rep.perf_metrics.is_empty() { for (name, avg, max, count) in &rep.perf_metrics { s.push_str(&format!("- {}: avg {} ms, max {} ms ({} samples)\n", name, avg, max, count)); } }
    if let Some(pc) = &rep.perf_counters {
        if let Some(v) = pc.cpu_percent { s.push_str(&format!("- CPU: {}%\n", v)); }
        if let Some(v) = pc.avg_disk_ms_per_transfer { s.push_str(&format!("- Avg Disk Transfer: {:.2} ms\n", v)); }
        if let Some(v) = pc.disk_reads_per_sec { s.push_str(&format!("- Reads/s: {}\n", v)); }
        if let Some(v) = pc.disk_writes_per_sec { s.push_str(&format!("- Writes/s: {}\n", v)); }
    }
    if let Some(pred) = rep.smart_failure_predicted && pred { s.push_str("- SMART: Predicts failure on one or more drives\n"); }
    s
}
