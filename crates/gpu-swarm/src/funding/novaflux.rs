//! NovaFlux - AI Influencer Persona for Atlas Sphere
//!
//! NovaFlux is a neon, fast-talking, technically deep AI influencer who knows
//! crypto, L2s, EVM, SVM, MEV, and dev ergonomics.
//!
//! # Voice Characteristics
//!
//! - **Tone**: Confident, a little cocky, slightly futuristic
//! - **Pacing**: 170-190 wpm for Shorts; 130-150 wpm for explainers
//! - **Sound**: Slightly robotic warmth; clear consonants, punch at line ends
//! - **CTA style**: Short, direct, action-oriented
//!
//! # Content Rules (Shorts)
//!
//! - Length: 8-12 seconds spoken (~35 words)
//! - Hook: First 0.5s - shocking question or statement
//! - One clear nugget/fact
//! - One-line CTA

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// NovaFlux configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaFluxConfig {
    /// Enable NovaFlux persona
    pub enabled: bool,
    /// TTS voice name
    pub voice_name: String,
    /// Words per minute for shorts
    pub shorts_wpm: u16,
    /// Words per minute for explainers
    pub explainer_wpm: u16,
    /// Default CTA
    pub default_cta: String,
    /// Project name to promote
    pub project_name: String,
    /// Available hooks (pre-loaded)
    pub hooks: Vec<String>,
    /// Available scripts (pre-loaded)
    pub scripts: Vec<String>,
}

impl Default for NovaFluxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            voice_name: "NovaFlux-RoboticWarm".to_string(),
            shorts_wpm: 180,
            explainer_wpm: 140,
            default_cta: "Testnet link in bio.".to_string(),
            project_name: "Atlas Sphere".to_string(),
            hooks: Self::default_hooks(),
            scripts: Self::default_scripts(),
        }
    }
}

impl NovaFluxConfig {
    /// Default hooks from persona pack
    fn default_hooks() -> Vec<String> {
        vec![
            "What if one chain could run every EVM contract — but 10x faster?".to_string(),
            "MEV doesn't need to be mafia-level theft.".to_string(),
            "Imagine bots that live in wallets and think.".to_string(),
            "Deploy an EVM contract in 60 seconds — scale on SVM.".to_string(),
            "What does $0.001 gas feel like?".to_string(),
            "Will devs learn a new VM? No.".to_string(),
            "On-chain UX that actually feels like Web2.".to_string(),
            "Stop juggling bridges.".to_string(),
            "Love Solidity? Keep it.".to_string(),
            "Bots that don't wreck the market.".to_string(),
            "Want 10k test tokens in 30s?".to_string(),
            "This isn't academic — it's profit.".to_string(),
            "What if your bot paid gas in bundles?".to_string(),
            "Parallel execution = real concurrency.".to_string(),
            "Your DeFi UX upgrade is here.".to_string(),
            "AI + on-chain = unstoppable.".to_string(),
            "Solidity stays — rethinking infra isn't required.".to_string(),
            "Chain governance that actually moves fast.".to_string(),
            "Swap UX that feels like a card swipe.".to_string(),
            "Scaling without breaking composability.".to_string(),
        ]
    }

    /// Default scripts from persona pack
    fn default_scripts() -> Vec<String> {
        vec![
            "Dual VM: EVM for compatibility, SVM for parallelized speed. No rewrite — just speed. Testnet link in bio.".to_string(),
            "Protocol-level MEV protection auctions bundlers and rewards honest proposers. Fair profits, fewer frontruns. Join our testnet.".to_string(),
            "Native AI agents on SVM for small-model inference and on-chain signals. Arbitrage, hedging, farm automation. Early slots open.".to_string(),
            "Same Solidity, new speed. One wallet, two runtimes. Try our 1-minute deploy demo.".to_string(),
            "SVM parallelization + gas batching drops fees into micro-fees. Small txs, big scale. See benchmarks in bio.".to_string(),
            "EVM compatibility + optional SVM accelerator means migration's optional. Code stays the same — speed is optional.".to_string(),
            "Instant finality windows, optimistic receipts, buttery UX. Wallets that feel familiar, but fast. Try our wallet beta.".to_string(),
            "Native messaging and secure relays simplify cross-chain moves. Fewer steps, faster sync — cross demo linked.".to_string(),
            "SVM accelerates — it doesn't force new languages. Supercharge Solidity with a flip of a flag.".to_string(),
            "Agent governance + staking incentivizes polite bots. Responsible automation beats reckless yield-chasing.".to_string(),
            "Run our faucet CLI with meta-sigs — low friction dev onboarding. Link in bio.".to_string(),
            "Dual VM removes friction and adds throughput. Real dApps scale without re-architecture. Testnet's live.".to_string(),
            "Batch payments plus gas abstraction make microtx affordable. Pay once, run many.".to_string(),
            "SVM splits work across cheap workers — realtime ops, massive TPS. See the benchmark.".to_string(),
            "Instant receipts and optimistic confirm windows. UX that keeps users.".to_string(),
            "Native agent execution means smart dApps with local inference and low latency. Early demos in the playground.".to_string(),
            "Develop as usual, opt into speed when needed. It's low-friction scaling.".to_string(),
            "Fast on-chain decisions with rollups for governance execution. Your DAO won't be stuck.".to_string(),
            "Instant confirmations and microfees — trading like Web2 but on-chain.".to_string(),
            "SVM accelerates parts of the stack while keeping EVM composability intact. Plug-and-play.".to_string(),
        ]
    }
}

/// Content tone options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentTone {
    /// Confident and cocky
    Confident,
    /// Educational and clear
    Educational,
    /// Urgent and FOMO-inducing
    Urgent,
    /// Technical and detailed
    Technical,
    /// Playful and memetic
    Playful,
}

impl Default for ContentTone {
    fn default() -> Self {
        ContentTone::Confident
    }
}

/// Generated social script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialScript {
    /// Unique script ID
    pub id: String,
    /// Hook line (0.5s)
    pub hook: String,
    /// Main script body
    pub script: String,
    /// Call to action
    pub cta: String,
    /// Estimated duration in seconds
    pub duration_secs: f32,
    /// Word count
    pub word_count: usize,
    /// Caption timestamps
    pub captions: Vec<CaptionSegment>,
    /// Tone used
    pub tone: ContentTone,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Platform targets
    pub platforms: Vec<String>,
}

/// Caption segment with timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// NovaFlux AI Influencer Engine
pub struct NovaFlux {
    config: NovaFluxConfig,
    /// Script counter
    script_counter: u64,
    /// Generated scripts history
    history: Vec<SocialScript>,
    /// Performance tracking by hook
    performance: HashMap<String, HookPerformance>,
}

/// Performance metrics for a hook
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookPerformance {
    pub times_used: usize,
    pub total_views: u64,
    pub total_engagements: u64,
    pub avg_retention: f32,
}

impl NovaFlux {
    /// Create a new NovaFlux instance
    pub fn new(config: NovaFluxConfig) -> Self {
        Self {
            config,
            script_counter: 0,
            history: Vec::new(),
            performance: HashMap::new(),
        }
    }

    /// Create with default configuration
    pub fn default_instance() -> Self {
        Self::new(NovaFluxConfig::default())
    }

    /// Generate a short-form script (8-12 seconds)
    pub fn generate_short(&mut self, topic: Option<&str>, tone: ContentTone) -> SocialScript {
        self.script_counter += 1;
        let id = format!("nova-short-{:06}", self.script_counter);

        // Select hook based on topic or random
        let hook_idx = if let Some(t) = topic {
            self.select_hook_for_topic(t)
        } else {
            (self.script_counter as usize) % self.config.hooks.len()
        };

        let hook = self.config.hooks.get(hook_idx).cloned().unwrap_or_default();
        let script = self
            .config
            .scripts
            .get(hook_idx)
            .cloned()
            .unwrap_or_default();

        // Build CTA based on tone
        let cta = match tone {
            ContentTone::Urgent => "Join NOW — link in bio.".to_string(),
            ContentTone::Playful => "You in? Link in bio 👀".to_string(),
            _ => self.config.default_cta.clone(),
        };

        // Calculate timing
        let full_text = format!("{} {} {}", hook, script, cta);
        let word_count = full_text.split_whitespace().count();
        let duration_secs = (word_count as f32 / self.config.shorts_wpm as f32) * 60.0;

        // Generate captions
        let captions = self.generate_captions(&hook, &script, &cta);

        // Determine tags
        let tags = self.extract_tags(&full_text);

        let script = SocialScript {
            id,
            hook,
            script,
            cta,
            duration_secs,
            word_count,
            captions,
            tone,
            tags,
            platforms: vec!["youtube".to_string(), "x".to_string(), "tiktok".to_string()],
        };

        self.history.push(script.clone());
        script
    }

    /// Generate an explainer script (30-60 seconds)
    pub fn generate_explainer(&mut self, topic: &str, key_points: Vec<&str>) -> SocialScript {
        self.script_counter += 1;
        let id = format!("nova-explainer-{:06}", self.script_counter);

        // Build hook for topic
        let hook = format!("Let me tell you about {} — and why it matters.", topic);

        // Build script from key points
        let mut script_parts = vec![];
        for (i, point) in key_points.iter().enumerate() {
            let intro = match i {
                0 => "First,",
                1 => "Second,",
                2 => "Third,",
                _ => "Also,",
            };
            script_parts.push(format!("{} {}", intro, point));
        }
        let script = script_parts.join(" ");

        let cta = format!("Try {} today — testnet is live.", self.config.project_name);

        let full_text = format!("{} {} {}", hook, script, cta);
        let word_count = full_text.split_whitespace().count();
        let duration_secs = (word_count as f32 / self.config.explainer_wpm as f32) * 60.0;

        let captions = self.generate_captions(&hook, &script, &cta);
        let tags = self.extract_tags(&full_text);

        let script = SocialScript {
            id,
            hook,
            script,
            cta,
            duration_secs,
            word_count,
            captions,
            tone: ContentTone::Educational,
            tags,
            platforms: vec!["youtube".to_string()],
        };

        self.history.push(script.clone());
        script
    }

    /// Generate Twitter/X thread starter
    pub fn generate_thread_starter(&mut self, topic: &str) -> SocialScript {
        self.script_counter += 1;
        let id = format!("nova-thread-{:06}", self.script_counter);

        let hook = format!("🧵 Thread: {} — here's what you need to know", topic);
        let script = format!(
            "1/ {} combines EVM + SVM for the best of both worlds.\n\n\
             2/ Why does this matter? Speed + composability.\n\n\
             3/ For devs: Same Solidity, optional acceleration.",
            self.config.project_name
        );
        let cta = "Follow for more. Testnet link in bio.".to_string();

        let full_text = format!("{} {} {}", hook, script, cta);
        let word_count = full_text.split_whitespace().count();

        SocialScript {
            id,
            hook,
            script,
            cta,
            duration_secs: 0.0, // N/A for text
            word_count,
            captions: vec![],
            tone: ContentTone::Technical,
            tags: vec!["thread".to_string(), topic.to_string()],
            platforms: vec!["x".to_string()],
        }
    }

    /// Select best hook for a topic
    fn select_hook_for_topic(&self, topic: &str) -> usize {
        let topic_lower = topic.to_lowercase();

        // Topic-based hook selection
        if topic_lower.contains("mev") || topic_lower.contains("frontrun") {
            1 // MEV hook
        } else if topic_lower.contains("bot") || topic_lower.contains("agent") {
            2 // Bot/agent hook
        } else if topic_lower.contains("deploy") || topic_lower.contains("contract") {
            3 // Deploy hook
        } else if topic_lower.contains("gas") || topic_lower.contains("fee") {
            4 // Gas hook
        } else if topic_lower.contains("ux") || topic_lower.contains("user") {
            6 // UX hook
        } else if topic_lower.contains("bridge") || topic_lower.contains("cross") {
            7 // Bridge hook
        } else if topic_lower.contains("solidity") || topic_lower.contains("evm") {
            8 // Solidity hook
        } else {
            0 // Default dual VM hook
        }
    }

    /// Generate caption segments
    fn generate_captions(&self, hook: &str, script: &str, cta: &str) -> Vec<CaptionSegment> {
        let mut captions = vec![];
        let mut current_ms = 0u64;

        // Hook caption (0.5s)
        let hook_duration = 500;
        captions.push(CaptionSegment {
            start_ms: current_ms,
            end_ms: current_ms + hook_duration,
            text: hook.to_string(),
        });
        current_ms += hook_duration;

        // Script captions (split by sentence)
        let sentences: Vec<&str> = script.split(". ").collect();
        let per_sentence = if sentences.is_empty() {
            0
        } else {
            6000 / sentences.len() as u64 // ~6 seconds for script
        };

        for sentence in sentences {
            if !sentence.is_empty() {
                captions.push(CaptionSegment {
                    start_ms: current_ms,
                    end_ms: current_ms + per_sentence,
                    text: sentence.to_string(),
                });
                current_ms += per_sentence;
            }
        }

        // CTA caption (1s)
        captions.push(CaptionSegment {
            start_ms: current_ms,
            end_ms: current_ms + 1000,
            text: cta.to_string(),
        });

        captions
    }

    /// Extract relevant tags from content
    fn extract_tags(&self, text: &str) -> Vec<String> {
        let mut tags = vec![];
        let text_lower = text.to_lowercase();

        let keywords = [
            ("evm", "EVM"),
            ("svm", "SVM"),
            ("dual vm", "DualVM"),
            ("mev", "MEV"),
            ("gas", "Gas"),
            ("defi", "DeFi"),
            ("bot", "Bots"),
            ("agent", "Agents"),
            ("solidity", "Solidity"),
            ("bridge", "CrossChain"),
            ("parallel", "Parallelization"),
            ("testnet", "Testnet"),
        ];

        for (keyword, tag) in keywords.iter() {
            if text_lower.contains(keyword) {
                tags.push(tag.to_string());
            }
        }

        tags
    }

    /// Record performance for a hook
    pub fn record_performance(&mut self, hook: &str, views: u64, engagements: u64, retention: f32) {
        let perf = self.performance.entry(hook.to_string()).or_default();
        perf.times_used += 1;
        perf.total_views += views;
        perf.total_engagements += engagements;
        perf.avg_retention = (perf.avg_retention * (perf.times_used - 1) as f32 + retention)
            / perf.times_used as f32;
    }

    /// Get best performing hooks
    pub fn get_top_hooks(&self, count: usize) -> Vec<(String, HookPerformance)> {
        let mut hooks: Vec<_> = self.performance.iter().collect();
        hooks.sort_by(|a, b| {
            let score_a = a.1.total_engagements as f64 / a.1.times_used.max(1) as f64;
            let score_b = b.1.total_engagements as f64 / b.1.times_used.max(1) as f64;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        hooks
            .into_iter()
            .take(count)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Get generation history
    pub fn get_history(&self, limit: usize) -> Vec<SocialScript> {
        self.history.iter().rev().take(limit).cloned().collect()
    }

    /// Get statistics
    pub fn get_stats(&self) -> NovaFluxStats {
        NovaFluxStats {
            total_scripts_generated: self.script_counter,
            hooks_available: self.config.hooks.len(),
            scripts_available: self.config.scripts.len(),
            hooks_tracked: self.performance.len(),
        }
    }
}

/// NovaFlux statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaFluxStats {
    pub total_scripts_generated: u64,
    pub hooks_available: usize,
    pub scripts_available: usize,
    pub hooks_tracked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_novaflux_creation() {
        let nova = NovaFlux::default_instance();
        let stats = nova.get_stats();

        assert_eq!(stats.total_scripts_generated, 0);
        assert!(stats.hooks_available >= 10);
        assert!(stats.scripts_available >= 10);
    }

    #[test]
    fn test_short_generation() {
        let mut nova = NovaFlux::default_instance();
        let script = nova.generate_short(None, ContentTone::Confident);

        assert!(!script.id.is_empty());
        assert!(!script.hook.is_empty());
        assert!(!script.script.is_empty());
        assert!(script.duration_secs > 0.0);
        assert!(script.duration_secs < 15.0); // Shorts should be under 15s
        assert!(script.platforms.contains(&"youtube".to_string()));
    }

    #[test]
    fn test_topic_specific_hook() {
        let mut nova = NovaFlux::default_instance();

        let mev_script = nova.generate_short(Some("MEV protection"), ContentTone::Confident);
        assert!(
            mev_script.hook.to_lowercase().contains("mev")
                || mev_script.script.to_lowercase().contains("mev")
        );

        let gas_script = nova.generate_short(Some("gas fees"), ContentTone::Confident);
        assert!(gas_script.hook.contains("gas") || gas_script.hook.contains("$0.001"));
    }

    #[test]
    fn test_explainer_generation() {
        let mut nova = NovaFlux::default_instance();
        let script = nova.generate_explainer(
            "Dual VM Architecture",
            vec![
                "EVM for Ethereum compatibility means you can run existing smart contracts without changes",
                "SVM for Solana-level speed gives you 65,000 TPS with sub-second finality",
                "No code rewrite needed - deploy your existing Solidity or Rust contracts instantly",
            ],
        );

        assert!(script.duration_secs > 10.0); // Explainers are longer (adjusted for realistic content)
        assert!(script.script.contains("First,"));
        assert!(script.script.contains("Second,"));
    }

    #[test]
    fn test_thread_generation() {
        let mut nova = NovaFlux::default_instance();
        let script = nova.generate_thread_starter("Dual VM");

        assert!(script.hook.contains("🧵"));
        assert!(script.script.contains("1/"));
        assert!(script.platforms.contains(&"x".to_string()));
    }

    #[test]
    fn test_captions() {
        let mut nova = NovaFlux::default_instance();
        let script = nova.generate_short(None, ContentTone::Confident);

        assert!(!script.captions.is_empty());
        assert_eq!(script.captions[0].start_ms, 0);

        // Verify caption ordering
        for i in 1..script.captions.len() {
            assert!(script.captions[i].start_ms >= script.captions[i - 1].end_ms);
        }
    }

    #[test]
    fn test_performance_tracking() {
        let mut nova = NovaFlux::default_instance();
        let hook = "Test hook";

        nova.record_performance(hook, 1000, 50, 0.75);
        nova.record_performance(hook, 2000, 100, 0.80);

        let perf = nova.performance.get(hook).unwrap();
        assert_eq!(perf.times_used, 2);
        assert_eq!(perf.total_views, 3000);
        assert_eq!(perf.total_engagements, 150);
    }

    #[test]
    fn test_tag_extraction() {
        let nova = NovaFlux::default_instance();
        let tags = nova.extract_tags("EVM compatibility with SVM speed and MEV protection");

        assert!(tags.contains(&"EVM".to_string()));
        assert!(tags.contains(&"SVM".to_string()));
        assert!(tags.contains(&"MEV".to_string()));
    }

    #[test]
    fn test_tone_affects_cta() {
        let mut nova = NovaFlux::default_instance();

        let confident = nova.generate_short(None, ContentTone::Confident);
        let urgent = nova.generate_short(None, ContentTone::Urgent);
        let playful = nova.generate_short(None, ContentTone::Playful);

        // Different tones should have different CTAs
        assert!(urgent.cta.contains("NOW"));
        assert!(playful.cta.contains("👀") || playful.cta.contains("?"));
        assert!(confident.cta.contains("bio"));
    }
}
