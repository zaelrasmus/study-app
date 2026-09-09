//! The two axes of a note's knowledge state.
//!
//! Everything here is derived from stored facts. Nothing in this module is
//! written to the database, which is why there are no illegal transitions to
//! guard and no way for a note to get stuck.
//!
//! **Axis 1** is [`RecallState`] — one word, shown as the primary state.
//! **Axis 2** is [`NoteFlags::never_contrasted`] — a separate marker, because
//! "reproduced from memory" and "checked against a source" are independent.
//! A note can be perfectly recalled and confidently wrong; collapsing that into
//! one adjective is what made the earlier vocabulary lie.
//!
//! Nothing here decays with time. Drift and failure are events, not clocks.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::Note;

/// A recall made within this window of an ordinary edit is recorded, but does
/// not clear [`RecallState::EditedSinceRecall`].
///
/// Reciting text you wrote ninety seconds ago is short-term memory, not
/// knowledge. Without this, editing and immediately re-recalling would restore
/// `recalled` for free — and it is exactly the shortcut a fast iterator takes.
pub const RECALL_COOLING_HOURS: i64 = 24;

/// Axis 1. Exactly one of these is true at any moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecallState {
    /// Captured, never written into.
    Raw,
    /// Written with the source visible. A draft for this session, not forever.
    Draft,
    /// Reproduced blind, and unchanged since.
    Recalled,
    /// The body changed after the recall that produced it.
    EditedSinceRecall,
    /// Recalled once, then failed in review. Distinct from never having tried.
    Failing,
}

impl RecallState {
    /// The word shown on the card. English, always — the interface language is
    /// settled and there are no carve-outs, not even for state vocabulary.
    pub fn label(self) -> &'static str {
        match self {
            Self::Raw => "not distilled",
            Self::Draft => "draft",
            Self::Recalled => "recalled",
            Self::EditedSinceRecall => "edited since recall",
            Self::Failing => "failing",
        }
    }

    /// Whether the state names something to do.
    ///
    /// Only two states carry colour: the one worth reaching, and the one asking
    /// for action. `raw` and `draft` are uncoloured, so the palette can never
    /// read as a grade.
    pub fn needs_action(self) -> bool {
        matches!(self, Self::EditedSinceRecall | Self::Failing)
    }

    /// Whether this state is rendered at all.
    ///
    /// A note that has never been reproduced from memory has nothing to say
    /// about memory, so it says nothing -- on any surface, on any board. This
    /// is what stops a note you only wanted to write from being labelled with
    /// how far it is from the end of a pipeline it was never entered into.
    ///
    /// It also means the only states ever shown are the two that carry colour,
    /// which is why the palette can never read as a grade.
    pub fn is_visible(self) -> bool {
        !matches!(self, Self::Raw | Self::Draft)
    }

    /// A raw note has no worked body, so it never enters a document even when
    /// it sits inside a frame. Frame membership is necessary, not sufficient.
    pub fn may_enter_document(self) -> bool {
        self != Self::Raw
    }

    /// An atomic note *is* a card: front is the title, back is the body. A card
    /// therefore exists exactly when the note has been reproduced blind.
    pub fn may_review(self) -> bool {
        matches!(self, Self::Recalled | Self::Failing | Self::EditedSinceRecall)
    }
}

/// Both axes, computed together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NoteFlags {
    pub state: RecallState,
    /// False for notes that never attempted memory. Computed here rather than
    /// in the frontend so no surface can get the rule wrong.
    pub visible: bool,
    /// Axis 2: reproduced from memory but never contrasted against a source.
    /// Coexists with any recall state rather than replacing it.
    pub never_contrasted: bool,
}

impl Note {
    /// Axis 1.
    ///
    /// Precedence is deliberate: `failing` outranks `edited since recall`,
    /// because demonstrably not knowing something is a stronger signal than the
    /// text having moved. Both are superseded by a newer recall, which is why
    /// neither needs clearing — a successful recall simply becomes the most
    /// recent event.
    pub fn recall_state(&self) -> RecallState {
        let Some(recalled_at) = self.last_recall_at else {
            return match self.distilled_at {
                Some(_) => RecallState::Draft,
                None => RecallState::Raw,
            };
        };

        if self
            .last_recall_failed_at
            .is_some_and(|failed_at| failed_at > recalled_at)
        {
            return RecallState::Failing;
        }

        if self.content_hash_at_recall.as_deref() != Some(self.content_hash.as_str()) {
            return RecallState::EditedSinceRecall;
        }

        RecallState::Recalled
    }

    /// Axis 2. Only meaningful once something has been recalled: an unrecalled
    /// note is not yet claiming to be right about anything.
    pub fn never_contrasted(&self) -> bool {
        self.last_recall_at.is_some() && self.last_checked_at.is_none()
    }

    pub fn flags(&self) -> NoteFlags {
        let state = self.recall_state();
        NoteFlags {
            state,
            visible: state.is_visible(),
            never_contrasted: self.never_contrasted(),
        }
    }

    /// True when a recall recorded at `at` is too close to the last ordinary
    /// edit to count as knowledge.
    ///
    /// The recall is still recorded — the app does not refuse the action or
    /// scold — it simply does not update `content_hash_at_recall`, so the note
    /// keeps reading as edited until the recall survives a night's sleep.
    pub fn recall_is_cooling(&self, at: DateTime<Utc>) -> bool {
        self.content_edited_at
            .is_some_and(|edited| at - edited < Duration::hours(RECALL_COOLING_HOURS))
    }
}

/// Folds Spanish text for matching: search and deduplication only.
///
/// Reconstruction never uses this. Round two is manual self-marking, and a
/// machine that fuzzy-matched recall would be judging it.
///
/// The trap: the usual `NFD` + strip-combining-marks recipe destroys `ñ`,
/// collapsing `año` into `ano`. Only the five accented vowels are folded, and
/// `ñ`/`ü` are left standing.
pub fn fold_es(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'û' => 'u',
            'Á' | 'À' | 'Ä' | 'Â' => 'A',
            'É' | 'È' | 'Ë' | 'Ê' => 'E',
            'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
            'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
            'Ú' | 'Ù' | 'Û' => 'U',
            other => other,
        })
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Case- and accent-insensitive containment, for search boxes.
pub fn matches_es(haystack: &str, needle: &str) -> bool {
    fold_es(haystack).contains(&fold_es(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{content_hash, new_id, NoteKind};

    fn note(body: &str) -> Note {
        let now = Utc::now();
        Note {
            id: new_id(),
            kind: NoteKind::Atomic,
            title: "t".into(),
            body_json: String::new(),
            body_html: String::new(),
            body_text: body.into(),
            content_hash: content_hash(body),
            distilled_at: None,
            last_recall_at: None,
            last_checked_at: None,
            last_recall_failed_at: None,
            content_edited_at: None,
            summary: String::new(),
            content_hash_at_recall: None,
            source_note_id: None,
            journal_day: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Written blind and saved: the recall write, in one place.
    fn recalled(body: &str, when: DateTime<Utc>) -> Note {
        let mut n = note(body);
        n.distilled_at = Some(when);
        n.last_recall_at = Some(when);
        n.content_hash_at_recall = Some(n.content_hash.clone());
        n
    }

    #[test]
    fn a_capture_with_no_body_is_raw() {
        assert_eq!(note("").recall_state(), RecallState::Raw);
    }

    #[test]
    fn written_but_never_recalled_is_draft() {
        let mut n = note("something");
        n.distilled_at = Some(Utc::now());
        assert_eq!(n.recall_state(), RecallState::Draft);
    }

    #[test]
    fn reproduced_blind_is_recalled() {
        assert_eq!(recalled("V = IR", Utc::now()).recall_state(), RecallState::Recalled);
    }

    #[test]
    fn editing_after_a_recall_reads_as_edited_not_as_decay() {
        let mut n = recalled("V = IR", Utc::now());
        n.body_text = "V = IR for ohmic conductors".into();
        n.content_hash = content_hash(&n.body_text);
        assert_eq!(n.recall_state(), RecallState::EditedSinceRecall);
    }

    #[test]
    fn reformatting_alone_does_not_disturb_a_recall() {
        let mut n = recalled("V = IR", Utc::now());
        n.body_text = "  V = IR  ".into();
        n.content_hash = content_hash(&n.body_text);
        assert_eq!(n.recall_state(), RecallState::Recalled);
    }

    #[test]
    fn nothing_decays_with_time() {
        let ancient = Utc::now() - Duration::days(900);
        assert_eq!(recalled("V = IR", ancient).recall_state(), RecallState::Recalled);
    }

    #[test]
    fn a_failure_after_a_recall_is_failing_not_draft() {
        let when = Utc::now() - Duration::days(2);
        let mut n = recalled("V = IR", when);
        n.last_recall_failed_at = Some(Utc::now());

        assert_eq!(n.recall_state(), RecallState::Failing);
        // The recall itself is not erased: "recalled but failing" and "never
        // recalled" stay different facts.
        assert!(n.last_recall_at.is_some());
    }

    #[test]
    fn a_newer_recall_supersedes_an_older_failure() {
        let mut n = recalled("V = IR", Utc::now());
        n.last_recall_failed_at = Some(Utc::now() - Duration::days(5));
        assert_eq!(n.recall_state(), RecallState::Recalled);
    }

    #[test]
    fn failing_outranks_edited() {
        let when = Utc::now() - Duration::days(2);
        let mut n = recalled("V = IR", when);
        n.last_recall_failed_at = Some(Utc::now());
        n.body_text = "changed".into();
        n.content_hash = content_hash(&n.body_text);

        assert_eq!(n.recall_state(), RecallState::Failing);
    }

    #[test]
    fn recall_and_contrast_are_independent_axes() {
        let n = recalled("V = IR", Utc::now());
        // Recalled perfectly, never compared against anything: the case where a
        // note can be confidently wrong.
        assert_eq!(n.recall_state(), RecallState::Recalled);
        assert!(n.never_contrasted());

        let mut checked = n.clone();
        checked.last_checked_at = Some(Utc::now());
        assert_eq!(checked.recall_state(), RecallState::Recalled);
        assert!(!checked.never_contrasted());
    }

    #[test]
    fn an_unrecalled_note_is_not_marked_uncontrasted() {
        let mut n = note("something");
        n.distilled_at = Some(Utc::now());
        assert!(!n.never_contrasted());
    }

    #[test]
    fn a_recall_within_a_day_of_an_edit_is_cooling() {
        let mut n = note("V = IR");
        n.content_edited_at = Some(Utc::now() - Duration::hours(2));
        assert!(n.recall_is_cooling(Utc::now()));

        n.content_edited_at = Some(Utc::now() - Duration::hours(30));
        assert!(!n.recall_is_cooling(Utc::now()));
    }

    #[test]
    fn a_note_that_never_attempted_memory_says_nothing() {
        // The whole point: a note you only wanted to write is not labelled with
        // how far it is from the end of a pipeline it never entered.
        assert!(!RecallState::Raw.is_visible());
        assert!(!RecallState::Draft.is_visible());

        assert!(RecallState::Recalled.is_visible());
        assert!(RecallState::EditedSinceRecall.is_visible());
        assert!(RecallState::Failing.is_visible());
    }

    #[test]
    fn everything_visible_is_exactly_what_carries_colour_or_is_recalled() {
        for state in [
            RecallState::Raw,
            RecallState::Draft,
            RecallState::Recalled,
            RecallState::EditedSinceRecall,
            RecallState::Failing,
        ] {
            let coloured = state == RecallState::Recalled || state.needs_action();
            assert_eq!(state.is_visible(), coloured, "{state:?}");
        }
    }

    #[test]
    fn visibility_tracks_whether_a_recall_was_ever_attempted() {
        let mut n = note("a");
        n.distilled_at = Some(Utc::now());
        assert!(n.last_recall_at.is_none());
        assert!(!n.flags().visible);

        let n = recalled("a", Utc::now());
        assert!(n.last_recall_at.is_some());
        assert!(n.flags().visible);
    }

    #[test]
    fn only_two_states_carry_colour() {
        assert!(!RecallState::Raw.needs_action());
        assert!(!RecallState::Draft.needs_action());
        assert!(!RecallState::Recalled.needs_action());
        assert!(RecallState::EditedSinceRecall.needs_action());
        assert!(RecallState::Failing.needs_action());
    }

    #[test]
    fn raw_notes_never_enter_a_document() {
        assert!(!RecallState::Raw.may_enter_document());
        assert!(RecallState::Draft.may_enter_document());
    }

    #[test]
    fn a_card_exists_only_once_something_has_been_recalled() {
        assert!(!RecallState::Raw.may_review());
        assert!(!RecallState::Draft.may_review());
        assert!(RecallState::Recalled.may_review());
        assert!(RecallState::Failing.may_review());
    }

    #[test]
    fn folding_spanish_keeps_the_enye() {
        // The whole point: `año` and `ano` are different words.
        assert_eq!(fold_es("año"), "año");
        assert_ne!(fold_es("año"), fold_es("ano"));

        assert_eq!(fold_es("localización"), "localizacion");
        assert!(matches_es("Localización de memoria", "localizacion"));
        assert!(matches_es("El Niño", "nino") == false);
        assert!(matches_es("El Niño", "niño"));
    }

    #[test]
    fn folding_is_case_insensitive() {
        assert_eq!(fold_es("ÁRBOL"), "arbol");
        assert!(matches_es("Árbol de decisión", "ARBOL"));
    }
}
