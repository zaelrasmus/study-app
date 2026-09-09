//! Review scheduling.
//!
//! There is no card content here and no card table with a front and a back. An
//! atomic note *is* the card — the title is the front, the body is the back —
//! so a card cannot drift from its source and editing a card means editing the
//! note. Only the schedule is stored.
//!
//! The queue is capped at a day's worth and the true backlog is never returned
//! by anything in this module. Coming back after three months to "412 due" is
//! the single most reliable way to abandon a review system, and it is the same
//! accumulating-debt feeling the rest of the app is built to avoid.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// How many cards a single day may show. Everything beyond this exists in the
/// database and is deliberately invisible: it is not work you can do today.
pub const DAILY_CAP: i64 = 20;

const MIN_EASE: f64 = 1.3;
const MAX_EASE: f64 = 2.8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

/// The stored schedule for one note.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReviewState {
    pub note_id: String,
    pub due_at: DateTime<Utc>,
    pub interval_days: f64,
    pub ease: f64,
    pub reps: i64,
    pub lapses: i64,
    pub last_reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What a grade does to a schedule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheduled {
    pub interval_days: f64,
    pub ease: f64,
    pub lapses_delta: i64,
}

/// SM-2 in the shape this app needs.
///
/// `Again` does not reset the interval to zero and shove the card to the front
/// of an ever-growing "learning" pile — it drops the interval hard and brings
/// the card back tomorrow. A card you failed is information, not a punishment
/// queue that grows while you are away.
pub fn schedule(current: Option<&ReviewState>, grade: Grade) -> Scheduled {
    let interval = current.map_or(0.0, |s| s.interval_days);
    let ease = current.map_or(2.5, |s| s.ease);

    let (next_interval, next_ease, lapses) = match grade {
        Grade::Again => (1.0, ease - 0.2, 1),
        Grade::Hard => ((interval.max(1.0) * 1.2).max(1.0), ease - 0.15, 0),
        Grade::Good => {
            let next = if interval < 1.0 { 1.0 } else { interval * ease };
            (next, ease, 0)
        }
        Grade::Easy => {
            let next = if interval < 1.0 { 3.0 } else { interval * ease * 1.3 };
            (next, ease + 0.15, 0)
        }
    };

    Scheduled {
        interval_days: next_interval,
        ease: next_ease.clamp(MIN_EASE, MAX_EASE),
        lapses_delta: lapses,
    }
}

pub fn due_after(now: DateTime<Utc>, interval_days: f64) -> DateTime<Utc> {
    now + Duration::seconds((interval_days * 86_400.0) as i64)
}

/// Round-robin across topics so consecutive cards rarely come from the same
/// one. Interleaving is most of what makes review transfer rather than becoming
/// pattern-matching within a single subject.
pub fn interleave<T, K: Eq + std::hash::Hash + Clone>(
    items: Vec<T>,
    key: impl Fn(&T) -> K,
) -> Vec<T> {
    let mut buckets: Vec<(K, Vec<T>)> = Vec::new();

    for item in items {
        let k = key(&item);
        match buckets.iter_mut().find(|(bk, _)| *bk == k) {
            Some((_, bucket)) => bucket.push(item),
            None => buckets.push((k, vec![item])),
        }
    }

    let mut out = Vec::new();
    let mut drained = false;

    while !drained {
        drained = true;
        for (_, bucket) in buckets.iter_mut() {
            if !bucket.is_empty() {
                out.push(bucket.remove(0));
                drained = false;
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(interval: f64, ease: f64) -> ReviewState {
        let now = Utc::now();
        ReviewState {
            note_id: "n".into(),
            due_at: now,
            interval_days: interval,
            ease,
            reps: 3,
            lapses: 0,
            last_reviewed_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn a_new_card_graded_good_comes_back_tomorrow() {
        assert_eq!(schedule(None, Grade::Good).interval_days, 1.0);
    }

    #[test]
    fn a_new_card_graded_easy_skips_ahead() {
        assert_eq!(schedule(None, Grade::Easy).interval_days, 3.0);
    }

    #[test]
    fn good_multiplies_by_ease() {
        let out = schedule(Some(&state(10.0, 2.5)), Grade::Good);
        assert_eq!(out.interval_days, 25.0);
        assert_eq!(out.ease, 2.5);
    }

    #[test]
    fn again_brings_it_back_tomorrow_rather_than_building_a_pile() {
        let out = schedule(Some(&state(40.0, 2.5)), Grade::Again);
        assert_eq!(out.interval_days, 1.0);
        assert_eq!(out.lapses_delta, 1);
        assert!(out.ease < 2.5, "a miss should cost ease");
    }

    #[test]
    fn ease_is_clamped_at_both_ends() {
        let mut s = state(10.0, 1.35);
        for _ in 0..5 {
            let out = schedule(Some(&s), Grade::Again);
            s.ease = out.ease;
        }
        assert!(s.ease >= MIN_EASE);

        let mut s = state(10.0, 2.7);
        for _ in 0..5 {
            let out = schedule(Some(&s), Grade::Easy);
            s.ease = out.ease;
        }
        assert!(s.ease <= MAX_EASE);
    }

    #[test]
    fn hard_still_moves_forward() {
        // A card graded hard must not stay due today forever.
        let out = schedule(Some(&state(5.0, 2.5)), Grade::Hard);
        assert!(out.interval_days > 5.0);
    }

    #[test]
    fn topics_are_interleaved_rather_than_blocked() {
        let items = vec![
            ("a", 1),
            ("a", 2),
            ("a", 3),
            ("b", 1),
            ("b", 2),
            ("c", 1),
        ];
        let out = interleave(items, |(topic, _)| *topic);
        let topics: Vec<_> = out.iter().map(|(t, _)| *t).collect();

        assert_eq!(topics, vec!["a", "b", "c", "a", "b", "a"]);
    }

    #[test]
    fn interleaving_one_topic_changes_nothing() {
        let items = vec![("a", 1), ("a", 2)];
        let out = interleave(items, |(t, _)| *t);
        assert_eq!(out, vec![("a", 1), ("a", 2)]);
    }
}
