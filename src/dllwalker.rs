use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DllImport {
    pub name: String,
    pub resolved: Option<String>,
    pub deps: Vec<DllImport>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DllFile {
    pub path: String,
    pub imports: Vec<DllImport>,
    pub unresolved_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DllWalkResult {
    pub files: Vec<DllFile>,
}

pub fn walk(root: &str, glob: Option<&str>, recursive: bool, chain_depth: usize) -> DllWalkResult {
    let mut out: Vec<DllFile> = Vec::new();
    let mut set_opt = None;
    if let Some(g) = glob {
        let mut gb = globset::GlobSetBuilder::new();
        let glob = globset::GlobBuilder::new(g).case_insensitive(true).build().unwrap();
        gb.add(glob);
        set_opt = Some(gb.build().unwrap());
    }
    let wd = if recursive { walkdir::WalkDir::new(root) } else { walkdir::WalkDir::new(root).max_depth(1) };
    for de in wd.into_iter().filter_map(Result::ok) {
        let fp = de.path();
        if !fp.is_file() { continue; }
        if let Some(set) = &set_opt { if !set.is_match(fp) { continue; } }
        let ext = fp.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext != "dll" && ext != "exe" { continue; }
        let mut imports: Vec<DllImport> = Vec::new();
        if let Ok(bytes) = std::fs::read(fp)
            && let Ok(goblin::Object::PE(pe)) = goblin::Object::parse(&bytes) {
            for imp in pe.imports {
                let name = imp.name.to_string();
                if name.is_empty() { continue; }
                let resolved = find_on_path(&name, fp.parent());
                let mut deps: Vec<DllImport> = Vec::new();
                if chain_depth > 0 {
                    let mut visited: HashSet<String> = HashSet::new();
                    if let Some(ref path) = resolved { deps = collect_deps(path, chain_depth - 1, &mut visited); }
                }
                imports.push(DllImport { name, resolved, deps });
            }
        }
        let unresolved = imports.iter().filter(|i| i.resolved.is_none()).count();
        out.push(DllFile { path: fp.to_string_lossy().into_owned(), imports, unresolved_count: unresolved });
    }
    DllWalkResult { files: out }
}

use std::sync::OnceLock;

fn normalize_dll_name(name: &str) -> String {
    let n = name.trim();
    let nl = n.to_lowercase();
    if nl.ends_with(".dll") { n.to_string() } else { format!("{}.dll", n) }
}

fn find_on_path(dll: &str, hint_dir: Option<&std::path::Path>) -> Option<String> {
    use std::path::{Path, PathBuf};
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, Option<String>>>> = OnceLock::new();
    let name = normalize_dll_name(dll);
    let key = name.to_lowercase();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Some(v) = cache.lock().unwrap().get(&key).cloned() { return v; }
    if Path::new(&name).is_absolute() {
        let p = PathBuf::from(&name);
        let r = if p.exists() { Some(p.to_string_lossy().into_owned()) } else { None };
        let _ = cache.lock().unwrap().insert(key, r.clone());
        return r;
    }
    if let Some(dir) = hint_dir {
        let p = dir.join(&name);
        if p.exists() {
            let r = Some(p.to_string_lossy().into_owned());
            let _ = cache.lock().unwrap().insert(key, r.clone());
            return r;
        }
    }
    if let Ok(root) = std::env::var("SystemRoot") {
        for sub in ["System32", "SysWOW64", "System"].iter() {
            let p = PathBuf::from(&root).join(sub).join(&name);
            if p.exists() {
                let r = Some(p.to_string_lossy().into_owned());
                let _ = cache.lock().unwrap().insert(key, r.clone());
                return r;
            }
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join(&name);
            if p.exists() {
                let r = Some(p.to_string_lossy().into_owned());
                let _ = cache.lock().unwrap().insert(key, r.clone());
                return r;
            }
        }
    }
    let _ = cache.lock().unwrap().insert(key, None);
    None
}

fn collect_deps(path: &str, depth: usize, visited: &mut HashSet<String>) -> Vec<DllImport> {
    if depth == 0 { return Vec::new(); }
    let key = path.to_lowercase();
    if !visited.insert(key) { return Vec::new(); }
    let mut out: Vec<DllImport> = Vec::new();
    if let Ok(bytes) = std::fs::read(path)
        && let Ok(goblin::Object::PE(pe)) = goblin::Object::parse(&bytes) {
        for imp in pe.imports {
            let name = imp.name.to_string();
            if name.is_empty() { continue; }
            let resolved = find_on_path(&name, std::path::Path::new(path).parent());
            let deps = if let Some(ref p2) = resolved { collect_deps(p2, depth - 1, visited) } else { Vec::new() };
            out.push(DllImport { name, resolved, deps });
        }
    }
    out
}


pub fn render_html(res: &DllWalkResult, theme: crate::Theme) -> String {
    let mut s = String::new();
    s.push_str("<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>DLL Imports</title><style>");
    match theme {
        crate::Theme::Dark => s.push_str(":root{--bg:#0f1216;--fg:#e5e7eb;--muted:#9aa0a6;--card:#141820;--border:#1f2430;--warn:#f59e0b} body{margin:0;background:var(--bg);color:var(--fg);font-family:Segoe UI,system-ui,-apple-system,Arial,sans-serif} .container{max-width:1200px;margin:0 auto;padding:24px} .section{margin-top:18px} .table{width:100%;border-collapse:separate;border-spacing:0;background:var(--card);border:1px solid var(--border);border-radius:10px;overflow:hidden} .table th{background:#18202b;color:var(--fg);text-align:left;font-weight:600;padding:10px;border-bottom:1px solid var(--border)} .table td{padding:10px;border-bottom:1px solid var(--border)} .pill{display:inline-block;background:#1b2330;color:var(--fg);padding:4px 8px;border-radius:999px;border:1px solid var(--border);font-size:12px}"),
        crate::Theme::Light => s.push_str(":root{--bg:#f7fafc;--fg:#111827;--muted:#6b7280;--card:#ffffff;--border:#e5e7eb;--warn:#d97706} body{margin:0;background:var(--bg);color:var(--fg);font-family:Segoe UI,system-ui,-apple-system,Arial,sans-serif} .container{max-width:1200px;margin:0 auto;padding:24px} .section{margin-top:18px} .table{width:100%;border-collapse:separate;border-spacing:0;background:var(--card);border:1px solid var(--border);border-radius:10px;overflow:hidden} .table th{background:#f3f4f6;color:var(--fg);text-align:left;font-weight:600;padding:10px;border-bottom:1px solid var(--border)} .table td{padding:10px;border-bottom:1px solid var(--border)} .pill{display:inline-block;background:#eef2f7;color:var(--fg);padding:4px 8px;border-radius:999px;border:1px solid var(--border);font-size:12px}"),
    }
    s.push_str("</style></head><body><div class=\"container\"><h2>DLL Imports</h2>");
    s.push_str("<table class=\"table\"><thead><tr><th>File</th><th>Import</th><th>Resolved</th></tr></thead><tbody>");
    for f in &res.files {
        for i in &f.imports {
            let resolved = i.resolved.clone().unwrap_or_else(|| "Unresolved".to_string());
            s.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td></tr>", html_escape(&f.path), html_escape(&i.name), html_escape(&resolved)));
            if !i.deps.is_empty() { for d in &i.deps { let r = d.resolved.clone().unwrap_or_else(|| "Unresolved".to_string()); s.push_str(&format!("<tr><td></td><td>↳ {}</td><td>{}</td></tr>", html_escape(&d.name), html_escape(&r))); } }
        }
    }
    s.push_str("</tbody></table></div></body></html>");
    s
}

fn html_escape(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }
    
#[cfg(test)]
mod tests_path_resolve {
    use super::*;
    #[test]
    fn resolves_with_extension_and_hint_dir() {
        let dir = std::env::temp_dir().join("dllwalker_test");
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join("foo.dll");
        let _ = std::fs::write(&f, b"");
        let r = find_on_path("foo", Some(&dir));
        assert!(r.is_some());
        let _ = std::fs::remove_file(&f);
        let _ = std::fs::remove_dir(&dir);
    }
}
