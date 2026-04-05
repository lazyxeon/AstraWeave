//! Dialogue bank for the hello_companion visual demo.
//!
//! Goals:
//! - Provide large, varied dialogue libraries for all visual demo modes.
//! - Keep replies in-character (friendly NPC in a calm game world).
//! - Provide contextual filler while LLM responses are pending.

#![allow(dead_code)] // Some methods reserved for different dialogue scenarios

use rand::prelude::*;
use rand::{rngs::StdRng, SeedableRng};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::scene::DemoMode;

#[path = "dialogue_content.rs"]
mod content;

/// Collection of pre-written dialogue lines.
///
/// Note: content is stored in `dialogue_content.rs` as static slices.
#[derive(Debug, Clone, Copy, Default)]
pub struct DialogueBank;

impl DialogueBank {
    /// Create a new dialogue bank with default lines
    pub fn new() -> Self {
        Self
    }

    fn stable_hash_u64(value: impl Hash) -> u64 {
        let mut h = DefaultHasher::new();
        value.hash(&mut h);
        h.finish()
    }

    fn choose_seeded<'a>(items: &'a [&'a str], seed: u64, fallback: &'a str) -> &'a str {
        if items.is_empty() {
            return fallback;
        }
        let mut rng = StdRng::seed_from_u64(seed);
        items.choose(&mut rng).copied().unwrap_or(fallback)
    }

    fn choose_random<'a>(items: &'a [&'a str], fallback: &'a str) -> &'a str {
        if items.is_empty() {
            return fallback;
        }
        let mut rng = rand::rng();
        items.choose(&mut rng).copied().unwrap_or(fallback)
    }

    fn detect_topic(input_lower: &str) -> Topic {
        let s = input_lower;
        if s.contains("help") || s.contains("assist") || s.contains("guide") {
            return Topic::Help;
        }
        if s.contains("control")
            || s.contains("wasd")
            || s.contains("mouse")
            || s.contains("escape")
            || s.contains("mode")
            || s.contains("switch")
            || s.contains("hotkey")
        {
            return Topic::Controls;
        }
        if s.contains("weather") || s.contains("sky") || s.contains("sun") || s.contains("rain") {
            return Topic::Weather;
        }
        if s.contains("story") || s.contains("tale") || s.contains("tell me") {
            return Topic::Story;
        }
        if s.contains("quest")
            || s.contains("objective")
            || s.contains("goal")
            || s.contains("mission")
            || s.contains("plan")
            || s.contains("puzzle")
            || s.contains("hint")
            || s.contains("spoiler")
            || s.contains("clue")
        {
            return Topic::Objective;
        }
        if s.contains("summary")
            || s.contains("summarize")
            || s.contains("recap")
            || s.contains("tl;dr")
            || s.contains("tldr")
        {
            return Topic::Help;
        }
        if s.contains("decide")
            || s.contains("choice")
            || s.contains("option")
            || s.contains("tradeoff")
            || s.contains("pros and cons")
        {
            return Topic::Objective;
        }
        if s.contains("enemy")
            || s.contains("danger")
            || s.contains("fight")
            || s.contains("combat")
            || s.contains("attack")
        {
            return Topic::Combat;
        }
        if s.contains("stealth") || s.contains("hide") || s.contains("quiet") || s.contains("sneak")
        {
            return Topic::Stealth;
        }
        if s.contains("ai")
            || s.contains("model")
            || s.contains("llm")
            || s.contains("how do you work")
        {
            return Topic::Tech;
        }
        if s.contains("sad")
            || s.contains("happy")
            || s.contains("anx")
            || s.contains("stress")
            || s.contains("feel")
            || s.contains("panic")
            || s.contains("scared")
            || s.contains("afraid")
            || s.contains("lonely")
            || s.contains("overwhelmed")
            || s.contains("tired")
        {
            return Topic::Emotion;
        }
        Topic::General
    }

    fn trigger_matches(input_lower: &str, trigger: &str) -> bool {
        // Most triggers are short keywords. Use word-boundary matching for
        // single-token alphanumeric triggers to avoid false positives (e.g.
        // "hi" matching "this").
        let is_single_token = !trigger.contains(char::is_whitespace);
        let is_alnum = trigger.chars().all(|c| c.is_ascii_alphanumeric());

        if is_single_token && is_alnum {
            input_lower
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|tok| tok == trigger)
        } else {
            input_lower.contains(trigger)
        }
    }

    fn trigger_score(input_lower: &str, trigger: &str) -> Option<u32> {
        if !Self::trigger_matches(input_lower, trigger) {
            return None;
        }

        // Prefer more specific triggers:
        // - multi-word phrases outrank single words
        // - longer triggers outrank shorter triggers
        // - exact token matches for single-word alnum triggers get a bonus
        let word_count = trigger.split_whitespace().count() as u32;
        let len = trigger.len() as u32;

        let is_single_token = !trigger.contains(char::is_whitespace);
        let is_alnum = trigger.chars().all(|c| c.is_ascii_alphanumeric());
        let token_bonus = if is_single_token && is_alnum {
            // We only consider single-word alnum triggers matched via tokenization.
            10
        } else {
            0
        };

        // Weight words heavily so phrases win over substrings.
        Some(word_count.saturating_mul(100) + len + token_bonus)
    }

    fn extract_between<'a>(input: &'a str, left: &str, right: &str) -> Option<(&'a str, &'a str)> {
        let lpos = input.find(left)? + left.len();
        let rest = input.get(lpos..)?;
        let rpos = rest.find(right)?;
        let a = rest.get(..rpos)?.trim();
        let b = rest.get(rpos + right.len()..)?.trim();
        if a.is_empty() || b.is_empty() {
            return None;
        }
        Some((a, b))
    }

    fn normalize_name(name: &str) -> Option<String> {
        let cleaned = name
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != ' ')
            .trim();
        let first = cleaned.split_whitespace().next()?;
        if first.is_empty() {
            return None;
        }
        let capped = first.chars().take(32).collect::<String>();
        Some(capped)
    }

    fn goap_dynamic_response(&self, input: &str) -> Option<String> {
        let input_trim = input.trim();
        if input_trim.is_empty() {
            return None;
        }

        let lower = input_trim.to_lowercase();

        // Name parsing (lightweight; not persistent memory, but feels responsive).
        if let Some(rest) = lower.strip_prefix("my name is ") {
            let raw = input_trim
                .get((input_trim.len() - rest.len())..)
                .unwrap_or("");
            if let Some(name) = Self::normalize_name(raw) {
                return Some(format!(
                    "Nice to meet you, {}. What do you want to do next?",
                    name
                ));
            }
        }
        if let Some(rest) = lower.strip_prefix("call me ") {
            let raw = input_trim
                .get((input_trim.len() - rest.len())..)
                .unwrap_or("");
            if let Some(name) = Self::normalize_name(raw) {
                return Some(format!("Got it — I’ll call you {}. What’s the goal?", name));
            }
        }

        // Decision framing: "between X and Y".
        if let Some((a, b)) = Self::extract_between(&lower, "between ", " and ") {
            return Some(format!(
                "If you’re choosing between ‘{}’ and ‘{}’, what matters most: speed, safety, or depth?",
                a.trim(),
                b.trim()
            ));
        }

        // Decision framing: "X vs Y" or "X versus Y".
        if let Some(pos) = lower.find(" versus ") {
            let (a, b) = lower.split_at(pos);
            let b = b.trim_start_matches(" versus ").trim();
            let a = a.trim();
            if !a.is_empty() && !b.is_empty() {
                return Some(format!(
                    "If it’s ‘{}’ versus ‘{}’, tell me your priority (speed/safety/quality) and I’ll recommend one.",
                    a, b
                ));
            }
        }
        if let Some(pos) = lower.find(" vs ") {
            let (a, b) = lower.split_at(pos);
            let b = b.trim_start_matches(" vs ").trim();
            let a = a.trim();
            if !a.is_empty() && !b.is_empty() {
                return Some(format!(
                    "If it’s ‘{}’ vs ‘{}’, what’s your constraint: time, safety, or depth?",
                    a, b
                ));
            }
        }

        // Summaries / recap scaffolding.
        if lower.starts_with("summarize")
            || lower.starts_with("summary")
            || lower.starts_with("recap")
        {
            return Some(
                "Tell me what to summarize (goal, options, or the last exchange), and I’ll condense it into a next step."
                    .to_string(),
            );
        }

        // Hint style (no spoilers by default).
        if lower.contains("hint") || lower.contains("no spoilers") {
            return Some(
                "Okay — no spoilers. Tell me what you’ve tried and what result you got, and I’ll give a gentle nudge."
                    .to_string(),
            );
        }

        // --- Advanced Dynamic Patterns ---

        // Number extraction for quantitative reasoning.
        if let Some(n) = Self::extract_first_number(&lower) {
            if lower.contains("how many") || lower.contains("how much") {
                return Some(format!(
                    "You mentioned {} — is that your goal, constraint, or threshold?",
                    n
                ));
            }
            if lower.contains("minute") || lower.contains("min") {
                return Some(format!(
                    "Got it — {} minutes is our time budget. What's the success condition in that window?",
                    n
                ));
            }
            if lower.contains("second") || lower.contains("sec") {
                return Some(format!(
                    "Noted — {} seconds. We'll prioritize the fastest path.",
                    n
                ));
            }
            if lower.contains("step") {
                return Some(format!(
                    "Okay — {} steps max. Let's pick the highest-impact actions first.",
                    n
                ));
            }
        }

        // List parsing (comma or "or" separated).
        if let Some(items) = Self::extract_list(&lower) {
            if items.len() >= 2 {
                return Some(format!(
                    "You listed {} options: {}. What's the priority: speed, reliability, or depth?",
                    items.len(),
                    items.join(", ")
                ));
            }
        }

        // Follow-up question patterns.
        if lower.starts_with("what if") {
            return Some(
                "That's a good 'what if'. Tell me the scenario and I'll walk through the likely outcomes."
                    .to_string(),
            );
        }
        if lower.starts_with("why not") {
            return Some(
                "Good challenge — 'why not' reveals constraints. What option are you questioning?"
                    .to_string(),
            );
        }
        if lower.starts_with("can i") || lower.starts_with("could i") {
            return Some(
                "Usually, yes. Tell me what you want to do and what's stopping you, and I'll confirm or suggest an alternative."
                    .to_string(),
            );
        }
        if lower.starts_with("should i") {
            return Some(
                "It depends on your priority. Tell me the goal and I'll give a recommendation."
                    .to_string(),
            );
        }

        // Emotional acknowledgment patterns.
        if lower.contains("frustrated") || lower.contains("annoyed") {
            return Some(
                "Understandable. Let's reset: what's the one thing you want to accomplish right now?"
                    .to_string(),
            );
        }
        if lower.contains("excited") || lower.contains("pumped") || lower.contains("ready") {
            return Some(
                "Great energy! Let's channel it — what's the first action you want to take?"
                    .to_string(),
            );
        }
        if lower.contains("unsure") || lower.contains("not sure") || lower.contains("uncertain") {
            return Some(
                "That's okay. Uncertainty means we need more information. What's one question we could answer to reduce it?"
                    .to_string(),
            );
        }

        // Scale/rating patterns.
        if lower.contains("scale of") || lower.contains("out of 10") || lower.contains("rate") {
            return Some(
                "If you give me a number, I can calibrate: 1-3 is low priority, 4-7 is moderate, 8-10 is urgent."
                    .to_string(),
            );
        }

        // Time expressions.
        if lower.contains("in a hurry") || lower.contains("no time") || lower.contains("quick") {
            return Some(
                "Speed mode: give me the goal in one sentence and I'll give you the fastest path."
                    .to_string(),
            );
        }
        if lower.contains("take my time") || lower.contains("no rush") || lower.contains("patient")
        {
            return Some(
                "Relaxed mode: we can explore thoroughly. What do you want to understand deeply?"
                    .to_string(),
            );
        }

        // Conditional patterns ("if X then Y").
        if lower.contains(" if ") && lower.contains(" then ") {
            return Some(
                "Conditional detected. Let me trace it: what triggers the 'if' and what's the expected 'then'?"
                    .to_string(),
            );
        }

        // Negation patterns ("I don't", "I can't").
        if lower.starts_with("i don't")
            || lower.starts_with("i can't")
            || lower.starts_with("i cannot")
        {
            return Some(
                "Noted. Tell me what you *do* want or *can* do, and we'll work from there."
                    .to_string(),
            );
        }

        // Elaboration requests.
        if lower.contains("tell me more")
            || lower.contains("elaborate")
            || lower.contains("go deeper")
        {
            return Some(
                "Sure. Which part should I expand: the goal, the method, or the reasoning?"
                    .to_string(),
            );
        }

        None
    }

    /// Extract the first number from a string.
    fn extract_first_number(input: &str) -> Option<u32> {
        let mut num_str = String::new();
        for ch in input.chars() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
            } else if !num_str.is_empty() {
                break;
            }
        }
        num_str.parse().ok()
    }

    /// Extract a list of items from comma or "or" separated input.
    fn extract_list(input: &str) -> Option<Vec<String>> {
        // Look for patterns like "X, Y, or Z" or "X or Y or Z"
        let items: Vec<String> = if input.contains(',') {
            input
                .split(|c| c == ',' || c == '/')
                .map(|s| s.trim().trim_start_matches("or ").trim().to_string())
                .filter(|s| !s.is_empty() && s.len() < 50)
                .collect()
        } else if input.contains(" or ") {
            input
                .split(" or ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s.len() < 50)
                .collect()
        } else {
            return None;
        };

        if items.len() >= 2 {
            Some(items)
        } else {
            None
        }
    }

    /// GOAP response as an owned String. This allows dynamic (templated)
    /// responses that feel more intelligent than static triggers.
    pub fn goap_response_string(&self, input: &str) -> Option<String> {
        if let Some(dynamic) = self.goap_dynamic_response(input) {
            return Some(dynamic);
        }
        self.goap_response(input).map(|s| s.to_string())
    }

    fn thinking_openers_for(mode: DemoMode) -> &'static [&'static str] {
        match mode {
            DemoMode::PureLlm => content::THINKING_OPENERS_GENERAL,
            DemoMode::PureGoap => content::THINKING_OPENERS_GOAP_FLAVOR,
            DemoMode::Arbiter => content::THINKING_OPENERS_ARB_FLAVOR,
        }
    }

    fn topic_fillers_for(topic: Topic) -> &'static [&'static str] {
        match topic {
            Topic::Help => content::TOPIC_HELP_FILLER,
            Topic::Controls => content::TOPIC_CONTROLS_FILLER,
            Topic::Weather => content::TOPIC_WEATHER_FILLER,
            Topic::Story => content::TOPIC_STORY_FILLER,
            Topic::Objective => content::TOPIC_OBJECTIVE_FILLER,
            Topic::Combat => content::TOPIC_COMBAT_FILLER,
            Topic::Stealth => content::TOPIC_STEALTH_FILLER,
            Topic::Tech => content::TOPIC_TECH_FILLER,
            Topic::Emotion => content::TOPIC_EMOTION_FILLER,
            Topic::General => content::TOPIC_HELP_FILLER,
        }
    }

    /// Get a random thinking line
    pub fn random_thinking(&self) -> &'static str {
        Self::choose_random(content::THINKING_OPENERS_GENERAL, "Hmm…")
    }

    /// Get a random thinking action
    pub fn random_action(&self) -> &'static str {
        Self::choose_random(content::ACTIONS_GENERAL, "*pauses*")
    }

    /// Get a random idle line
    pub fn random_idle(&self) -> &'static str {
        Self::choose_random(content::IDLE_GENERAL, "…")
    }

    /// Get a random greeting
    pub fn random_greeting(&self) -> &'static str {
        self.greeting_for_mode(DemoMode::Arbiter)
    }

    /// Get a greeting appropriate for the current visual demo mode.
    pub fn greeting_for_mode(&self, mode: DemoMode) -> &'static str {
        match mode {
            DemoMode::PureLlm => Self::choose_random(content::GREETINGS_PURE_LLM, "Hello."),
            DemoMode::PureGoap => Self::choose_random(content::GREETINGS_PURE_GOAP, "Hello."),
            DemoMode::Arbiter => Self::choose_random(content::GREETINGS_ARBITER, "Hello."),
        }
    }

    /// General farewell line (mode-neutral).
    pub fn farewell(&self) -> &'static str {
        Self::choose_random(content::FAREWELLS_GENERAL, "Goodbye.")
    }

    /// Get a GOAP response for a trigger (for Pure GOAP mode)
    pub fn goap_response(&self, input: &str) -> Option<&'static str> {
        let input_lower = input.to_lowercase();
        let mut best: Option<(u32, usize, &'static str)> = None;

        for (idx, (trigger, response)) in content::GOAP_RESPONSES.iter().enumerate() {
            let Some(score) = Self::trigger_score(&input_lower, trigger) else {
                continue;
            };

            match best {
                None => best = Some((score, idx, *response)),
                Some((best_score, best_idx, _)) => {
                    // Higher score wins; ties go to earlier entry to preserve
                    // stable behavior when triggers are equally specific.
                    if score > best_score || (score == best_score && idx < best_idx) {
                        best = Some((score, idx, *response));
                    }
                }
            }
        }

        best.map(|(_, _, response)| response)
    }

    /// Get a fallback response for GOAP mode
    pub fn goap_fallback(&self) -> &'static str {
        Self::choose_random(content::GOAP_FALLBACKS_GENERAL, "I hear you.")
    }

    /// A GOAP fallback that tries to match the user's topic.
    pub fn goap_fallback_contextual(&self, input: &str) -> &'static str {
        let topic = Self::detect_topic(&input.to_lowercase());
        let items = match topic {
            Topic::Controls => content::TOPIC_CONTROLS_FILLER,
            Topic::Help => content::TOPIC_HELP_FILLER,
            Topic::Weather => content::TOPIC_WEATHER_FILLER,
            Topic::Story => content::TOPIC_STORY_FILLER,
            Topic::Objective => content::TOPIC_OBJECTIVE_FILLER,
            Topic::Combat => content::TOPIC_COMBAT_FILLER,
            Topic::Stealth => content::TOPIC_STEALTH_FILLER,
            Topic::Tech => content::TOPIC_TECH_FILLER,
            Topic::Emotion => content::TOPIC_EMOTION_FILLER,
            Topic::General => content::GOAP_FALLBACKS_GENERAL,
        };
        Self::choose_random(items, "Tell me a bit more.")
    }

    /// Get a sequence of context-aware thinking fillers for extended wait
    /// Analyzes user input to provide relevant filler responses
    pub fn thinking_sequence_contextual(
        &self,
        duration_seconds: f32,
        user_input: &str,
    ) -> Vec<String> {
        self.thinking_sequence_contextual_for_mode(DemoMode::Arbiter, duration_seconds, user_input)
    }

    /// Context-aware thinking fillers for a given visual demo mode.
    ///
    /// Uses a stable seed derived from (mode + user_input) so the sequence is
    /// deterministic for a given prompt (nice for demos and debugging).
    pub fn thinking_sequence_contextual_for_mode(
        &self,
        mode: DemoMode,
        duration_seconds: f32,
        user_input: &str,
    ) -> Vec<String> {
        let input_lower = user_input.to_lowercase();
        let topic = Self::detect_topic(&input_lower);
        let base_seed = Self::stable_hash_u64((mode as u8, &input_lower));

        let mut sequence = Vec::new();

        // Opener: mode-flavored acknowledgement
        let opener = Self::choose_seeded(
            Self::thinking_openers_for(mode),
            base_seed ^ 0xA11CE,
            "One moment…",
        );
        sequence.push(opener.to_string());

        // Add topic-relevant filler if wait > 3s
        if duration_seconds > 3.0 {
            let filler = Self::choose_seeded(
                Self::topic_fillers_for(topic),
                base_seed ^ 0xBEEF,
                "I’m thinking.",
            );
            sequence.push(filler.to_string());
        }

        // Add action/gesture filler if wait > 6s
        if duration_seconds > 6.0 {
            if mode == DemoMode::Arbiter {
                let progress = Self::choose_seeded(
                    content::ARB_PROGRESS_GENERAL,
                    base_seed ^ 0xCAFE,
                    "I’m checking a couple of options…",
                );
                sequence.push(progress.to_string());
            } else {
                let action =
                    Self::choose_seeded(content::ACTIONS_GENERAL, base_seed ^ 0xCAFE, "*pauses*");
                sequence.push(action.to_string());
            }
        }

        // Add deeper engagement filler if wait > 9s
        if duration_seconds > 9.0 {
            let deep = Self::choose_seeded(
                content::THINKING_DEEP_GENERAL,
                base_seed ^ 0xD00D,
                "Almost there…",
            );
            sequence.push(deep.to_string());
        }

        // Final reassurance if wait > 12s
        if duration_seconds > 12.0 {
            sequence
                .push("I’m shaping the answer carefully — thanks for your patience.".to_string());
        }

        sequence
    }

    /// In-character fallback response used when an LLM request fails.
    pub fn fallback_response_for_mode(&self, mode: DemoMode, input: &str) -> String {
        let input_lower = input.to_lowercase();
        let topic = Self::detect_topic(&input_lower);
        let seed = Self::stable_hash_u64((0xFA11_0BAD_u64, mode as u8, &input_lower));

        // Use a topic-shaped filler, but wrap it with a gentle apology.
        let filler = Self::choose_seeded(
            Self::topic_fillers_for(topic),
            seed,
            "Tell me a bit more, and we’ll take it from there.",
        );

        match mode {
            DemoMode::PureGoap => format!("I can’t answer that precisely, but: {}", filler),
            DemoMode::PureLlm => format!("*sorry* Give me a second — {}", filler),
            DemoMode::Arbiter => format!("Quick thought: {}", filler),
        }
    }

    /// Legacy thinking sequence (random, non-contextual) for backward compat
    pub fn thinking_sequence(&self, duration_seconds: f32) -> Vec<&'static str> {
        let mut out = Vec::new();
        out.push(self.random_thinking());
        if duration_seconds > 3.0 {
            out.push(self.random_action());
        }
        if duration_seconds > 6.0 {
            out.push(self.random_thinking());
        }
        if duration_seconds > 10.0 {
            out.push(self.random_action());
            out.push("This one takes a moment…");
        }
        if duration_seconds > 15.0 {
            out.push(self.random_idle());
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Topic {
    General,
    Help,
    Controls,
    Weather,
    Story,
    Objective,
    Combat,
    Stealth,
    Tech,
    Emotion,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goap_response_matches_common_triggers() {
        let bank = DialogueBank::new();
        let r = bank.goap_response("Can you explain the controls?");
        assert!(r.is_some());
    }

    #[test]
    fn contextual_thinking_sequence_is_deterministic_for_same_input() {
        let bank = DialogueBank::new();
        let a = bank.thinking_sequence_contextual_for_mode(
            DemoMode::Arbiter,
            10.0,
            "How do I switch modes and move the camera?",
        );
        let b = bank.thinking_sequence_contextual_for_mode(
            DemoMode::Arbiter,
            10.0,
            "How do I switch modes and move the camera?",
        );
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn short_trigger_does_not_match_inside_other_words() {
        let bank = DialogueBank::new();
        // Previously, "hi" could match the substring in "this".
        let r = bank
            .goap_response("This place feels calm.")
            .unwrap_or("<none>");
        assert!(
            r.contains("peaceful") || r.contains("quiet"),
            "response={r}"
        );

        let hi = bank.goap_response("hi").unwrap_or("<none>");
        assert!(hi.starts_with("Hi") || hi.starts_with("Hello") || hi.starts_with("Hey"));
    }

    #[test]
    fn specific_phrase_outranks_generic_trigger() {
        let bank = DialogueBank::new();
        // Should prefer "switch" guidance over the generic "mode" line.
        let r = bank
            .goap_response("How do I switch modes?")
            .unwrap_or("<none>");
        assert!(
            r.contains("Press 1") || r.contains("switch"),
            "response={r}"
        );
    }

    #[test]
    fn dynamic_between_parsing_produces_question() {
        let bank = DialogueBank::new();
        let r = bank
            .goap_response_string("Help me decide between stealth and speed")
            .unwrap_or_default();
        assert!(r.contains("choosing between"), "response={r}");
    }
}

// `Default` is derived.
