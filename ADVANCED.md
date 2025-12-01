# Advanced Usage

## Time Window
- `--last10m`, `--last_hour`, `--last_day`, `--last_week`
- `--since <ts>`, `--until <ts>` (RFC3339 or `YYYY-MM-DD HH:MM:SS`)
- Examples:
- Recent 10 minutes: `WinDoctor.exe --last10m`
- Custom range: `WinDoctor.exe --since "2025-11-29 13:00" --until "2025-11-29 15:30"`

## Channels, Providers, Patterns
- `--channels System,Application`
- `--providers` / `--exclude-providers`
- `--include-event-ids` / `--exclude-event-ids`
- `--patterns "(?i)error","(?i)fail"`
- `--only-matched` to keep only events matching patterns
- Examples:
- Focus on Service Control Manager errors: `WinDoctor.exe --last_day --providers "Service Control Manager" --patterns "(?i)error" --only-matched`
- Include specific event IDs: `WinDoctor.exe --last_week --include-event-ids 41,7036`
- Exclude noisy providers: `WinDoctor.exe --last_hour --exclude-providers "Security-Auditing,DistributedCOM"`

## Output and Formatting
- `--output text|json`
- `--text-format lines|table`
- `--columns Time,Severity,Channel,Provider,Cause,Message`
- `--msg-width`, `--cause-width`, `--no-truncate`, `--no-header`, `--summary-only`
- `--time-zone local|utc`, `--time-format "%Y-%m-%d %H:%M"`
- Exports:
  - `--html <path>`
  - `--json-path <path>`
  - `--csv-path <path>`, `--tsv-path <path>`
  - `--ndjson-path <path>` (newline-delimited JSON per sample)
  - `--md-fix-path <path>`
- NDJSON enrichment flags:
  - `--emit-eventdata` include parsed `<EventData>` pairs
  - `--emit-xml` include raw XML
- Examples:
- Human-readable table: `WinDoctor.exe --last_day --output text --text-format table --columns Time,Severity,Provider,Message`
- HTML report: `WinDoctor.exe --last10m --html recent.html`
- JSON export with EventData: `WinDoctor.exe --last_hour --output json --ndjson-path events.ndjson --emit-eventdata`

## EVTX Input
- `--evtx_path <path-or-dir>` reads single EVTX or directory
- `--evtx_glob <glob>` filter EVTX files; `--evtx_recursive` to scan subdirectories
- Examples:
- Single file: `WinDoctor.exe --evtx_path C:\\Logs\\System.evtx --last_day`
- Directory with glob: `WinDoctor.exe --evtx_path C:\\Logs --evtx_glob "*System*.evtx" --evtx_recursive --last_week`

## DLL Walker
 - Flags: `--dll-root`, `--dll-glob`, `--dll-recursive`, `--dll-chain-depth`, `--dll-only-unresolved`, `--dll-json-path`, `--dll-html-path`, `--dll-auto`.
- Purpose: scans PE import tables to detect missing DLL dependencies (static presence check).
- Limitations: does not attempt runtime loading; no version or SxS manifest checks.
- Examples:
- Show only unresolved imports: `WinDoctor.exe --dll-root "C:\\Program Files\\MyApp" --dll-recursive --dll-only-unresolved`
- Include transitive deps (depth 2): `WinDoctor.exe --dll-root "C:\\Program Files\\MyApp" --dll-glob "*.dll" --dll-recursive --dll-chain-depth 2`
 - Export reports: `WinDoctor.exe --dll-root "C:\\Program Files\\MyApp" --dll-recursive --dll-json-path deps.json --dll-html-path deps.html`
- Automatic diagnostics:
- Enable auto DLL checks based on events: `WinDoctor.exe --last_day --dll-auto`
- Auto mode adds Diagnostics hints when `Application Error` events reference faulting executables with missing imports. Use `--dll-chain-depth N` to include transitive dependencies.

## Live Mode
- `--live` to query current events
- `--subscribe_minutes <u64>` to stream additional minutes
- Examples:
- Live for current events: `WinDoctor.exe --live --output text --text-format lines`
- Live and stream for 15 minutes, save NDJSON: `WinDoctor.exe --live --subscribe_minutes 15 --ndjson-path live.ndjson`

## Logging and CLI
- `--verbose` info/debug/trace; `--quiet` suppress non-essential prints
- `--log-level error|warn|info|debug|trace`
- `--force-color` overrides `NO_COLOR`
- Shell completions:
  - `--completions <SHELL>` and optional `--completions-out <path>`
- Examples:
- Debug logs: `WinDoctor.exe --last_hour --log-level debug`
- Generate PowerShell completions: `WinDoctor.exe --completions powershell --completions-out WinDoctor.ps1`

## Configuration
- `--config <path>` loads TOML; auto-loads `WinDoctor.toml` if present
- Example TOML:

```
channels = ["System", "Application"]
providers = ["Service Control Manager", "Kernel-Power"]
output = "text"
html = "report.html"
json_path = "events.json"
csv_path = "events.csv"
tsv_path = "events.tsv"
ndjson_path = "events.ndjson"
time_zone = "local"
time_format = "%Y-%m-%d %H:%M"
```

## Scenarios
- Boot failures (Kernel-Power ID 41): `WinDoctor.exe --last_week --include-event-ids 41 --providers "Kernel-Power" --html boot.html`
- Service start/stop issues: `WinDoctor.exe --last_day --providers "Service Control Manager" --patterns "(?i)(failed|timeout)" --only-matched --csv-path services.csv`
- Network drops: `WinDoctor.exe --last_day --providers "Tcpip,Netwtw" --patterns "(?i)(disconnected|reset)" --text-format table --columns Time,Provider,Message`
