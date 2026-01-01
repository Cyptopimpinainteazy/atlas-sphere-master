/// Rule miner: generates peephole optimization rules from telemetry n-grams.
///
/// Mines frequent opcode sequences and emits structured rule suggestions
/// that can be integrated into the peephole pass.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PeepholeRule {
    pub pattern: Vec<String>,
    pub replacement: Vec<String>,
    pub frequency: u64,
    pub name: String,
}

pub struct RuleMiner;

impl RuleMiner {
    /// Mine patterns from frequent n-grams and generate rule suggestions.
    /// Returns a list of suggested peephole rules.
    pub fn mine_rules(ngrams_2: &[(String, u64)], ngrams_3: &[(String, u64)]) -> Vec<PeepholeRule> {
        let mut rules = Vec::new();

        // Pattern 1: literal coalescing
        // "literal literal" -> merge into single constant
        for (gram, freq) in ngrams_2.iter() {
            if gram == "literal literal" {
                rules.push(PeepholeRule {
                    pattern: vec!["literal".to_string(), "literal".to_string()],
                    replacement: vec!["literal".to_string()],
                    frequency: *freq,
                    name: "literal_coalesce".to_string(),
                });
            }
        }

        // Pattern 2: redundant return elimination
        for (gram, freq) in ngrams_2.iter() {
            if gram == "return return" {
                rules.push(PeepholeRule {
                    pattern: vec!["return".to_string(), "return".to_string()],
                    replacement: vec!["return".to_string()],
                    frequency: *freq,
                    name: "redundant_return".to_string(),
                });
            }
        }

        // Pattern 3: binary add chaining
        for (gram, freq) in ngrams_3.iter() {
            if gram == "binary_add binary_add binary_add" {
                rules.push(PeepholeRule {
                    pattern: vec![
                        "binary_add".to_string(),
                        "binary_add".to_string(),
                        "binary_add".to_string(),
                    ],
                    replacement: vec!["binary_add".to_string()],
                    frequency: *freq,
                    name: "chain_adds".to_string(),
                });
            }
        }

        // Pattern 4: comparison + branch cluster
        for (gram, freq) in ngrams_2.iter() {
            if gram == "binary_gt branch" {
                rules.push(PeepholeRule {
                    pattern: vec!["binary_gt".to_string(), "branch".to_string()],
                    replacement: vec!["branch_if_gt".to_string()],
                    frequency: *freq,
                    name: "cmp_branch_fold".to_string(),
                });
            }
        }

        // Pattern 5: load duplicate elimination
        for (gram, freq) in ngrams_2.iter() {
            if gram == "load load" {
                rules.push(PeepholeRule {
                    pattern: vec!["load".to_string(), "load".to_string()],
                    replacement: vec!["load".to_string()],
                    frequency: *freq,
                    name: "dup_load_cse".to_string(),
                });
            }
        }

        // Sort by frequency descending for prioritization
        rules.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        rules
    }

    /// Generate Rust code template for a peephole rule.
    pub fn generate_rule_code(rule: &PeepholeRule) -> String {
        format!(
            "/// Auto-generated peephole: {}\n/// Pattern: {} => {}\n/// Frequency: {} occurrences\nfn fold_{}(statements: &[MirStatement]) -> Option<Vec<MirStatement>> {{\n    // TODO: Implement pattern matching and rewriting\n    // Pattern: {:?}\n    // Replace with: {:?}\n    None\n}}\n",
            rule.name,
            rule.pattern.join(" "),
            rule.replacement.join(" "),
            rule.frequency,
            rule.name,
            rule.pattern,
            rule.replacement
        )
    }

    /// Emit a summary of top rules for human review.
    pub fn emit_summary(rules: &[PeepholeRule]) -> String {
        let mut output = String::from("# Top Peephole Optimization Rules\n\n");

        for (idx, rule) in rules.iter().take(10).enumerate() {
            output.push_str(&format!(
                "{}. **{}**: `{}` → `{}` (freq: {})\n",
                idx + 1,
                rule.name,
                rule.pattern.join(" "),
                rule.replacement.join(" "),
                rule.frequency
            ));
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mine_basic_patterns() {
        let ngrams_2 = vec![
            ("literal literal".to_string(), 42),
            ("return return".to_string(), 15),
            ("load load".to_string(), 8),
        ];
        let ngrams_3 = vec![("binary_add binary_add binary_add".to_string(), 5)];

        let rules = RuleMiner::mine_rules(&ngrams_2, &ngrams_3);
        assert!(rules.len() > 0);
        assert!(rules[0].frequency >= 42);
    }

    #[test]
    fn generate_rule_code_template() {
        let rule = PeepholeRule {
            pattern: vec!["literal".to_string(), "literal".to_string()],
            replacement: vec!["literal".to_string()],
            frequency: 42,
            name: "test_rule".to_string(),
        };

        let code = RuleMiner::generate_rule_code(&rule);
        assert!(code.contains("test_rule"));
        assert!(code.contains("Auto-generated"));
    }
}
