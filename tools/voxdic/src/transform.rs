use std::collections::HashSet;

use crate::config::{Profile, RewriteMode};

pub fn apply_pipeline(text: &str, profile: &Profile) -> String {
    let mut out = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if profile.strip_fillers {
        let fillers: HashSet<String> = profile
            .filler_words
            .iter()
            .map(|w| w.to_lowercase())
            .collect();
        out = out
            .split_whitespace()
            .filter(|token| {
                let cleaned = token
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                !fillers.contains(&cleaned)
            })
            .collect::<Vec<_>>()
            .join(" ");
    }

    out = apply_rewrite_mode(&out, profile.rewrite_mode);

    if profile.auto_punctuate {
        out = auto_punctuate(&out);
    }

    out.trim().to_string()
}

pub fn apply_rewrite_mode(text: &str, mode: RewriteMode) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    match mode {
        RewriteMode::None => trimmed.to_string(),
        RewriteMode::FixGrammar => fix_grammar(trimmed),
        RewriteMode::Concise => concise(trimmed),
        RewriteMode::Formal => formal(trimmed),
        RewriteMode::Bulletize => bulletize(trimmed),
    }
}

fn auto_punctuate(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() {
        return String::new();
    }
    if t.ends_with('.') || t.ends_with('!') || t.ends_with('?') {
        return capitalize_first(t);
    }
    format!("{}.", capitalize_first(t))
}

fn fix_grammar(text: &str) -> String {
    auto_punctuate(text)
}

fn concise(text: &str) -> String {
    let stop_words = [
        "basically",
        "actually",
        "really",
        "very",
        "just",
        "literally",
    ];
    let reduced = text
        .split_whitespace()
        .filter(|w| {
            let cleaned = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            !stop_words.contains(&cleaned.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut sentences = reduced
        .split_terminator(['.', '!', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join(". ");

    if !sentences.is_empty() && !sentences.ends_with('.') {
        sentences.push('.');
    }

    if sentences.is_empty() {
        reduced
    } else {
        sentences
    }
}

fn formal(text: &str) -> String {
    let mut out = text.to_string();
    let pairs = [
        ("can't", "cannot"),
        ("won't", "will not"),
        ("don't", "do not"),
        ("i'm", "I am"),
        ("it's", "it is"),
        ("you're", "you are"),
        ("we're", "we are"),
        ("they're", "they are"),
        ("didn't", "did not"),
        ("isn't", "is not"),
        ("aren't", "are not"),
    ];

    for (from, to) in pairs {
        out = out.replace(from, to);
    }

    auto_punctuate(&out)
}

fn bulletize(text: &str) -> String {
    let sentences = text
        .split_terminator(['.', '!', '?'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if sentences.is_empty() {
        return format!("- {}", text.trim());
    }

    sentences
        .into_iter()
        .map(|s| format!("- {}", s))
        .collect::<Vec<_>>()
        .join("\n")
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.collect::<String>()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{InjectApp, Profile};

    #[test]
    fn pipeline_strips_fillers() {
        let profile = Profile {
            filler_words: vec!["um".into()],
            inject_app: InjectApp::TerminalOnly,
            ..Profile::default()
        };
        let out = apply_pipeline("um hello world", &profile);
        assert!(out.starts_with("Hello"));
        assert!(out.ends_with('.'));
    }

    #[test]
    fn formal_rewrite_expands_contractions() {
        let out = apply_rewrite_mode("i'm sure we can't", RewriteMode::Formal);
        assert!(out.contains("I am"));
        assert!(out.contains("cannot"));
    }

    #[test]
    fn bulletize_mode_formats_list() {
        let out = apply_rewrite_mode("first point. second point.", RewriteMode::Bulletize);
        assert!(out.contains("- first point"));
        assert!(out.contains("- second point"));
    }
}
