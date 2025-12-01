# WinDoctor

Windows diagnostics and Event Viewer reporter for Windows 10/11.

## Quick Start
- Use `WinDoctor.exe`, or build with `cargo build --release`.
- Recent events (last 10 minutes): `WinDoctor.exe --last10m`.
- Create an HTML report: `WinDoctor.exe --last10m --html report.html`.

## Basic Features
- Scan common Event Viewer channels (System, Application).
- Export to `HTML`, `JSON`, `CSV`, `TSV`.
- Optional live mode to stream new events.
- DLL import scanning to detect missing dependencies.
- Automatic DLL diagnostics with `--dll-auto` to flag missing dependencies from events.

## Learn More
- Full parameters and examples: see `ADVANCED.md`.

## License
- Dual-licensed under `MIT OR Apache-2.0`. See `LICENSE-MIT` and `LICENSE-APACHE`.
