use globset::{GlobBuilder, GlobSetBuilder};
use walkdir::WalkDir;
use regex::Regex;
use std::io::{BufRead, BufReader};

#[derive(Clone, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FileSample {
    pub path: String,
    pub pattern: String,
    pub line_no: u64,
    pub line: String,
}

#[derive(Clone, Debug)]
pub struct FileScanSummary {
    pub by_term: Vec<(String, usize)>,
    pub samples: Vec<FileSample>,
}

pub fn scan(root: &str, file_glob: Option<&str>, patterns: &[String], top: usize) -> FileScanSummary {
    let mut set_opt = None;
    if let Some(g) = file_glob {
        let mut gs = GlobSetBuilder::new();
        let glob = GlobBuilder::new(g).case_insensitive(true).build().unwrap();
        gs.add(glob);
        set_opt = Some(gs.build().unwrap());
    }
    let mut term_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut samples: Vec<FileSample> = vec![];
    let mut matchers: Vec<(String, Regex)> = vec![];
    for p in patterns { if let Ok(m) = Regex::new(p) { matchers.push((p.clone(), m)); } }
    for de in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        let p = de.path();
        if !p.is_file() { continue; }
        if let Some(set) = &set_opt && !set.is_match(p) { continue; }
        let path_str = p.to_string_lossy().to_string();
        let f = match std::fs::File::open(p) { Ok(f) => f, Err(_) => continue };
        let mut hits: Vec<bool> = vec![false; matchers.len()];
        let mut br = BufReader::new(f);
        let mut line = String::new();
        let mut idx: u64 = 0;
        loop {
            line.clear();
            let read = br.read_line(&mut line).unwrap_or(0);
            if read == 0 { break; }
            idx += 1;
            if samples.len() >= top { break; }
            for (i, (pat, re)) in matchers.iter().enumerate() {
                if re.is_match(line.trim_end()) {
                    hits[i] = true;
                    if samples.len() < top { samples.push(FileSample { path: path_str.clone(), pattern: pat.clone(), line_no: idx, line: line.trim_end().to_string() }); }
                }
            }
        }
        for (i, (pat, _)) in matchers.iter().enumerate() { if hits[i] { *term_counts.entry(pat.clone()).or_insert(0) += 1; } }
    }
    let mut by_term: Vec<(String, usize)> = term_counts.into_iter().collect();
    by_term.sort_by(|a, b| b.1.cmp(&a.1));
    FileScanSummary { by_term, samples }
}
