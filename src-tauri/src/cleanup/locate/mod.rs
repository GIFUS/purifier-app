pub mod env;
pub mod fixed;
pub mod pattern;

use std::path::PathBuf;

use crate::cleanup::model::Locator;

pub fn resolve(locator: &Locator) -> Vec<PathBuf> {
    match locator {
        Locator::Fixed(path) => dedupe(vec![crate::common::resolve(path)]),
        Locator::Env(var) => env::resolve_env(var),
        Locator::Pattern { base, pattern } => pattern::resolve_pattern(base, pattern),
        Locator::Join(parts, tail) => {
            let mut out = Vec::new();
            for part in parts.iter() {
                for base in resolve(part) {
                    out.push(base.join(tail));
                }
            }
            dedupe(out)
        }
    }
}

pub fn resolve_all(locators: &[Locator]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for locator in locators {
        out.extend(resolve(locator));
    }
    dedupe(out)
}

fn dedupe(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        let key = path.to_string_lossy().to_lowercase();
        if seen.insert(key) {
            out.push(path);
        }
    }
    out
}
