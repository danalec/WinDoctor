use std::path::PathBuf;
use std::sync::OnceLock;
use chrono::{DateTime, Duration, Utc, Local};
use clap::{Parser, ValueEnum, ColorChoice, ArgAction, CommandFactory};
use clap_complete::Shell;
use comfy_table::{Table, ContentArrangement};
use evtx::EvtxParser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;
use is_terminal::IsTerminal;
mod windows_live;
mod decoder;
mod html;
mod file_scan;
mod hints;
mod device_map;
mod rules;
mod event_xml;
mod markdown;
mod perf;

static ENABLE_COLOR: OnceLock<bool> = OnceLock::new();

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum OutputFmt { Text, Json }

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum TimeZone { Local, Utc }

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum SortBy { Time, Severity, Provider, Channel, EventId }

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum SortOrder { Desc, Asc }

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum Column { Time, Severity, Channel, Provider, EventId, Cause, Message }

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum Theme { Dark, Light }

#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum LogLevel { Error, Warn, Info, Debug, Trace }

#[derive(Parser, Debug)]
#[command(
    name = "WinDoctor",
    about = "Windows diagnostics and event log reporter",
    long_about = "Windows diagnostics and event log reporter that scans EVTX channels, summarizes issues, and can emit HTML/JSON reports.",
    after_long_help = "Examples:\n  WinDoctor --last10m --output text\n  WinDoctor --hours 6 --channels System,Application --top 50\n  WinDoctor --evtx-path C:\\Windows\\System32\\winevt\\Logs\\System.evtx --html report.html\n  WinDoctor --scan-path C:\\Logs --file-glob *.log --patterns error,timeout\n  WinDoctor --providers Disk --exclude-providers DistributedCOM --output json",
    color = ColorChoice::Auto
)]
struct Args {
    #[arg(long, short = 'm', default_value_t = 0)]
    minutes: i64,
    #[arg(long, default_value_t = 0)]
    hours: i64,
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    channels: Vec<String>,
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    patterns: Vec<String>,
    #[arg(long, short = 'n', default_value_t = 20)]
    top: usize,
    #[arg(long, short = 'o', value_enum, default_value = "text")]
    output: OutputFmt,
    #[arg(long, value_enum, default_value = "lines")]
    text_format: TextFormat,
    #[arg(long, value_enum, default_value = "dark")]
    theme: Theme,
    #[arg(long, short = 'L', default_value_t = false)]
    live: bool,
    #[arg(long, default_value_t = 0)]
    subscribe_minutes: u64,
    #[arg(long, default_value_t = false, help = "Shortcut: last 10 minutes", conflicts_with_all = ["minutes", "hours", "since", "until"])]
    last10m: bool,
    #[arg(long, default_value_t = false, help = "Shortcut: last day (24 hours)", conflicts_with_all = ["minutes", "hours", "since", "until"])]
    last_day: bool,
    #[arg(long, default_value_t = false, help = "Shortcut: last hour", conflicts_with_all = ["minutes", "hours", "since", "until"])]
    last_hour: bool,
    #[arg(long, default_value_t = false, help = "Shortcut: last week (7 days)", conflicts_with_all = ["minutes", "hours", "since", "until"])]
    last_week: bool,
    #[arg(long, default_value_t = false, help = "Include information level (4)")]
    include_info: bool,
    #[arg(long, default_value_t = false, help = "Disable level filtering (include all levels)")]
    no_level_filter: bool,
    #[arg(long)]
    html: Option<String>,
    #[arg(long, short = 's')]
    scan_path: Option<String>,
    #[arg(long, short = 'g')]
    file_glob: Option<String>,
    #[arg(long, default_value_t = 20)]
    max_file_samples: usize,
    #[arg(long, short = 'e')]
    evtx_path: Option<String>,
    #[arg(long)]
    evtx_glob: Option<String>,
    #[arg(long, default_value_t = false)]
    evtx_recursive: bool,
    #[arg(long, conflicts_with_all = ["last10m", "last_hour", "last_day", "last_week", "minutes", "hours"])]
    since: Option<String>,
    #[arg(long, conflicts_with_all = ["last10m", "last_hour", "last_day", "last_week", "minutes", "hours"])]
    until: Option<String>,
    /// Fetch last N error events (default 50; ignored if any time window flag is provided)
    #[arg(long, default_value_t = 50)]
    last_errors: usize,
    /// Fetch last N critical events (default 50; ignored if any time window flag is provided)
    #[arg(long, default_value_t = 50)]
    last_criticals: usize,
    /// Path to JSON rules registry (default ./rules.json)
    #[arg(long)]
    rules: Option<String>,
    #[arg(long, short = 'C', default_value_t = false)]
    no_color: bool,
    #[arg(long, default_value_t = false)]
    no_emoji: bool,
    #[arg(long)]
    log_level: Option<LogLevel>,
    #[arg(long, value_enum)]
    log_format: Option<LogFormat>,
    #[arg(long)]
    log_path: Option<String>,
    #[arg(long, default_value_t = false)]
    no_open: bool,
    #[arg(long, short = 'j')]
    json_path: Option<String>,
    #[arg(long)]
    csv_path: Option<String>,
    #[arg(long)]
    ndjson_path: Option<String>,
    #[arg(long, default_value_t = false)]
    emit_eventdata: bool,
    #[arg(long, default_value_t = false)]
    emit_xml: bool,
    #[arg(long)]
    md_path: Option<String>,
    #[arg(long)]
    md_fix_path: Option<String>,
    #[arg(long)]
    tsv_path: Option<String>,
    #[arg(long, short = 'p', num_args = 0.., value_delimiter = ',')]
    providers: Vec<String>,
    #[arg(long, short = 'x', num_args = 0.., value_delimiter = ',')]
    exclude_providers: Vec<String>,
    #[arg(long, short = 'E', default_value_t = 5000)]
    max_events: usize,
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=4))]
    min_level: Option<u8>,
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=4))]
    max_level: Option<u8>,
    #[arg(long, default_value_t = false)]
    only_critical: bool,
    #[arg(long, default_value_t = false)]
    only_errors: bool,
    #[arg(long, default_value_t = false)]
    only_warnings: bool,
    #[arg(short = 'v', long, action = ArgAction::Count)]
    verbose: u8,
    #[arg(short = 'q', long, default_value_t = false)]
    quiet: bool,
    #[arg(long, default_value_t = false)]
    progress: bool,
    #[arg(long, default_value_t = false)]
    warnings_as_errors: bool,
    #[arg(long, value_enum)]
    completions: Option<Shell>,
    #[arg(long)]
    completions_out: Option<String>,
    #[arg(long)]
    config: Option<String>,
    #[arg(long, default_value_t = false)]
    only_matched: bool,
    #[arg(long)]
    msg_width: Option<usize>,
    #[arg(long)]
    cause_width: Option<usize>,
    #[arg(long, default_value_t = false)]
    no_header: bool,
    
    #[arg(long, default_value_t = false)]
    summary_only: bool,
    #[arg(long, default_value_t = false)]
    analysis_only: bool,
    #[arg(long)]
    sample_count: Option<usize>,
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    include_event_ids: Vec<u32>,
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    exclude_event_ids: Vec<u32>,
    #[arg(long, default_value_t = false)]
    force_color: bool,
    #[arg(long, value_enum, default_value = "local")]
    time_zone: TimeZone,
    #[arg(long, value_enum, default_value = "time")]
    sort_by: SortBy,
    #[arg(long, value_enum, default_value = "desc")]
    sort_order: SortOrder,
    #[arg(long, num_args = 0.., value_delimiter = ',')]
    columns: Vec<Column>,
    #[arg(long, default_value_t = false)]
    no_truncate: bool,
    #[arg(long)]
    time_format: Option<String>,
    #[arg(long)]
    per_channel_sample_limit: Option<usize>,
    #[arg(long)]
    per_provider_sample_limit: Option<usize>,
    #[arg(long, default_value_t = false)]
    collect_perf: bool,
    #[arg(long, default_value_t = false)]
    smart_check: bool,
    #[arg(long, num_args = 2, value_delimiter = ',', help = "Two NDJSON paths: base,current")]
    compare_ndjson: Option<Vec<String>>,
    #[arg(long, help = "Write comparison summary to JSON path")]
    compare_out: Option<String>,
    #[arg(long, help = "Export a bundled set of outputs to this directory")]
    export_dir: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            minutes: 0,
            hours: 0,
            channels: vec![],
            patterns: vec![],
            top: 20,
            output: OutputFmt::Text,
            text_format: TextFormat::Lines,
            theme: Theme::Dark,
            live: false,
            subscribe_minutes: 0,
            last10m: false,
            last_day: false,
            last_hour: false,
            last_week: false,
            include_info: false,
            no_level_filter: false,
            html: None,
            scan_path: None,
            file_glob: None,
            max_file_samples: 20,
            evtx_path: None,
            evtx_glob: None,
            evtx_recursive: false,
            since: None,
            until: None,
            last_errors: 50,
            last_criticals: 50,
            rules: None,
            no_color: false,
            no_emoji: false,
            log_level: None,
            log_format: None,
            log_path: None,
            no_open: false,
            json_path: None,
            csv_path: None,
            ndjson_path: None,
            emit_eventdata: false,
            emit_xml: false,
        md_path: None,
        md_fix_path: None,
        tsv_path: None,
        providers: vec![],
        exclude_providers: vec![],
            max_events: 5000,
            min_level: None,
            max_level: None,
            only_critical: false,
            only_errors: false,
            only_warnings: false,
            verbose: 0,
            quiet: false,
            progress: false,
            warnings_as_errors: false,
            completions: None,
            completions_out: None,
            config: None,
            only_matched: false,
            msg_width: None,
            cause_width: None,
            no_header: false,
            summary_only: false,
            analysis_only: false,
            sample_count: None,
            include_event_ids: vec![],
            exclude_event_ids: vec![],
            force_color: false,
            time_zone: TimeZone::Local,
            sort_by: SortBy::Time,
            sort_order: SortOrder::Desc,
            columns: vec![],
            no_truncate: false,
            time_format: None,
            per_channel_sample_limit: None,
            per_provider_sample_limit: None,
            collect_perf: false,
            smart_check: false,
            compare_ndjson: None,
            compare_out: None,
            export_dir: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EventItem {
    time: DateTime<Utc>,
    level: u8,
    channel: String,
    provider: String,
    event_id: u32,
    content: String,
    raw_xml: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ReportSummary {
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
    total: usize,
    errors: usize,
    warnings: usize,
    by_provider: Vec<(String, usize)>,
    by_channel: Vec<(String, usize)>,
    by_event_id: Vec<(u32, usize)>,
    by_device: Vec<(String, usize)>,
    by_domain: Vec<(String, usize)>,
    matched_terms: Vec<(String, usize)>,
    samples: Vec<EventItem>,
    file_matched_terms: Vec<(String, usize)>,
    file_samples: Vec<crate::file_scan::FileSample>,
    scanned_records: usize,
    parsed_events: usize,
    novice_hints: Vec<crate::hints::NoviceHint>,
    mode: Option<String>,
    performance_score: u8,
    degradation_signals: Vec<(String, u8)>,
    recommendations: Vec<String>,
    likely_causes: Vec<String>,
    timeline: Vec<(String, usize, usize)>,
    by_category: Vec<(String, usize)>,
    perf_metrics: Vec<(String, u32, u32, usize)>,
    perf_counters: Option<crate::perf::PerfCounters>,
    smart_failure_predicted: Option<bool>,
    risk_grade: String,
}

#[derive(Deserialize)]
struct AppConfig {
    channels: Option<Vec<String>>,
    patterns: Option<Vec<String>>,
    providers: Option<Vec<String>>,
    exclude_providers: Option<Vec<String>>,
    output: Option<OutputFmt>,
    text_format: Option<TextFormat>,
    theme: Option<Theme>,
    max_events: Option<usize>,
    include_info: Option<bool>,
    no_level_filter: Option<bool>,
    min_level: Option<u8>,
    max_level: Option<u8>,
    scan_path: Option<String>,
    file_glob: Option<String>,
    evtx_path: Option<String>,
    evtx_glob: Option<String>,
    html: Option<String>,
    json_path: Option<String>,
    csv_path: Option<String>,
    ndjson_path: Option<String>,
    md_path: Option<String>,
    md_fix_path: Option<String>,
    warnings_as_errors: Option<bool>,
    progress: Option<bool>,
    last_errors: Option<usize>,
    last_criticals: Option<usize>,
    hours: Option<i64>,
    minutes: Option<i64>,
    since: Option<String>,
    until: Option<String>,
    summary_only: Option<bool>,
    analysis_only: Option<bool>,
    sample_count: Option<usize>,
    include_event_ids: Option<Vec<u32>>,
    exclude_event_ids: Option<Vec<u32>>,
    emit_eventdata: Option<bool>,
    emit_xml: Option<bool>,
    force_color: Option<bool>,
    time_zone: Option<TimeZone>,
    columns: Option<Vec<Column>>,
    no_truncate: Option<bool>,
    time_format: Option<String>,
    log_format: Option<LogFormat>,
    log_path: Option<String>,
    export_dir: Option<String>,
}
 

fn main() {
    let mut args = Args::parse();
    if let Some(sh) = args.completions {
        let mut cmd = Args::command();
        if let Some(path) = args.completions_out.as_ref() {
            if let Ok(mut f) = std::fs::File::create(path) { clap_complete::generate(sh, &mut cmd, "WinDoctor", &mut f); } else { clap_complete::generate(sh, &mut cmd, "WinDoctor", &mut std::io::stdout()); }
        } else {
            clap_complete::generate(sh, &mut cmd, "WinDoctor", &mut std::io::stdout());
        }
        return;
    }
    if let Some(p) = args.config.as_ref()
        && let Ok(s) = std::fs::read_to_string(p)
        && let Ok(cfg) = toml::from_str::<AppConfig>(&s) { apply_config(&mut args, cfg); }
    else {
        let def = "WinDoctor.toml";
        if let Ok(s) = std::fs::read_to_string(def)
            && let Ok(cfg) = toml::from_str::<AppConfig>(&s) { apply_config(&mut args, cfg); }
    }
    {
        let mut builder = env_logger::Builder::from_env(env_logger::Env::default());
        if args.quiet {
            builder.filter_level(log::LevelFilter::Error);
        } else if let Some(lvl) = args.log_level {
            let f = match lvl { LogLevel::Error => log::LevelFilter::Error, LogLevel::Warn => log::LevelFilter::Warn, LogLevel::Info => log::LevelFilter::Info, LogLevel::Debug => log::LevelFilter::Debug, LogLevel::Trace => log::LevelFilter::Trace };
            builder.filter_level(f);
        } else if args.verbose > 0 {
            let f = if args.verbose >= 3 { log::LevelFilter::Trace } else if args.verbose == 2 { log::LevelFilter::Debug } else { log::LevelFilter::Info };
            builder.filter_level(f);
        }
        if let Some(fmt) = args.log_format {
            match fmt {
                LogFormat::Json => {
                    builder.format(|buf, record| {
                        use std::io::Write;
                        let ts = chrono::Local::now().to_rfc3339();
                        let obj = serde_json::json!({
                            "ts": ts,
                            "level": record.level().to_string(),
                            "target": record.target(),
                            "msg": record.args().to_string(),
                        });
                        writeln!(buf, "{}", obj)
                    });
                }
                LogFormat::Text => {
                    builder.format(|buf, record| {
                        use std::io::Write;
                        let ts = chrono::Local::now().format("%H:%M:%S");
                        writeln!(buf, "[{:<5} {}] {}", record.level(), ts, record.args())
                    });
                }
            }
        }
        if let Some(path) = args.log_path.as_ref() {
            match std::fs::File::create(path) {
                Ok(f) => {
                    builder.target(env_logger::Target::Pipe(Box::new(f)));
                }
                Err(e) => {
                    eprintln!("Failed to open log file {}: {}", path, e);
                }
            }
        }
        builder.init();
    }
    let term = std::env::var("TERM").unwrap_or_default();
    let no_color_env = std::env::var_os("NO_COLOR").is_some();
    let color_default = std::io::stdout().is_terminal() && !no_color_env && term != "dumb";
    let enable_color = if args.force_color { true } else { color_default && !args.no_color };
    let _ = ENABLE_COLOR.set(enable_color);
    let since = compute_since(&args);
    let until = compute_until(&args);
    let channels = if args.channels.is_empty() {
        vec!["System".to_string(), "Application".to_string()]
    } else {
        args.channels.clone()
    };
    let rules_cfg = crate::rules::load_rules(args.rules.as_deref());
    let patterns = if args.patterns.is_empty() {
        if let Some(cfg) = rules_cfg.as_ref() {
            if let Some(p) = cfg.event_patterns.as_ref() { p.clone() } else {
                vec![
                    "(?i)error",
                    "(?i)fail",
                    "(?i)exception",
                    "(?i)timeout",
                    "(?i)bugcheck",
                    "(?i)crash",
                    "(?i)access denied",
                    "(?i)disk",
                    "(?i)io error",
                    "(?i)network",
                    "(?i)service",
                    "(?i)reset",
                    "(?i)retry",
                    "(?i)corrupt",
                    "(?i)degraded",
                    "(?i)unexpected",
                    "(?i)dcom",
                    "(?i)dns",
                    "(?i)w32time",
                    "(?i)group policy",
                    "(?i)usb",
                    "(?i)cdrom",
                    "(?i)netlogon",
                ]
                .into_iter()
                .map(|s| s.to_string())
                .collect()
            }
        } else {
            vec![
                "(?i)error",
                "(?i)fail",
                "(?i)exception",
                "(?i)timeout",
                "(?i)bugcheck",
                "(?i)crash",
                "(?i)access denied",
                "(?i)disk",
                "(?i)io error",
                "(?i)network",
                "(?i)service",
                "(?i)reset",
                "(?i)retry",
                "(?i)corrupt",
                "(?i)degraded",
                "(?i)unexpected",
                "(?i)dcom",
                "(?i)dns",
                "(?i)w32time",
                "(?i)group policy",
                "(?i)usb",
                "(?i)cdrom",
                "(?i)netlogon",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
        }
    } else {
        args.patterns.clone()
    };
    let compiled_patterns: Vec<Regex> = if args.only_matched { patterns.iter().filter_map(|p| Regex::new(p).ok()).collect() } else { Vec::new() };
    let mut events: Vec<EventItem> = vec![];
    let mut scanned_records: usize = 0;
    let mut parsed_events: usize = 0;
    if args.live {
        let live_events = crate::windows_live::query_live_events(&channels, since);
        scanned_records += live_events.len();
        parsed_events += live_events.len();
        events = live_events;
        if args.subscribe_minutes > 0 {
            let more = crate::windows_live::subscribe_events(&channels, args.subscribe_minutes * 60);
            scanned_records += more.len();
            parsed_events += more.len();
            events.extend(more);
        }
        events.retain(|e| e.time >= since && e.time <= until && pass_level(&args, e.level) && pass_provider(&args, &e.provider) && (!args.only_matched || compiled_patterns.iter().any(|re| re.is_match(&e.content))));
    } else if let Some(evtx) = args.evtx_path.as_ref() {
        let p = PathBuf::from(evtx);
        if !p.exists() { log::warn!("Missing EVTX: {}", p.to_string_lossy()); }
        if p.is_file() {
            if let Ok(mut parser) = EvtxParser::from_path(&p) {
                let ch = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let pb = if args.progress { Some(indicatif::ProgressBar::new_spinner()) } else { None };
                if let Some(ref pb) = pb { pb.set_message(format!("Scanning {}", ch)); }
                for r in parser.records() {
                    scanned_records += 1;
                    if let Some(ref pb) = pb { if scanned_records % 500 == 0 { pb.tick(); pb.set_message(format!("Scanned {} records", scanned_records)); } }
                    if r.is_err() { continue; }
                    let r = r.unwrap();
                    let xml = r.data;
                    if let Some(mut item) = parse_event_xml(&xml, &ch) {
                        parsed_events += 1;
                        if let Some(msg) = crate::decoder::decode_event(&item.provider, item.event_id, &xml) { item.content = msg; }
                        if args.emit_xml || args.emit_eventdata { item.raw_xml = Some(xml.clone()); }
                        if item.time >= since && item.time <= until && pass_level(&args, item.level) && pass_provider(&args, &item.provider) && pass_event_id(&args, item.event_id) && (!args.only_matched || compiled_patterns.iter().any(|re| re.is_match(&item.content))) { events.push(item); }
                    }
                    if events.len() >= args.max_events { break; }
                }
                if let Some(pb) = pb { pb.finish_and_clear(); }
            } else { log::error!("EVTX open failed: {}. Reading .evtx may require Administrator privileges.", p.to_string_lossy()); }
        } else if p.is_dir() {
            let mut set_opt = None;
            if let Some(g) = args.evtx_glob.as_ref() {
                let mut gb = globset::GlobSetBuilder::new();
                let glob = globset::GlobBuilder::new(g).case_insensitive(true).build().unwrap();
                gb.add(glob);
                set_opt = Some(gb.build().unwrap());
            }
            let wd = if args.evtx_recursive { walkdir::WalkDir::new(&p) } else { walkdir::WalkDir::new(&p).max_depth(1) };
            for de in wd.into_iter().filter_map(Result::ok) {
                let fp = de.path();
                if !fp.is_file() { continue; }
                if let Some(set) = &set_opt { if !set.is_match(fp) { continue; } }
                if fp.extension().and_then(|e| e.to_str()).map(|s| s.eq_ignore_ascii_case("evtx")).unwrap_or(false) {
                    let mut parser = match EvtxParser::from_path(fp) { Ok(x) => x, Err(e) => { log::error!("EVTX open failed for {}: {}", fp.to_string_lossy(), e); continue } };
                    let ch = fp.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    let pb = if args.progress { Some(indicatif::ProgressBar::new_spinner()) } else { None };
                    if let Some(ref pb) = pb { pb.set_message(format!("Scanning {}", ch)); }
                    for r in parser.records() {
                        scanned_records += 1;
                        if let Some(ref pb) = pb { if scanned_records % 500 == 0 { pb.tick(); pb.set_message(format!("Scanned {} records", scanned_records)); } }
                        if r.is_err() { continue; }
                        let r = r.unwrap();
                        let xml = r.data;
                        if let Some(mut item) = parse_event_xml(&xml, &ch) {
                            parsed_events += 1;
                            if let Some(msg) = crate::decoder::decode_event(&item.provider, item.event_id, &xml) { item.content = msg; }
                            if args.emit_xml || args.emit_eventdata { item.raw_xml = Some(xml.clone()); }
                            if item.time >= since && pass_level(&args, item.level) && pass_provider(&args, &item.provider) && pass_event_id(&args, item.event_id) && (!args.only_matched || compiled_patterns.iter().any(|re| re.is_match(&item.content))) { events.push(item); }
                        }
                        if events.len() >= args.max_events { break; }
                    }
                    if let Some(pb) = pb { pb.finish_and_clear(); }
                }
            }
        } else {
            log::warn!("EVTX path is neither file nor directory: {}", p.to_string_lossy());
        }
    } else {
        let mut live_events = crate::windows_live::query_live_events(&channels, since);
        scanned_records += live_events.len();
        parsed_events += live_events.len();
        live_events.retain(|e| e.time >= since && e.time <= until && pass_level(&args, e.level) && pass_provider(&args, &e.provider) && pass_event_id(&args, e.event_id));
        if !live_events.is_empty() {
            events = live_events;
        } else {
            for ch in channels.clone() {
                let path = PathBuf::from(r"C:\Windows\System32\winevt\Logs").join(format!("{}.evtx", ch));
                if !path.exists() { log::warn!("Missing EVTX: {}", path.to_string_lossy()); continue; }
                let mut parser = match EvtxParser::from_path(&path) { Ok(p) => p, Err(e) => { log::error!("EVTX open failed for {}: {}. Reading .evtx may require Administrator privileges.", ch, e); continue } };
                let pb = if args.progress { Some(indicatif::ProgressBar::new_spinner()) } else { None };
                if let Some(ref pb) = pb { pb.set_message(format!("Scanning {}", ch)); }
                for r in parser.records() {
                    scanned_records += 1;
                    if let Some(ref pb) = pb { if scanned_records % 500 == 0 { pb.tick(); pb.set_message(format!("Scanned {} records", scanned_records)); } }
                    if r.is_err() { continue; }
                    let r = r.unwrap();
                    let xml = r.data;
                    if let Some(mut item) = parse_event_xml(&xml, &ch) {
                        parsed_events += 1;
                        if let Some(msg) = crate::decoder::decode_event(&item.provider, item.event_id, &xml) { item.content = msg; }
                        if args.emit_xml || args.emit_eventdata { item.raw_xml = Some(xml.clone()); }
                        if item.time >= since && item.time <= until && pass_level(&args, item.level) && pass_provider(&args, &item.provider) && pass_event_id(&args, item.event_id) && (!args.only_matched || compiled_patterns.iter().any(|re| re.is_match(&item.content))) { events.push(item); }
                    }
                    if events.len() >= args.max_events { break; }
                }
                if let Some(pb) = pb { pb.finish_and_clear(); }
            }
        }
    }
    if events.len() > args.max_events { events.sort_by(|a, b| b.time.cmp(&a.time)); events.truncate(args.max_events); }
    let mut file_terms: Vec<(String, usize)> = vec![];
    let mut file_samples: Vec<crate::file_scan::FileSample> = vec![];
    if let Some(root) = args.scan_path.as_ref() {
        let file_patterns = if let Some(cfg) = rules_cfg.as_ref() { cfg.file_patterns.clone().unwrap_or_else(|| patterns.clone()) } else { patterns.clone() };
        let fs = crate::file_scan::scan(root, args.file_glob.as_deref(), &file_patterns, args.max_file_samples);
        file_terms = fs.by_term;
        file_samples = fs.samples;
    }
    {
        let any_time_flag = args.last10m || args.last_hour || args.last_day || args.last_week || args.hours > 0 || args.minutes > 0;
        if !any_time_flag {
            let mut crit: Vec<EventItem> = events.iter().filter(|e| e.level == 1).cloned().collect();
            crit.sort_by(|a, b| b.time.cmp(&a.time));
            crit.truncate(args.last_criticals);
            let mut err: Vec<EventItem> = events.iter().filter(|e| e.level == 2).cloned().collect();
            err.sort_by(|a, b| b.time.cmp(&a.time));
            err.truncate(args.last_errors);
            let mut combined = crit;
            combined.extend(err);
            combined.sort_by(|a, b| b.time.cmp(&a.time));
            events = combined;
        }
    }
    let any_time_flag = args.last10m || args.last_hour || args.last_day || args.last_week || args.hours > 0 || args.minutes > 0;
    let mode = if !any_time_flag { Some(format!("Last {} critical + last {} errors", args.last_criticals, args.last_errors)) } else { None };
    let sample_n = args.sample_count.unwrap_or(args.top);
    let perf_counters = if args.collect_perf { Some(crate::perf::collect_perf_counters()) } else { None };
    let smart_pred = if args.smart_check { crate::perf::smart_predict_failure() } else { None };
    let summary = build_summary_with_files(events, patterns, args.top, sample_n, args.sort_by, args.sort_order, since, until, file_terms, file_samples, scanned_records, parsed_events, mode, rules_cfg, perf_counters, smart_pred, args.per_channel_sample_limit, args.per_provider_sample_limit);
    if let Some(path) = args.html.as_ref() {
        let html = crate::html::render_html(&summary, args.theme, !args.no_emoji, args.time_zone, args.time_format.as_deref());
        match std::fs::write(path, html) {
            Ok(_) => {
                if !args.no_open { open_file_default(PathBuf::from(path)); }
                if !args.quiet { println!("{}", paint(&format!("HTML generated: {}", path), "1;36")); }
            }
            Err(e) => { log::error!("HTML write failed for {}: {}", path, e); }
        }
    } else if summary.mode.is_some() {
        let def = PathBuf::from("report.html");
        let html = crate::html::render_html(&summary, args.theme, !args.no_emoji, args.time_zone, args.time_format.as_deref());
        match std::fs::write(&def, html) {
            Ok(_) => {
                let s = def.to_string_lossy().into_owned();
                if !args.no_open { open_file_default(def.clone()); }
                if !args.quiet { println!("{}", paint(&format!("HTML generated: {}", s), "1;36")); }
            }
            Err(e) => { log::error!("HTML write failed for {}: {}", def.to_string_lossy(), e); }
        }
    }
    match args.output {
        OutputFmt::Text => {
            let widths = PrintWidths { msg: args.msg_width.unwrap_or(96), cause: args.cause_width.unwrap_or(24) };
            let cols = if args.columns.is_empty() { vec![Column::Time, Column::Severity, Column::Channel, Column::Provider, Column::Cause, Column::Message] } else { args.columns.clone() };
            match args.text_format {
                TextFormat::Lines => print_text(&summary, widths, args.no_header, args.summary_only, args.analysis_only, args.time_zone, &cols, args.no_truncate, args.time_format.as_deref(), !args.no_emoji),
                TextFormat::Table => print_text_table(&summary, widths, args.no_header, args.summary_only, args.analysis_only, args.time_zone, &cols, args.no_truncate, args.time_format.as_deref(), !args.no_emoji),
            }
        },
        OutputFmt::Json => {
            if let Some(p) = args.json_path.as_ref() {
                match std::fs::write(p, serde_json::to_vec_pretty(&summary).unwrap()) {
                    Ok(_) => { if !args.quiet { println!("{}", paint(&format!("JSON written: {}", p), "1;36")); } },
                    Err(e) => log::error!("JSON write failed for {}: {}", p, e),
                }
            } else if !args.quiet { println!("{}", serde_json::to_string_pretty(&summary).unwrap()); }
        }
    }
    if let Some(p) = args.csv_path.as_ref() {
        if let Err(e) = write_csv(p, &summary, args.time_zone, args.time_format.as_deref()) { log::error!("CSV write failed for {}: {}", p, e); } else if !args.quiet { println!("{}", paint(&format!("CSV written: {}", p), "1;36")); }
    }
    if let Some(p) = args.ndjson_path.as_ref() {
        if let Err(e) = write_ndjson(p, &summary, args.time_zone, args.time_format.as_deref(), args.emit_eventdata, args.emit_xml) { log::error!("NDJSON write failed for {}: {}", p, e); } else if !args.quiet { println!("{}", paint(&format!("NDJSON written: {}", p), "1;36")); }
    }
    if let Some(p) = args.md_path.as_ref() {
        let md = crate::markdown::render_markdown(&summary, args.time_zone, args.time_format.as_deref());
        match std::fs::write(p, md) {
            Ok(_) => { if !args.quiet { println!("{}", paint(&format!("Markdown written: {}", p), "1;36")); } }
            Err(e) => { log::error!("Markdown write failed for {}: {}", p, e); }
        }
    }
    if let Some(p) = args.tsv_path.as_ref() {
        if let Err(e) = write_tsv(p, &summary, args.time_zone, args.time_format.as_deref()) { log::error!("TSV write failed for {}: {}", p, e); } else if !args.quiet { println!("{}", paint(&format!("TSV written: {}", p), "1;36")); }
    }
    
    if let Some(p) = args.md_fix_path.as_ref() {
        let md = crate::markdown::render_fix_markdown(&summary, args.time_zone, args.time_format.as_deref());
        match std::fs::write(p, md.as_bytes()) {
            Ok(_) => { if !args.quiet { println!("{}", paint(&format!("Fix-It Markdown written: {}", p), "1;36")); } }
            Err(e) => log::error!("Fix-It Markdown write failed for {}: {}", p, e),
        }
    }
    if let Some(dir) = args.export_dir.as_ref() {
        let _ = std::fs::create_dir_all(dir);
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let base = std::path::PathBuf::from(dir);
        let html_path = base.join(format!("report-{}.html", ts));
        let html = crate::html::render_html(&summary, args.theme, !args.no_emoji, args.time_zone, args.time_format.as_deref());
        match std::fs::write(&html_path, html) {
            Ok(_) => { if !args.no_open { open_file_default(html_path.clone()); } if !args.quiet { println!("{}", paint(&format!("HTML generated: {}", html_path.to_string_lossy()), "1;36")); } }
            Err(e) => { log::error!("HTML write failed for {}: {}", html_path.to_string_lossy(), e); }
        }
        let json_path = base.join(format!("report-{}.json", ts));
        match std::fs::write(&json_path, serde_json::to_vec_pretty(&summary).unwrap()) {
            Ok(_) => { if !args.quiet { println!("{}", paint(&format!("JSON written: {}", json_path.to_string_lossy()), "1;36")); } }
            Err(e) => log::error!("JSON write failed for {}: {}", json_path.to_string_lossy(), e),
        }
        let ndjson_path = base.join(format!("events-{}.ndjson", ts));
        if let Err(e) = write_ndjson(&ndjson_path.to_string_lossy(), &summary, args.time_zone, args.time_format.as_deref(), args.emit_eventdata, args.emit_xml) {
            log::error!("NDJSON write failed for {}: {}", ndjson_path.to_string_lossy(), e);
        } else if !args.quiet { println!("{}", paint(&format!("NDJSON written: {}", ndjson_path.to_string_lossy()), "1;36")); }
        let csv_path = base.join(format!("events-{}.csv", ts));
        if let Err(e) = write_csv(&csv_path.to_string_lossy(), &summary, args.time_zone, args.time_format.as_deref()) { log::error!("CSV write failed for {}: {}", csv_path.to_string_lossy(), e); } else if !args.quiet { println!("{}", paint(&format!("CSV written: {}", csv_path.to_string_lossy()), "1;36")); }
        let tsv_path = base.join(format!("events-{}.tsv", ts));
        if let Err(e) = write_tsv(&tsv_path.to_string_lossy(), &summary, args.time_zone, args.time_format.as_deref()) { log::error!("TSV write failed for {}: {}", tsv_path.to_string_lossy(), e); } else if !args.quiet { println!("{}", paint(&format!("TSV written: {}", tsv_path.to_string_lossy()), "1;36")); }
        let fix_md_path = base.join(format!("fix-{}.md", ts));
        let fix_md = crate::markdown::render_fix_markdown(&summary, args.time_zone, args.time_format.as_deref());
        match std::fs::write(&fix_md_path, fix_md.as_bytes()) {
            Ok(_) => { if !args.quiet { println!("{}", paint(&format!("Fix-It Markdown written: {}", fix_md_path.to_string_lossy()), "1;36")); } }
            Err(e) => log::error!("Fix-It Markdown write failed for {}: {}", fix_md_path.to_string_lossy(), e),
        }
    }
    if let Some(paths) = args.compare_ndjson.as_ref()
        && paths.len() == 2
        && let Some(cmp) = compare_ndjson(&paths[0], &paths[1]) {
        print_comparison(&cmp);
        if let Some(p) = args.compare_out.as_ref() { let _ = write_compare_json(p, &cmp); }
    }
    if args.warnings_as_errors && (summary.errors > 0 || summary.warnings > 0) { std::process::exit(1); }
}

fn apply_config(args: &mut Args, cfg: AppConfig) {
    if args.channels.is_empty() && let Some(v) = cfg.channels { args.channels = v; }
    if args.patterns.is_empty() && let Some(v) = cfg.patterns { args.patterns = v; }
    if args.providers.is_empty() && let Some(v) = cfg.providers { args.providers = v; }
    if args.exclude_providers.is_empty() && let Some(v) = cfg.exclude_providers { args.exclude_providers = v; }
    if let Some(v) = cfg.output { args.output = v; }
    if let Some(v) = cfg.text_format { args.text_format = v; }
    if let Some(v) = cfg.theme { args.theme = v; }
    if args.max_events == 5000 && let Some(v) = cfg.max_events { args.max_events = v; }
    if let Some(v) = cfg.include_info { args.include_info = v; }
    if let Some(v) = cfg.no_level_filter { args.no_level_filter = v; }
    if args.min_level.is_none() && let Some(v) = cfg.min_level { args.min_level = Some(v); }
    if args.max_level.is_none() && let Some(v) = cfg.max_level { args.max_level = Some(v); }
    if args.scan_path.is_none() && let Some(v) = cfg.scan_path { args.scan_path = Some(v); }
    if args.file_glob.is_none() && let Some(v) = cfg.file_glob { args.file_glob = Some(v); }
    if args.evtx_path.is_none() && let Some(v) = cfg.evtx_path { args.evtx_path = Some(v); }
    if args.evtx_glob.is_none() && let Some(v) = cfg.evtx_glob { args.evtx_glob = Some(v); }
    if args.html.is_none() && let Some(v) = cfg.html { args.html = Some(v); }
    if args.json_path.is_none() && let Some(v) = cfg.json_path { args.json_path = Some(v); }
    if args.csv_path.is_none() && let Some(v) = cfg.csv_path { args.csv_path = Some(v); }
    if args.ndjson_path.is_none() && let Some(v) = cfg.ndjson_path { args.ndjson_path = Some(v); }
    if args.md_path.is_none() && let Some(v) = cfg.md_path { args.md_path = Some(v); }
    if args.md_fix_path.is_none() && let Some(v) = cfg.md_fix_path { args.md_fix_path = Some(v); }
    if let Some(v) = cfg.warnings_as_errors { args.warnings_as_errors = v; }
    if let Some(v) = cfg.progress { args.progress = v; }
    if let Some(v) = cfg.summary_only { args.summary_only = v; }
    if let Some(v) = cfg.analysis_only { args.analysis_only = v; }
    if args.sample_count.is_none() && let Some(v) = cfg.sample_count { args.sample_count = Some(v); }
    if args.include_event_ids.is_empty() && let Some(v) = cfg.include_event_ids { args.include_event_ids = v; }
    if args.exclude_event_ids.is_empty() && let Some(v) = cfg.exclude_event_ids { args.exclude_event_ids = v; }
    if let Some(v) = cfg.emit_eventdata { args.emit_eventdata = v; }
    if let Some(v) = cfg.emit_xml { args.emit_xml = v; }
    if let Some(v) = cfg.force_color { args.force_color = v; }
    if let Some(v) = cfg.time_zone { args.time_zone = v; }
    if args.columns.is_empty() && let Some(v) = cfg.columns { args.columns = v; }
    if let Some(v) = cfg.no_truncate { args.no_truncate = v; }
    if args.time_format.is_none() && let Some(v) = cfg.time_format { args.time_format = Some(v); }
    if let Some(v) = cfg.log_format { args.log_format = Some(v); }
    if args.log_path.is_none() && let Some(v) = cfg.log_path { args.log_path = Some(v); }
    if args.export_dir.is_none() && let Some(v) = cfg.export_dir { args.export_dir = Some(v); }
    let any_time_flag = args.last10m || args.last_hour || args.last_day || args.last_week || args.hours > 0 || args.minutes > 0 || args.since.is_some() || args.until.is_some();
    if !any_time_flag {
        if let Some(v) = cfg.last_errors { args.last_errors = v; }
        if let Some(v) = cfg.last_criticals { args.last_criticals = v; }
        if let Some(v) = cfg.hours { args.hours = v; }
        if let Some(v) = cfg.minutes { args.minutes = v; }
        if args.since.is_none() && let Some(v) = cfg.since { args.since = Some(v); }
        if args.until.is_none() && let Some(v) = cfg.until { args.until = Some(v); }
    }
}

fn compute_since(args: &Args) -> DateTime<Utc> {
    let now = Utc::now();
    if let Some(s) = args.since.as_ref() && let Some(dt) = parse_system_time(s) { return dt; }
    let any_time_flag = args.last10m || args.last_hour || args.last_day || args.last_week || args.hours > 0 || args.minutes > 0;
    if !any_time_flag && (args.last_errors > 0 || args.last_criticals > 0) { return now - Duration::days(36500); }
    if args.last10m { return now - Duration::minutes(10); }
    if args.last_hour { return now - Duration::hours(1); }
    if args.last_day { return now - Duration::hours(24); }
    if args.last_week { return now - Duration::days(7); }
    if args.hours > 0 { return now - Duration::hours(args.hours); }
    if args.minutes > 0 { return now - Duration::minutes(args.minutes); }
    now - Duration::hours(1)
}

fn compute_until(args: &Args) -> DateTime<Utc> {
    if let Some(s) = args.until.as_ref() && let Some(dt) = parse_system_time(s) { return dt; }
    Utc::now()
}

fn parse_event_xml(xml: &str, channel: &str) -> Option<EventItem> {
    if let Some(item) = parse_event_xml_qx(xml, channel) { return Some(item); }
    let t = extract_attr(xml, "TimeCreated", "SystemTime").and_then(|s| parse_system_time(&s))
        .or_else(|| extract_between(xml, "<TimeCreated SystemTime=\"", "\"").and_then(|s| parse_system_time(&s)));
    let time = t?;
    let level = extract_between(xml, "<Level>", "</Level>").and_then(|s| s.parse::<u8>().ok()).unwrap_or(0);
    let provider = extract_attr(xml, "Provider", "Name").unwrap_or_default();
    let event_id = extract_between(xml, "<EventID", "</EventID>").and_then(|s| {
        let s2 = if let Some(idx) = s.rfind('>') { &s[idx+1..] } else { &s };
        s2.trim().parse::<u32>().ok()
    }).unwrap_or(0);
    let content = extract_between(xml, "<EventData>", "</EventData>").unwrap_or_else(|| xml.to_string());
    let ch_xml = extract_between(xml, "<Channel>", "</Channel>").unwrap_or_else(|| channel.to_string());
    Some(EventItem { time, level, channel: ch_xml, provider, event_id, content, raw_xml: None })
}

fn parse_event_xml_qx(xml: &str, channel: &str) -> Option<EventItem> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut time_opt: Option<DateTime<Utc>> = None;
    let mut level_opt: Option<u8> = None;
    let mut provider = String::new();
    let mut event_id_opt: Option<u32> = None;
    let mut channel_s = String::new();
    let mut cur = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) => {
                cur = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if cur == "TimeCreated" {
                    for a in e.attributes().flatten() {
                        let k = String::from_utf8_lossy(a.key.as_ref());
                        if k == "SystemTime" {
                            let v = a.unescape_value().ok()?.to_string();
                            if let Some(dt) = parse_system_time(&v) { time_opt = Some(dt); }
                        }
                    }
                } else if cur == "Provider" {
                    for a in e.attributes().flatten() {
                        let k = String::from_utf8_lossy(a.key.as_ref());
                        if k == "Name" { provider = a.unescape_value().ok()?.to_string(); }
                    }
                }
            }
            Ok(XmlEvent::Text(t)) => {
                let v = String::from_utf8_lossy(t.as_ref()).into_owned();
                if cur == "Level" { if let Ok(n) = v.parse::<u8>() { level_opt = Some(n); } }
                else if cur == "EventID" { if let Ok(n) = v.trim().parse::<u32>() { event_id_opt = Some(n); } }
                else if cur == "Channel" { channel_s = v; }
            }
            Ok(XmlEvent::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
    let time = time_opt?;
    let level = level_opt.unwrap_or(0);
    let event_id = event_id_opt.unwrap_or(0);
    let content = extract_between(xml, "<EventData>", "</EventData>").unwrap_or_else(|| xml.to_string());
    let ch_xml = if channel_s.is_empty() { channel.to_string() } else { channel_s };
    Some(EventItem { time, level, channel: ch_xml, provider, event_id, content, raw_xml: None })
}

fn parse_system_time(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) { return Some(dt.with_timezone(&Utc)); }
    let mut alt = s.replace(' ', "T");
    if !alt.ends_with('Z') && !alt.contains('+') { alt.push('Z'); }
    if let Ok(dt) = DateTime::parse_from_rfc3339(&alt) { return Some(dt.with_timezone(&Utc)); }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") { return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)); }
    None
}

fn extract_between(hay: &str, start: &str, end: &str) -> Option<String> {
    let s = hay.find(start)?;
    let e = hay[s + start.len()..].find(end)? + s + start.len();
    Some(hay[s + start.len()..e].to_string())
}

fn extract_attr(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let open = format!("<{} ", tag);
    let s = xml.find(&open)?;
    let rest = &xml[s + open.len()..];
    let key = format!("{}=\"", attr);
    let ks = rest.find(&key)?;
    let after = &rest[ks + key.len()..];
    let ke = after.find('"')?;
    Some(after[..ke].to_string())
}

#[allow(clippy::too_many_arguments)]
fn build_summary_with_files(events: Vec<EventItem>, patterns: Vec<String>, top: usize, sample_count: usize, sort_by: SortBy, sort_order: SortOrder, since: DateTime<Utc>, until: DateTime<Utc>, file_terms: Vec<(String, usize)>, file_samples: Vec<crate::file_scan::FileSample>, scanned_records: usize, parsed_events: usize, mode: Option<String>, rules_cfg: Option<crate::rules::RulesConfig>, perf_counters: Option<crate::perf::PerfCounters>, smart_pred: Option<bool>, per_channel_sample_limit: Option<usize>, per_provider_sample_limit: Option<usize>) -> ReportSummary {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for e in &events {
        if e.level == 2 { errors += 1; } else if e.level == 3 { warnings += 1; }
    }
    let by_provider: Vec<(String, usize)> = {
        let mut pc: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for e in &events { *pc.entry(e.provider.clone()).or_insert(0) += 1; }
        let mut pv: Vec<(String, usize)> = pc.into_iter().collect();
        pv.sort_by(|a, b| b.1.cmp(&a.1));
        pv.into_iter().take(top).collect()
    };
    let by_channel: Vec<(String, usize)> = {
        let mut cc: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for e in &events { *cc.entry(e.channel.clone()).or_insert(0) += 1; }
        let mut cv: Vec<(String, usize)> = cc.into_iter().collect();
        cv.sort_by(|a, b| b.1.cmp(&a.1));
        cv.into_iter().take(top).collect()
    };
    let by_event_id: Vec<(u32, usize)> = {
        let mut ec: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for e in &events { *ec.entry(e.event_id).or_insert(0) += 1; }
        let mut ev: Vec<(u32, usize)> = ec.into_iter().collect();
        ev.sort_by(|a, b| b.1.cmp(&a.1));
        ev.into_iter().take(top).collect()
    };
    let by_device: Vec<(String, usize)> = {
        let mut dc: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for e in &events {
            let pairs = crate::event_xml::event_data_pairs_or_fallback(&e.content);
            for k in ["DeviceName", "TargetDevice", "Device", "InstancePath", "PhysicalDeviceObjectName"].iter() {
                if let Some(v) = pairs.get(*k) && !v.is_empty() { *dc.entry(v.clone()).or_insert(0) += 1; break; }
            }
        }
        let mut dv: Vec<(String, usize)> = dc.into_iter().collect();
        dv.sort_by(|a, b| b.1.cmp(&a.1));
        dv.into_iter().take(top).collect()
    };
    let by_domain: Vec<(String, usize)> = {
        let mut dm: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for e in &events {
            let d = classify_domain(&e.provider, &e.channel, e.event_id, &e.content);
            *dm.entry(d).or_insert(0) += 1;
        }
        let mut dv: Vec<(String, usize)> = dm.into_iter().collect();
        dv.sort_by(|a, b| b.1.cmp(&a.1));
        dv.into_iter().take(top).collect()
    };
    let matched_terms: Vec<(String, usize)> = {
        let mut tc: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for pat in patterns {
            if let Ok(re) = Regex::new(&pat) {
                let mut count = 0usize;
                for e in &events {
                    if re.is_match(&e.content) { count += 1; }
                }
                if count > 0 { tc.insert(pat, count); }
            }
        }
        let mut tv: Vec<(String, usize)> = tc.into_iter().collect();
        tv.sort_by(|a, b| b.1.cmp(&a.1));
        tv
    };
    let mut samples = events.clone();
    match (sort_by, sort_order) {
        (SortBy::Time, SortOrder::Desc) => samples.sort_by(|a, b| b.time.cmp(&a.time)),
        (SortBy::Time, SortOrder::Asc) => samples.sort_by(|a, b| a.time.cmp(&b.time)),
        (SortBy::Severity, SortOrder::Desc) => samples.sort_by(|a, b| b.level.cmp(&a.level)),
        (SortBy::Severity, SortOrder::Asc) => samples.sort_by(|a, b| a.level.cmp(&b.level)),
        (SortBy::Provider, SortOrder::Desc) => samples.sort_by(|a, b| b.provider.cmp(&a.provider)),
        (SortBy::Provider, SortOrder::Asc) => samples.sort_by(|a, b| a.provider.cmp(&b.provider)),
        (SortBy::Channel, SortOrder::Desc) => samples.sort_by(|a, b| b.channel.cmp(&a.channel)),
        (SortBy::Channel, SortOrder::Asc) => samples.sort_by(|a, b| a.channel.cmp(&b.channel)),
        (SortBy::EventId, SortOrder::Desc) => samples.sort_by(|a, b| b.event_id.cmp(&a.event_id)),
        (SortBy::EventId, SortOrder::Asc) => samples.sort_by(|a, b| a.event_id.cmp(&b.event_id)),
    }
    if per_channel_sample_limit.is_some() || per_provider_sample_limit.is_some() {
        let cl = per_channel_sample_limit.unwrap_or(usize::MAX);
        let pl = per_provider_sample_limit.unwrap_or(usize::MAX);
        let mut ch_cnt: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut pr_cnt: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut limited: Vec<EventItem> = Vec::new();
        for e in samples.iter() {
            let cc = *ch_cnt.get(&e.channel).unwrap_or(&0);
            let pc = *pr_cnt.get(&e.provider).unwrap_or(&0);
            if cc < cl && pc < pl {
                ch_cnt.insert(e.channel.clone(), cc + 1);
                pr_cnt.insert(e.provider.clone(), pc + 1);
                limited.push(e.clone());
            }
        }
        samples = limited;
    }
    match (sort_by, sort_order) {
        (SortBy::Time, SortOrder::Desc) => samples.sort_by(|a, b| b.time.cmp(&a.time)),
        (SortBy::Time, SortOrder::Asc) => samples.sort_by(|a, b| a.time.cmp(&b.time)),
        (SortBy::Severity, SortOrder::Desc) => samples.sort_by(|a, b| b.level.cmp(&a.level)),
        (SortBy::Severity, SortOrder::Asc) => samples.sort_by(|a, b| a.level.cmp(&b.level)),
        (SortBy::Provider, SortOrder::Desc) => samples.sort_by(|a, b| b.provider.cmp(&a.provider)),
        (SortBy::Provider, SortOrder::Asc) => samples.sort_by(|a, b| a.provider.cmp(&b.provider)),
        (SortBy::Channel, SortOrder::Desc) => samples.sort_by(|a, b| b.channel.cmp(&a.channel)),
        (SortBy::Channel, SortOrder::Asc) => samples.sort_by(|a, b| a.channel.cmp(&b.channel)),
        (SortBy::EventId, SortOrder::Desc) => samples.sort_by(|a, b| b.event_id.cmp(&a.event_id)),
        (SortBy::EventId, SortOrder::Asc) => samples.sort_by(|a, b| a.event_id.cmp(&b.event_id)),
    }
    samples.truncate(sample_count);
    {
        use std::collections::HashMap;
        let mut deduped: Vec<EventItem> = Vec::new();
        let mut seen: HashMap<(String, String), usize> = HashMap::new();
        let max_dups = 3usize;
        for e in samples.iter() {
            if e.provider == "Application Error" {
                let key = (event_cause(e), event_message(e));
                let c = *seen.get(&key).unwrap_or(&0);
                if c < max_dups {
                    seen.insert(key, c + 1);
                    deduped.push(e.clone());
                }
            } else {
                deduped.push(e.clone());
            }
        }
        samples = deduped;
    }
    
    let mut novice_hints = crate::hints::generate_hints(&events);
    if let Some(cfg) = rules_cfg.as_ref() {
        let extra = crate::rules::apply_hint_rules(&events, cfg);
        if !extra.is_empty() { novice_hints.extend(extra); }
    }
    let (perf_score, perf_signals) = perf::compute_performance_metrics(&events);
    let perf_metrics = perf::compute_perf_details(&events);
    let recs = perf::generate_recommendations(&novice_hints);
    let causes = perf::compute_root_causes(&novice_hints);
    let timeline = perf::compute_timeline(&events, since, until);
    let by_category = perf::compute_by_category(&novice_hints);
    let risk_grade = {
        let mut grade = if perf_score >= 80 { "Critical" } else if perf_score >= 60 { "High" } else if perf_score >= 40 { "Medium" } else { "Low" };
        if novice_hints.iter().any(|h| h.category == "Storage" && h.severity == "high") && perf_score >= 40 { grade = "High"; }
        grade.to_string()
    };
    ReportSummary {
        window_start: since,
        window_end: until,
        total: events.len(),
        errors,
        warnings,
        by_provider,
        by_channel,
        by_event_id,
        by_device,
        by_domain,
        matched_terms,
        samples,
        file_matched_terms: file_terms,
        file_samples,
        scanned_records,
        parsed_events,
        novice_hints,
        mode,
        performance_score: perf_score,
        degradation_signals: perf_signals,
        recommendations: recs,
        likely_causes: causes,
        timeline,
        by_category,
        perf_metrics,
        perf_counters,
        smart_failure_predicted: smart_pred,
        risk_grade,
    }
}

struct PrintWidths { msg: usize, cause: usize }

#[allow(clippy::too_many_arguments)]
fn print_text(rep: &ReportSummary, widths: PrintWidths, no_header: bool, summary_only: bool, analysis_only: bool, tz: TimeZone, cols: &Vec<Column>, no_trunc: bool, tfmt: Option<&str>, emoji: bool) {
    let start_local = rep.window_start.with_timezone(&Local);
    let end_local = rep.window_end.with_timezone(&Local);
    let start_s = format!("{}", start_local.format("%Y-%m-%d %H:%M"));
    let end_s = format!("{}", end_local.format("%Y-%m-%d %H:%M"));
    if !no_header { println!("{}", paint(&format!("Time Window: {} to {} (local time)", start_s, end_s), "1;36")); }
    if !no_header && let Some(m) = rep.mode.as_ref() { println!("{}", paint(&format!("Mode: {}", m), "1;36")); }
    if rep.errors == 0 && rep.warnings == 0 {
        if !no_header { println!("{}", paint("Status: No errors or warnings detected.", "1;32")); }
    } else if !no_header { println!("{}", paint(&format!("Status: {} errors and {} warnings detected.", rep.errors, rep.warnings), "1;33")); }
    if !no_header { println!("{} {}", paint("Risk:", "1"), rep.risk_grade); }
    if !rep.likely_causes.is_empty() {
        if !no_header { println!("{}", paint("Likely Root Causes:", "1")); }
        for c in &rep.likely_causes { println!("- {}", c); }
    }
    if !rep.by_category.is_empty() {
        if !no_header { println!("{}", paint("Impact Assessment:", "1")); }
        for (cat, cnt) in &rep.by_category { println!("• {} ({})", cat, cnt); }
    }
    if analysis_only || rep.mode.is_some() {
        if !no_header { println!("{}", paint("Diagnostics:", "1")); }
        if rep.novice_hints.is_empty() {
            if !no_header { println!("{}", paint("None", "2")); }
        } else {
            for h in &rep.novice_hints {
                let ev = if h.evidence.is_empty() { String::new() } else { format!(" | Examples: {}", h.evidence.join(", ")) };
                println!("[{} {}%] {} ({} occurrences){}", h.severity, h.probability, h.message, h.count, ev);
            }
        }
        println!("{} {}", paint("Performance Score:", "1"), rep.performance_score);
        if let Some(pc) = &rep.perf_counters {
            println!("{}", paint("Live Performance:", "1"));
            if let Some(v) = pc.cpu_percent { println!("• CPU: {}%", v); }
            if let Some(v) = pc.avg_disk_ms_per_transfer { println!("• Avg Disk Transfer: {:.2} ms", v); }
            if let Some(v) = pc.disk_reads_per_sec { println!("• Reads/s: {}", v); }
            if let Some(v) = pc.disk_writes_per_sec { println!("• Writes/s: {}", v); }
        }
        if let Some(pred) = rep.smart_failure_predicted && pred { println!("{}", paint("SMART: Predicts failure on one or more drives", "1;31")); }
        if !rep.degradation_signals.is_empty() { println!("{}", paint("Degradation Signals:", "1")); for (n,w) in &rep.degradation_signals { println!("• {} (weight {})", n, w); } }
        if !rep.recommendations.is_empty() { println!("{}", paint("Recommendations:", "1")); for r in &rep.recommendations { println!("- {}", r); } }
        if !rep.recommendations.is_empty() { println!("{}", paint("Checklist:", "1")); for r in &rep.recommendations { println!("[ ] {}", r); } }
        if !rep.timeline.is_empty() {
            println!("{}", paint("Timeline:", "1"));
            let max_e = rep.timeline.iter().map(|(_,e,_)| *e).max().unwrap_or(1);
            let max_w = rep.timeline.iter().map(|(_,_,w)| *w).max().unwrap_or(1);
            for (t,e,w) in &rep.timeline {
                let eb = bar(*e, max_e, 20);
                let wb = bar(*w, max_w, 20);
                println!("{}  E:{:<3} {}  W:{:<3} {}", t, e, eb, w, wb);
            }
        }
        if !rep.perf_metrics.is_empty() {
            println!("{}", paint("Performance Metrics:", "1"));
            for (name, avg, max, count) in &rep.perf_metrics {
                println!("{}: avg {} ms, max {} ms ({} samples)", name, avg, max, count);
            }
        }
        return;
    }
    if !no_header { println!("{} {}", paint("Events:", "1"), rep.total); }
    if !no_header { println!("{}", paint("Key Sources:", "1")); }
    if !no_header { if rep.by_provider.is_empty() { println!("{}", paint("None", "2")); } else { for (p, c) in &rep.by_provider { println!("• {} ({})", p, c); } } }
    if !no_header { println!("{}", paint("Key Domains:", "1")); }
    if !no_header { if rep.by_domain.is_empty() { println!("{}", paint("None", "2")); } else { for (d, c) in &rep.by_domain { println!("• {} ({})", d, c); } } }
    if !no_header { println!("{}", paint("Key Devices:", "1")); }
    if !no_header { if rep.by_device.is_empty() { println!("{}", paint("None", "2")); } else { for (d, c) in &rep.by_device { println!("• {} ({})", d, c); } } }
    if !no_header { println!("{}", paint("Common Event Codes:", "1")); }
    if !no_header { if rep.by_event_id.is_empty() { println!("{}", paint("None", "2")); } else { for (id, c) in &rep.by_event_id { println!("• {} ({})", id, c); } } }
    if !no_header { println!("{}", paint("Matched Keywords:", "1")); }
    if !no_header { if rep.matched_terms.is_empty() { println!("{}", paint("None", "2")); } else { for (t, c) in &rep.matched_terms { println!("• {} ({})", t, c); } } }
    if !no_header { println!("{}", paint("Recent Activity:", "1;36")); }
    if !no_header {
        let header = build_line(cols, "Time", "Severity", "Channel", "Provider", Some("EventId"), "Cause", "Message", 16, 10, 14, 18, 8, 24, 96);
        println!("{}", paint(&header, "1"));
    }
    if summary_only { return; }
    for e in &rep.samples {
        let ts = match (tz, tfmt) {
            (TimeZone::Local, Some(f)) => format!("{}", e.time.with_timezone(&Local).format(f)),
            (TimeZone::Utc, Some(f)) => format!("{}", e.time.format(f)),
            (TimeZone::Local, None) => format!("{}", e.time.with_timezone(&Local).format("%Y-%m-%d %H:%M")),
            (TimeZone::Utc, None) => format!("{}", e.time.format("%Y-%m-%d %H:%M")),
        };
        let sev = level_name(e.level);
        let sev_disp = if emoji { match sev { "Critical"=>"⛔ Critical", "Error"=>"⛔ Error", "Warning"=>"⚠️ Warning", "Information"=>"🛈 Information", _=>sev } } else { sev };
        let sev_s = paint(sev_disp, sev_code(e.level));
        let ch = if no_trunc { e.channel.clone() } else { truncate(&e.channel, 14) };
        let pr = if no_trunc { e.provider.clone() } else { truncate(&e.provider, 18) };
        let eid = e.event_id.to_string();
        let cause = if no_trunc { event_cause(e) } else { truncate(&event_cause(e), widths.cause) };
        let msg = if no_trunc { event_message(e) } else { truncate(&event_message(e), widths.msg) };
        let line = build_line(cols, &ts, &sev_s, &ch, &pr, Some(&eid), &cause, &msg, 16, 10, 14, 18, 8, 24, 96);
        println!("{}", line);
    }
    if !rep.file_samples.is_empty() || !rep.file_matched_terms.is_empty() {
        println!("{}", paint("Files:", "1;36"));
        if !rep.file_matched_terms.is_empty() {
            println!("{}", paint("Matched Keywords:", "1"));
            for (t, c) in &rep.file_matched_terms { println!("• {} ({} files)", t, c); }
        }
        if !rep.file_samples.is_empty() {
            println!("{}", paint("Examples:", "1"));
            for s in &rep.file_samples {
                let msg = truncate(&s.line.replace('\n', " "), 120);
                println!("{} [{}] line {}: {}", s.path, s.pattern, s.line_no, msg);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn print_text_table(rep: &ReportSummary, widths: PrintWidths, no_header: bool, summary_only: bool, analysis_only: bool, tz: TimeZone, cols: &Vec<Column>, no_trunc: bool, tfmt: Option<&str>, emoji: bool) {
    let start_local = rep.window_start.with_timezone(&Local);
    let end_local = rep.window_end.with_timezone(&Local);
    let start_s = format!("{}", start_local.format("%Y-%m-%d %H:%M"));
    let end_s = format!("{}", end_local.format("%Y-%m-%d %H:%M"));
    if !no_header { println!("{}", paint(&format!("Time Window: {} to {} (local time)", start_s, end_s), "1;36")); }
    if !no_header && let Some(m) = rep.mode.as_ref() { println!("{}", paint(&format!("Mode: {}", m), "1;36")); }
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    let mut hdr: Vec<String> = Vec::new();
    for c in cols {
        let h = match c { Column::Time => "Time", Column::Severity => "Severity", Column::Channel => "Channel", Column::Provider => "Provider", Column::EventId => "EventId", Column::Cause => "Cause", Column::Message => "Message" };
        hdr.push(paint(h, "1"));
    }
        table.set_header(hdr);
    if summary_only { println!("{}", table); return; }
    if analysis_only { println!("{}", paint("(Analysis-only mode — samples hidden)", "2")); return; }
    for e in &rep.samples {
        let ts = match (tz, tfmt) {
            (TimeZone::Local, Some(f)) => format!("{}", e.time.with_timezone(&Local).format(f)),
            (TimeZone::Utc, Some(f)) => format!("{}", e.time.format(f)),
            (TimeZone::Local, None) => format!("{}", e.time.with_timezone(&Local).format("%Y-%m-%d %H:%M")),
            (TimeZone::Utc, None) => format!("{}", e.time.format("%Y-%m-%d %H:%M")),
        };
        let sev = level_name(e.level);
        let sev_disp = if emoji { match sev { "Critical"=>"⛔ Critical", "Error"=>"⛔ Error", "Warning"=>"⚠️ Warning", "Information"=>"🛈 Information", _=>sev } } else { sev };
        let sev_s = paint(sev_disp, sev_code(e.level));
        let ch = if no_trunc { e.channel.clone() } else { truncate(&e.channel, 14) };
        let pr = if no_trunc { e.provider.clone() } else { truncate(&e.provider, 18) };
        let eid = e.event_id.to_string();
        let cause = if no_trunc { event_cause(e) } else { truncate(&event_cause(e), widths.cause) };
        let msg = if no_trunc { event_message(e) } else { truncate(&event_message(e), widths.msg) };
        let mut row: Vec<String> = Vec::new();
        for c in cols {
            match c {
                Column::Time => row.push(ts.clone()),
                Column::Severity => row.push(sev_s.to_string()),
                Column::Channel => row.push(ch.clone()),
                Column::Provider => row.push(pr.clone()),
                Column::EventId => row.push(eid.clone()),
                Column::Cause => row.push(cause.clone()),
                Column::Message => row.push(msg.clone()),
            }
        }
        table.add_row(row);
    }
    println!("{}", table);
    println!("{} {}", paint("Performance Score:", "1"), rep.performance_score);
    if let Some(pc) = &rep.perf_counters {
        println!("{}", paint("Live Performance:", "1"));
        if let Some(v) = pc.cpu_percent { println!("• CPU: {}%", v); }
        if let Some(v) = pc.avg_disk_ms_per_transfer { println!("• Avg Disk Transfer: {:.2} ms", v); }
        if let Some(v) = pc.disk_reads_per_sec { println!("• Reads/s: {}", v); }
        if let Some(v) = pc.disk_writes_per_sec { println!("• Writes/s: {}", v); }
    }
    if let Some(pred) = rep.smart_failure_predicted && pred { println!("{}", paint("SMART: Predicts failure on one or more drives", "1;31")); }
    if !rep.degradation_signals.is_empty() { println!("{}", paint("Degradation Signals:", "1")); for (n,w) in &rep.degradation_signals { println!("• {} (weight {})", n, w); } }
    if !rep.recommendations.is_empty() { println!("{}", paint("Recommendations:", "1")); for r in &rep.recommendations { println!("- {}", r); } }
    if !rep.recommendations.is_empty() { println!("{}", paint("Checklist:", "1")); for r in &rep.recommendations { println!("[ ] {}", r); } }
    if !rep.timeline.is_empty() {
        println!("{}", paint("Timeline:", "1"));
        let max_e = rep.timeline.iter().map(|(_,e,_)| *e).max().unwrap_or(1);
        let max_w = rep.timeline.iter().map(|(_,_,w)| *w).max().unwrap_or(1);
        for (t,e,w) in &rep.timeline {
            let eb = bar(*e, max_e, 20);
            let wb = bar(*w, max_w, 20);
            println!("{}  E:{:<3} {}  W:{:<3} {}", t, e, eb, w, wb);
        }
    }
}


fn bar(v: usize, max: usize, width: usize) -> String {
    if max == 0 { return String::new(); }
    let filled = ((v as f64 / max as f64) * width as f64).round() as usize;
    let mut s = String::new();
    for _ in 0..filled { s.push('█'); }
    s
}


#[allow(clippy::too_many_arguments)]
fn build_line(cols: &Vec<Column>, time: &str, sev: &str, ch: &str, pr: &str, eid: Option<&str>, cause: &str, msg: &str, tw: usize, sw: usize, chw: usize, prw: usize, ew: usize, cw: usize, mw: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in cols {
        match c {
            Column::Time => parts.push(format!("{:<tw$}", time, tw=tw)),
            Column::Severity => parts.push(format!("{:<sw$}", sev, sw=sw)),
            Column::Channel => parts.push(format!("{:<chw$}", ch, chw=chw)),
            Column::Provider => parts.push(format!("{:<prw$}", pr, prw=prw)),
            Column::EventId => parts.push(format!("{:<ew$}", eid.unwrap_or("") , ew=ew)),
            Column::Cause => parts.push(format!("{:<cw$}", cause, cw=cw)),
            Column::Message => parts.push(format!("{:<mw$}", msg, mw=mw)),
        }
    }
    parts.join(" ")
}

fn write_csv(path: &str, rep: &ReportSummary, tz: TimeZone, tfmt: Option<&str>) -> Result<(), std::io::Error> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(["time", "severity", "channel", "provider", "event_id", "cause", "message"])?;
    for e in &rep.samples {
        let ts = match (tz, tfmt) { (TimeZone::Local, Some(f)) => format!("{}", e.time.with_timezone(&Local).format(f)), (TimeZone::Utc, Some(f)) => format!("{}", e.time.format(f)), (TimeZone::Local, None) => format!("{}", e.time.with_timezone(&Local).format("%Y-%m-%d %H:%M")), (TimeZone::Utc, None) => format!("{}", e.time.format("%Y-%m-%d %H:%M")) };
        let sev = level_name(e.level);
        let cause = event_cause(e);
        let msg = event_message(e);
        wtr.write_record([ts, sev.to_string(), e.channel.clone(), e.provider.clone(), e.event_id.to_string(), cause, msg])?;
    }
    wtr.flush()?;
    Ok(())
}

fn write_tsv(path: &str, rep: &ReportSummary, tz: TimeZone, tfmt: Option<&str>) -> Result<(), std::io::Error> {
    let mut wtr = csv::WriterBuilder::new().delimiter(b'\t').from_path(path)?;
    wtr.write_record(["time", "severity", "channel", "provider", "event_id", "cause", "message"])?;
    for e in &rep.samples {
        let ts = match (tz, tfmt) { (TimeZone::Local, Some(f)) => format!("{}", e.time.with_timezone(&Local).format(f)), (TimeZone::Utc, Some(f)) => format!("{}", e.time.format(f)), (TimeZone::Local, None) => format!("{}", e.time.with_timezone(&Local).format("%Y-%m-%d %H:%M")), (TimeZone::Utc, None) => format!("{}", e.time.format("%Y-%m-%d %H:%M")) };
        let sev = level_name(e.level);
        let cause = event_cause(e);
        let msg = event_message(e);
        wtr.write_record([ts, sev.to_string(), e.channel.clone(), e.provider.clone(), e.event_id.to_string(), cause, msg])?;
    }
    wtr.flush()?;
    Ok(())
}

fn write_ndjson(path: &str, rep: &ReportSummary, tz: TimeZone, tfmt: Option<&str>, emit_eventdata: bool, emit_xml: bool) -> Result<(), std::io::Error> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    for e in &rep.samples {
        let ts = match (tz, tfmt) { (TimeZone::Local, Some(f)) => format!("{}", e.time.with_timezone(&Local).format(f)), (TimeZone::Utc, Some(f)) => format!("{}", e.time.format(f)), (TimeZone::Local, None) => format!("{}", e.time.with_timezone(&Local).format("%Y-%m-%d %H:%M")), (TimeZone::Utc, None) => format!("{}", e.time.format("%Y-%m-%d %H:%M")) };
        let mut obj = serde_json::json!({
            "time": ts,
            "severity": level_name(e.level),
            "channel": e.channel,
            "provider": e.provider,
            "event_id": e.event_id,
            "cause": event_cause(e),
            "message": event_message(e)
        });
        if emit_eventdata && let Some(xml) = e.raw_xml.as_ref()
            && let Some(map) = obj.as_object_mut() {
            let pairs = crate::event_xml::event_data_pairs_or_fallback(xml);
            map.insert("event_data".to_string(), serde_json::to_value(pairs).unwrap());
        }
        if emit_xml && let Some(xml) = e.raw_xml.as_ref()
            && let Some(map) = obj.as_object_mut() { map.insert("xml".to_string(), serde_json::Value::String(xml.clone())); }
        writeln!(file, "{}", obj)?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct NdRecord { severity: String, provider: String, event_id: u32 }

fn read_ndjson(path: &str) -> Option<Vec<NdRecord>> {
    if let Ok(data) = std::fs::read_to_string(path) {
        let mut out = Vec::new();
        for line in data.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let sev = v.get("severity").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let prv = v.get("provider").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let eid = v.get("event_id").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
                out.push(NdRecord { severity: sev, provider: prv, event_id: eid });
            }
        }
        return Some(out);
    }
    None
}

#[derive(Clone, Debug)]
struct ComparisonResult {
    delta_errors: isize,
    delta_warnings: isize,
    new_providers: Vec<String>,
    increased_event_ids: Vec<(u32, isize)>,
    decreased_event_ids: Vec<(u32, isize)>,
    provider_deltas: Vec<(String, isize)>,
    removed_providers: Vec<String>,
    new_event_ids: Vec<u32>,
}

fn compare_ndjson(base: &str, current: &str) -> Option<ComparisonResult> {
    let b = read_ndjson(base)?;
    let c = read_ndjson(current)?;
    let cnt = |v: &Vec<NdRecord>, sev: &str| -> usize { v.iter().filter(|r| r.severity.eq_ignore_ascii_case(sev)).count() };
    let be = cnt(&b, "Error") as isize + cnt(&b, "Critical") as isize;
    let bw = cnt(&b, "Warning") as isize;
    let ce = cnt(&c, "Error") as isize + cnt(&c, "Critical") as isize;
    let cw = cnt(&c, "Warning") as isize;
    use std::collections::HashMap;
    let mut bp: HashMap<String, usize> = HashMap::new(); for r in &b { let p = r.provider.to_lowercase(); *bp.entry(p).or_insert(0) += 1; }
    let mut cp: HashMap<String, usize> = HashMap::new(); for r in &c { let p = r.provider.to_lowercase(); *cp.entry(p).or_insert(0) += 1; }
    let mut new_providers: Vec<String> = Vec::new(); for p in cp.keys() { if !bp.contains_key(p) { new_providers.push(p.clone()); } }
    let mut removed_providers: Vec<String> = Vec::new(); for p in bp.keys() { if !cp.contains_key(p) { removed_providers.push(p.clone()); } }
    let mut provider_deltas: Vec<(String, isize)> = Vec::new();
    for (p, cc) in &cp { let bc = *bp.get(p).unwrap_or(&0) as isize; let d = *cc as isize - bc; if d != 0 { provider_deltas.push((p.clone(), d)); } }
    provider_deltas.sort_by(|a,b| b.1.cmp(&a.1));
    let mut beid: HashMap<u32, usize> = HashMap::new(); for r in &b { *beid.entry(r.event_id).or_insert(0) += 1; }
    let mut ceid: HashMap<u32, usize> = HashMap::new(); for r in &c { *ceid.entry(r.event_id).or_insert(0) += 1; }
    let mut incs: Vec<(u32, isize)> = Vec::new();
    let mut decs: Vec<(u32, isize)> = Vec::new();
    let mut new_event_ids: Vec<u32> = Vec::new();
    for (id, bc) in beid { let cc = *ceid.get(&id).unwrap_or(&0); let d = cc as isize - bc as isize; if d > 0 { incs.push((id, d)); } else if d < 0 { decs.push((id, d)); } }
    for (id, cc) in ceid { let bc = b.iter().filter(|r| r.event_id == id).count() as isize; if bc == 0 && cc > 0 { new_event_ids.push(id); } }
    incs.sort_by(|a,b| b.1.cmp(&a.1));
    decs.sort_by(|a,b| a.1.cmp(&b.1));
    new_event_ids.sort_unstable();
    removed_providers.sort_unstable();
    Some(ComparisonResult { delta_errors: ce - be, delta_warnings: cw - bw, new_providers, increased_event_ids: incs, decreased_event_ids: decs, provider_deltas, removed_providers, new_event_ids })
}

fn print_comparison(cmp: &ComparisonResult) {
    println!("{}", paint("Comparison (NDJSON):", "1"));
    println!("Δ Errors: {} | Δ Warnings: {}", cmp.delta_errors, cmp.delta_warnings);
    if !cmp.new_providers.is_empty() { println!("New Providers:"); for p in &cmp.new_providers { println!("• {}", p); } }
    if !cmp.removed_providers.is_empty() { println!("Removed Providers:"); for p in &cmp.removed_providers { println!("• {}", p); } }
    if !cmp.increased_event_ids.is_empty() { println!("Event IDs increased:"); for (id, d) in &cmp.increased_event_ids { println!("• {} (+{})", id, d); } }
    if !cmp.decreased_event_ids.is_empty() { println!("Event IDs decreased:"); for (id, d) in &cmp.decreased_event_ids { println!("• {} ({} )", id, d); } }
    if !cmp.new_event_ids.is_empty() { println!("New Event IDs:"); for id in &cmp.new_event_ids { println!("• {}", id); } }
    if !cmp.provider_deltas.is_empty() { println!("Provider deltas:"); for (p, d) in &cmp.provider_deltas { let sign = if *d > 0 { "+" } else { "" }; println!("• {} ({}{} )", p, sign, d); } }
}

fn write_compare_json(path: &str, cmp: &ComparisonResult) -> Result<(), std::io::Error> {
    let obj = serde_json::json!({
        "delta_errors": cmp.delta_errors,
        "delta_warnings": cmp.delta_warnings,
        "new_providers": cmp.new_providers,
        "removed_providers": cmp.removed_providers,
        "provider_deltas": cmp.provider_deltas,
        "increased_event_ids": cmp.increased_event_ids,
        "decreased_event_ids": cmp.decreased_event_ids,
        "new_event_ids": cmp.new_event_ids,
    });
    std::fs::write(path, serde_json::to_string_pretty(&obj).unwrap())
}

#[cfg(test)]
mod tests_ndjson_compare {
    use super::*;
    #[test]
    fn compare_detects_new_and_removed() {
        let dir = std::env::temp_dir();
        let b = dir.join("base.ndjson");
        let c = dir.join("curr.ndjson");
        let base = r#"{"severity":"Error","provider":"Disk","event_id":7}
{"severity":"Warning","provider":"DNS","event_id":1014}
"#;
        let curr = r#"{"severity":"Error","provider":"Disk","event_id":7}
{"severity":"Error","provider":"Disk","event_id":11}
{"severity":"Warning","provider":"Svc","event_id":7000}
"#;
        std::fs::write(&b, base).unwrap();
        std::fs::write(&c, curr).unwrap();
        let cmp = compare_ndjson(&b.to_string_lossy(), &c.to_string_lossy()).unwrap();
        assert!(cmp.new_providers.contains(&"Svc".to_string()));
        assert!(cmp.removed_providers.contains(&"DNS".to_string()));
        assert!(cmp.new_event_ids.contains(&11));
        let _ = std::fs::remove_file(&b);
        let _ = std::fs::remove_file(&c);
    }
    #[test]
    fn compare_out_writes_json() {
        let dir = std::env::temp_dir();
        let b = dir.join("base2.ndjson");
        let c = dir.join("curr2.ndjson");
        let o = dir.join("cmp.json");
        let base = r#"{"severity":"Error","provider":"Disk","event_id":7}
"#;
        let curr = r#"{"severity":"Error","provider":"disk","event_id":7}
"#;
        std::fs::write(&b, base).unwrap();
        std::fs::write(&c, curr).unwrap();
        let cmp = compare_ndjson(&b.to_string_lossy(), &c.to_string_lossy()).unwrap();
        write_compare_json(&o.to_string_lossy(), &cmp).unwrap();
        assert!(std::fs::read_to_string(&o).unwrap().contains("delta_errors"));
        let _ = std::fs::remove_file(&b);
        let _ = std::fs::remove_file(&c);
        let _ = std::fs::remove_file(&o);
    }
}

fn level_name(l: u8) -> &'static str { match l { 1 => "Critical", 2 => "Error", 3 => "Warning", 4 => "Information", _ => "Other" } }

fn truncate(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    if s.chars().count() > n { out.push_str("..."); }
    out
}

fn paint(s: &str, code: &str) -> String {
    if *ENABLE_COLOR.get().unwrap_or(&true) { format!("\x1b[{}m{}\x1b[0m", code, s) } else { s.to_string() }
}

fn sev_code(l: u8) -> &'static str { match l { 1 => "1;31", 2 => "31", 3 => "33", 4 => "34", _ => "37" } }

fn event_cause(e: &EventItem) -> String {
    let c = e.content.trim();
    if c.starts_with('<') || c.contains("<EventData>") { format!("{} {}", e.provider, e.event_id) } else { c.to_string() }
}

fn event_message(e: &EventItem) -> String { e.content.replace('\n', " ") }
fn classify_domain(provider: &str, channel: &str, event_id: u32, content: &str) -> String {
    let p = provider.to_lowercase();
    let ch = channel.to_lowercase();
    let ct = content.to_lowercase();
    // Storage / Filesystem
    if p.contains("disk") || p.contains("ntfs") || p.contains("storport") || p.contains("volmgr") || p.contains("volsnap") || ch.contains("storage") || [7u32,11,51,55,57,129,140,153,157].contains(&event_id) {
        return "Storage".to_string();
    }
    // GPU / Display
    if p.contains("display") || p.contains("nvlddmkm") || p.contains("amdkmdag") || ch.contains("graphics") || ct.contains("tdr") {
        return "GPU".to_string();
    }
    // Network / DNS
    if p.contains("dns") || p.contains("network") || ch.contains("network") || ct.contains("connect") || ct.contains("link") || ct.contains("timeout") {
        return "Network".to_string();
    }
    // Services
    if p.contains("service") || ch.contains("services") || p.contains("service control manager") {
        return "Services".to_string();
    }
    // Hardware
    if p.contains("whea") || p.contains("hardware") {
        return "Hardware".to_string();
    }
    // CPU/Power
    if p.contains("processor-power") || p.contains("kernel-processor-power") || p.contains("power") || p.contains("kernel-power") {
        return "CPU/Power".to_string();
    }
    // Permissions / DCOM
    if ct.contains("access denied") || p.contains("distributedcom") || [10016u32,10010].contains(&event_id) {
        return "Permissions".to_string();
    }
    // Time Sync
    if p.contains("w32time") || ct.contains("time service") || ct.contains("ntp") {
        return "Time Sync".to_string();
    }
    // TLS / Certificates
    if p.contains("schannel") || ct.contains("certificate") || ct.contains("tls") || ct.contains("ssl") {
        return "TLS/Certificates".to_string();
    }
    // Updates / Servicing
    if p.contains("windowsupdateclient") || ch.contains("setup") || ct.contains("update") || ct.contains("servicing") {
        return "Updates".to_string();
    }
    // USB / Device install
    if p.contains("usbhub") || p.contains("kernel-pnp") || ct.contains("usb") || ct.contains("device") {
        return "USB/Devices".to_string();
    }
    // Security / Authentication
    if ch.contains("security") || p.contains("security") || ct.contains("logon") || ct.contains("audit failure") {
        return "Security/Auth".to_string();
    }
    // Scheduler
    if p.contains("taskscheduler") {
        return "Scheduler".to_string();
    }
    "General".to_string()
}

// render_fix_markdown moved to crate::markdown module

#[cfg(target_os = "windows")]
fn open_file_default(p: PathBuf) {
    let mut s = p.to_string_lossy().into_owned();
    if s.starts_with("\\\\?\\") { s = s.trim_start_matches("\\\\?\\").to_string(); }
    if s.ends_with('\\') || s.ends_with('/') { s = s.trim_end_matches(['\\', '/']).to_string(); }
    let _ = std::process::Command::new("explorer").arg(&s).spawn()
        .or_else(|_| std::process::Command::new("cmd").args(["/C", "start", "", &s]).spawn())
        .map_err(|e| log::error!("Failed to open file {}: {}", s, e));
}

#[cfg(not(target_os = "windows"))]
fn open_file_default(p: PathBuf) {
    let s = p.to_string_lossy().into_owned();
    let _ = std::process::Command::new("xdg-open").arg(&s).spawn().map_err(|e| log::error!("Failed to open file {}: {}", s, e));
}

fn pass_level(args: &Args, level: u8) -> bool {
    if args.only_critical { return level == 1; }
    if args.only_errors { return level == 2; }
    if args.only_warnings { return level == 3; }
    if let Some(minl) = args.min_level && level < minl { return false; }
    if let Some(maxl) = args.max_level && level > maxl { return false; }
    if args.no_level_filter { true } else if args.include_info { (1..=4).contains(&level) } else { (1..=3).contains(&level) }
}

fn pass_provider(args: &Args, provider: &str) -> bool {
    if !args.providers.is_empty() {
        args.providers.iter().any(|p| p.eq_ignore_ascii_case(provider))
    } else if !args.exclude_providers.is_empty() {
        !args.exclude_providers.iter().any(|p| p.eq_ignore_ascii_case(provider))
    } else { true }
}

fn pass_event_id(args: &Args, id: u32) -> bool {
    if !args.include_event_ids.is_empty() {
        args.include_event_ids.contains(&id)
    } else if !args.exclude_event_ids.is_empty() {
        !args.exclude_event_ids.contains(&id)
    } else { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> Args {
        Args { no_open: true, ..Default::default() }
    }

    #[test]
    fn ndjson_writes_lines() {
        let rep = ReportSummary {
            window_start: Utc::now(),
            window_end: Utc::now(),
            total: 1,
            errors: 1,
            warnings: 0,
            by_provider: vec![("Disk".to_string(), 1)],
            by_channel: vec![("System".to_string(), 1)],
            by_event_id: vec![(7, 1)],
            by_device: vec![],
            by_domain: vec![],
            matched_terms: vec![],
            samples: vec![EventItem { time: Utc::now(), level: 2, channel: "System".to_string(), provider: "Disk".to_string(), event_id: 7, content: "Bad block".to_string(), raw_xml: None }],
            file_matched_terms: vec![],
            file_samples: vec![],
            scanned_records: 1,
            parsed_events: 1,
            novice_hints: vec![],
            mode: None,
            performance_score: 0,
            degradation_signals: vec![],
            recommendations: vec![],
            likely_causes: vec![],
            timeline: vec![],
            by_category: vec![],
            perf_metrics: vec![],
            perf_counters: None,
            smart_failure_predicted: None,
            risk_grade: "Unknown".to_string(),
        };
        let p = std::env::temp_dir().join("windoctor_test.ndjson");
        write_ndjson(&p.to_string_lossy(), &rep, TimeZone::Utc, None, false, false).unwrap();
        let data = std::fs::read_to_string(&p).unwrap();
        assert!(data.lines().count() >= 1);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn since_last10m() {
        let mut a = base_args();
        a.last10m = true;
        let s = compute_since(&a);
        let now = Utc::now();
        assert!(now - s >= Duration::minutes(9));
        assert!(now - s <= Duration::minutes(11));
    }

    #[test]
    fn since_hours() {
        let mut a = base_args();
        a.hours = 5;
        let s = compute_since(&a);
        let now = Utc::now();
        assert!(now - s >= Duration::hours(4));
        assert!(now - s <= Duration::hours(6));
    }

    #[test]
    fn since_last_mode_far_past() {
        let mut a = base_args();
        a.last_errors = 10;
        a.last_criticals = 10;
        let s = compute_since(&a);
        let now = Utc::now();
        assert!(now - s >= Duration::days(36000));
    }

    #[test]
    fn provider_filter_includes_only_selected() {
        let mut a = base_args();
        a.providers = vec!["Disk".to_string()];
        assert!(pass_provider(&a, "Disk"));
        assert!(!pass_provider(&a, "DistributedCOM"));
        a.providers.clear();
        a.exclude_providers = vec!["DistributedCOM".to_string()];
        assert!(pass_provider(&a, "Disk"));
        assert!(!pass_provider(&a, "DistributedCOM"));
    }

    #[test]
    fn pass_level_respects_min_max() {
        let mut a = base_args();
        a.min_level = Some(2);
        a.max_level = Some(3);
        assert!(pass_level(&a, 2));
        assert!(pass_level(&a, 3));
        assert!(!pass_level(&a, 1));
        assert!(!pass_level(&a, 4));
    }

    #[test]
    fn pass_level_respects_only_flags() {
        let mut a = base_args();
        a.only_errors = true;
        assert!(pass_level(&a, 2));
        assert!(!pass_level(&a, 3));
        a.only_errors = false;
        a.only_warnings = true;
        assert!(pass_level(&a, 3));
        assert!(!pass_level(&a, 2));
        a.only_warnings = false;
        a.only_critical = true;
        assert!(pass_level(&a, 1));
        assert!(!pass_level(&a, 2));
    }
}

#[cfg(test)]
mod tests_parse {
    use super::*;

    #[test]
    fn parse_event_xml_extracts_channel_and_level() {
        let xml = "<Event><System><TimeCreated SystemTime=\"2025-11-30T12:00:00Z\"/><Level>2</Level><Provider Name=\"Disk\"/><EventID Qualifiers=\"0\">7</EventID><Channel>System</Channel></System><EventData><Data Name=\"DeviceName\">\\\\.\\PHYSICALDRIVE0</Data></EventData></Event>";
        let item = parse_event_xml(xml, "Fallback").unwrap();
        assert_eq!(item.channel, "System");
        assert_eq!(item.level, 2);
        assert_eq!(item.provider, "Disk");
        assert_eq!(item.event_id, 7);
    }

    #[test]
    fn parse_event_xml_qx_handles_attrs() {
        let xml = "<Event><System><TimeCreated SystemTime=\"2025-11-30 12:00:00\"/><Level>3</Level><Provider Name=\"DistributedCOM\"/><EventID>10016</EventID><Channel>System</Channel></System><EventData><Data Name=\"CLSID\">{D63B10C5}</Data></EventData></Event>";
        let item = parse_event_xml(xml, "Application").unwrap();
        assert_eq!(item.level, 3);
        assert_eq!(item.provider, "DistributedCOM");
        assert_eq!(item.event_id, 10016);
        assert_eq!(item.channel, "System");
    }

    #[test]
    fn decoder_maps_disk_event_7() {
        let xml = "<Event><EventData><Data Name=\"DeviceName\">\\\\.\\PHYSICALDRIVE1</Data></EventData></Event>";
        let msg = crate::decoder::decode_event("Disk", 7, xml).unwrap();
        assert!(msg.contains("Bad block"));
    }
}

#[cfg(test)]
mod tests_domain {
    use super::*;
    #[test]
    fn classify_permissions_dcom_10016() {
        let d = classify_domain("DistributedCOM", "System", 10016, "Access denied to CLSID");
        assert_eq!(d, "Permissions");
    }
    #[test]
    fn classify_time_sync_w32time() {
        let d = classify_domain("Microsoft-Windows-Time-Service", "System", 0, "Time service NTP sync failed");
        assert_eq!(d, "Time Sync");
    }
    #[test]
    fn classify_tls_schannel() {
        let d = classify_domain("Schannel", "System", 36887, "TLS handshake failure certificate");
        assert_eq!(d, "TLS/Certificates");
    }
    #[test]
    fn classify_updates_wuclient() {
        let d = classify_domain("WindowsUpdateClient", "Setup", 0, "Update servicing failed");
        assert_eq!(d, "Updates");
    }
}

#[cfg(test)]
mod tests_sampling_limits {
    use super::*;
    #[test]
    fn per_channel_and_provider_limits_applied() {
        let now = Utc::now();
        let mut events: Vec<EventItem> = Vec::new();
        for i in 0..10 {
            events.push(EventItem { time: now - Duration::minutes(i as i64), level: 2, channel: "System".to_string(), provider: "Disk".to_string(), event_id: 7, content: format!("E{}", i), raw_xml: None });
        }
        for i in 0..10 {
            events.push(EventItem { time: now - Duration::minutes(20 + i as i64), level: 3, channel: "Application".to_string(), provider: "DistributedCOM".to_string(), event_id: 10016, content: format!("A{}", i), raw_xml: None });
        }
        let rep = build_summary_with_files(
            events,
            vec![],
            50,
            50,
            SortBy::Time,
            SortOrder::Desc,
            now - Duration::hours(1),
            now,
            vec![],
            vec![],
            0,
            20,
            None,
            None,
            None,
            None,
            Some(5),
            Some(5),
            false,
            0,
        );
        let sys = rep.samples.iter().filter(|e| e.channel == "System").count();
        let app = rep.samples.iter().filter(|e| e.channel == "Application").count();
        assert!(sys <= 5);
        assert!(app <= 5);
        let disk = rep.samples.iter().filter(|e| e.provider == "Disk").count();
        let dcom = rep.samples.iter().filter(|e| e.provider == "DistributedCOM").count();
        assert!(disk <= 5);
        assert!(dcom <= 5);
    }
}

#[cfg(test)]
mod tests_dedup_app_error {
    use super::*;
    #[test]
    fn limits_application_error_duplicates() {
        let now = Utc::now();
        let mut events: Vec<EventItem> = Vec::new();
        for i in 0..10 {
            events.push(EventItem { time: now - Duration::minutes(i as i64), level: 2, channel: "Application".to_string(), provider: "Application Error".to_string(), event_id: 1000, content: "Faulting app crash X".to_string(), raw_xml: None });
        }
        let rep = build_summary_with_files(
            events,
            vec![],
            50,
            50,
            SortBy::Time,
            SortOrder::Desc,
            now - Duration::hours(1),
            now,
            vec![],
            vec![],
            0,
            20,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            0,
        );
        let cnt = rep.samples.iter().filter(|e| e.provider == "Application Error" && event_message(e) == "Faulting app crash X" && event_cause(e) == "Application Error 1000").count();
        assert!(cnt <= 3);
    }
}

#[cfg(test)]
mod tests_truncate {
    use super::*;
    #[test]
    fn truncate_handles_multibyte() {
        let s = "你好世界";
        let t = truncate(s, 2);
        assert!(t.starts_with("你好"));
        assert!(t.ends_with("..."));
    }
    #[test]
    fn truncate_ascii() {
        let s = "abcdef";
        let t = truncate(s, 3);
        assert_eq!(t, "abc...");
        let t2 = truncate(s, 6);
        assert_eq!(t2, "abcdef");
    }
}
#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum TextFormat { Lines, Table }
#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
enum LogFormat { Text, Json }
