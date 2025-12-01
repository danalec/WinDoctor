use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RulesConfig {
    pub event_patterns: Option<Vec<String>>,
    pub file_patterns: Option<Vec<String>>,
    pub hint_rules: Option<Vec<HintRule>>,    
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HintRule {
    pub provider: Option<String>,
    pub event_id: Option<u32>,
    pub contains_any: Option<Vec<String>>, // case-insensitive substring match against event content
    pub regex: Option<String>,             // optional regex against event content
    pub category: Option<String>,
    pub severity: Option<String>,          // "high" | "medium" | "low"
    pub message: String,
}

pub fn load_rules(path_opt: Option<&str>) -> Option<RulesConfig> {
    let path = path_opt
        .map(|s| s.to_string())
        .or_else(|| std::env::var("WINDOCTOR_RULES_PATH").ok())
        .unwrap_or_else(|| "rules.json".to_string());
    let p = std::path::PathBuf::from(&path);
    let data = match std::fs::read(&p) { Ok(d) => d, Err(e) => { log::warn!("Failed to read rules file {}: {}", p.to_string_lossy(), e); return None } };
    let cfg: RulesConfig = match serde_json::from_slice(&data) { Ok(c) => c, Err(e) => { log::warn!("Failed to parse rules file {}: {}", p.to_string_lossy(), e); return None } };
    Some(cfg)
}

pub fn apply_hint_rules(events: &[crate::EventItem], cfg: &RulesConfig) -> Vec<crate::hints::NoviceHint> {
    let mut out: Vec<crate::hints::NoviceHint> = vec![];
    let rules = match &cfg.hint_rules { Some(r) => r, None => return out };
    for r in rules {
        for e in events {
            if let Some(p) = r.provider.as_ref() && e.provider != *p { continue; }
            if let Some(id) = r.event_id.as_ref() && e.event_id != *id { continue; }
            let mut matched = false;
            let content_lower = e.content.to_lowercase();
            if let Some(list) = r.contains_any.as_ref() {
                for k in list { if content_lower.contains(&k.to_lowercase()) { matched = true; break; } }
            }
            if !matched
                && let Some(rx) = r.regex.as_ref()
                && let Ok(re) = regex::Regex::new(rx) && re.is_match(&e.content) { matched = true; }
            if matched {
                let sev = r.severity.clone().unwrap_or_else(|| "medium".to_string());
                let cat = r.category.clone().unwrap_or_else(|| "General".to_string());
                out.push(crate::hints::NoviceHint { category: cat, severity: sev, message: r.message.clone(), evidence: vec![], count: 1, probability: 50 });
            }
        }
    }
    out
}
