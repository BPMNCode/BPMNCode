use strsim::jaro_winkler;

#[must_use]
pub fn suggest_similar(target: &str, candidates: &[&str], max_suggestions: usize) -> Vec<String> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(f64, &str)> = candidates
        .iter()
        .map(|candidate| (jaro_winkler(target, candidate), *candidate))
        .filter(|(score, _)| *score > 0.6)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    scored
        .into_iter()
        .take(max_suggestions)
        .map(|(_, candidate)| candidate.to_string())
        .collect()
}

pub const BPMN_KEYWORDS: &[&str] = &[
    "process",
    "start",
    "end",
    "task",
    "user",
    "service",
    "script",
    "call",
    "xor",
    "and",
    "event",
    "pool",
    "lane",
    "group",
    "note",
    "subprocess",
    "import",
    "from",
    "as",
];

pub const EVENT_TYPES: &[&str] = &[
    "message",
    "timer",
    "error",
    "signal",
    "terminate",
    "escalation",
    "compensation",
    "conditional",
];

pub const FLOW_TYPES: &[&str] = &["->", "-->", "=>", "..>"];

pub const ATTRIBUTE_NAMES: &[&str] = &[
    "timeout",
    "assignee",
    "priority",
    "endpoint",
    "method",
    "script",
    "params",
    "version",
    "author",
    "description",
    "collapsed",
    "parallel",
    "required",
    "secure",
    "instant",
    "form",
];

#[must_use]
pub fn suggest_keywords(target: &str) -> Vec<String> {
    suggest_similar(target, BPMN_KEYWORDS, 3)
}

#[must_use]
pub fn suggest_event_types(target: &str) -> Vec<String> {
    suggest_similar(target, EVENT_TYPES, 3)
}

#[must_use]
pub fn suggest_flow_types(target: &str) -> Vec<String> {
    suggest_similar(target, FLOW_TYPES, 2)
}

#[must_use]
pub fn suggest_attributes(target: &str) -> Vec<String> {
    suggest_similar(target, ATTRIBUTE_NAMES, 3)
}

#[must_use]
pub fn suggest_identifiers(target: &str, identifiers: &[String]) -> Vec<String> {
    let candidates: Vec<&str> = identifiers
        .iter()
        .map(std::string::String::as_str)
        .collect();
    suggest_similar(target, &candidates, 3)
}
