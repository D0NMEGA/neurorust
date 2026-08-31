//! Keeps `docs/measurement-protocol.md` honest against
//! [`nr_manifest::PreconditionCheck`] (D-06, PLAT-02). The protocol document is that
//! enum's variant list in prose, and these tests fail the moment the two drift
//! apart, in either direction: a check added to the code without being documented,
//! or a name in the document that names no real check.

const PROTOCOL_DOC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/measurement-protocol.md"
));

/// Every maximal run of ASCII letters/digits in `text`, as a separate token.
/// Punctuation, whitespace and markdown syntax (backticks, pipes, hyphens) all act
/// as separators, so a token never spans a hyphenated or backtick-wrapped phrase.
fn words(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start: Option<usize> = None;
    for (i, &b) in bytes.iter().enumerate() {
        let is_word = b.is_ascii_alphanumeric();
        match (is_word, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push(&text[s..i]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(&text[s..]);
    }
    out
}

/// Whether `word` has the shape of a [`nr_manifest::PreconditionCheck`] variant
/// name: two or more capital-initiated segments glued together with no separator
/// (e.g. `NoActiveSshSessions`), as opposed to an ordinary capitalised English word
/// (one segment, e.g. `Governor`) or an acronym with no lowercase letter at all
/// (e.g. `GDM`, `SSH`). A single leading capital is not enough on its own: every
/// sentence-case heading in this document starts with exactly one.
fn looks_like_check_name(word: &str) -> bool {
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() || !chars[0].is_ascii_uppercase() {
        return false;
    }
    let mut humps = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i].is_ascii_uppercase() {
            let seg_start = i;
            i += 1;
            while i < chars.len() && !chars[i].is_ascii_uppercase() {
                i += 1;
            }
            // A hump needs at least one lowercase/digit char after its capital; a
            // bare run of capitals (an acronym) does not count as one.
            if i > seg_start + 1 {
                humps += 1;
            }
        } else {
            i += 1;
        }
    }
    humps >= 2
}

fn known_check_names() -> Vec<String> {
    nr_manifest::PreconditionCheck::ALL
        .iter()
        .map(|check| format!("{check:?}"))
        .collect()
}

/// External identifiers the document legitimately names that happen to share the
/// PascalCase, multi-hump shape a `PreconditionCheck` variant has, but are not one:
/// systemd's own property names (`systemctl show --property=...`), not this
/// project's. Kept short and explicit rather than loosening the shape heuristic, so
/// a genuine typo of a real check name is still caught.
const KNOWN_NON_CHECK_IDENTIFIERS: &[&str] = &["ActiveState"];

/// For every `PreconditionCheck` variant, `docs/measurement-protocol.md` contains a
/// heading or table row naming that check by its exact (PascalCase, `Debug`-
/// formatted) variant name, so the consistency test can match on it.
#[test]
fn protocol_documents_every_precondition() {
    for name in known_check_names() {
        assert!(
            PROTOCOL_DOC.contains(&name),
            "docs/measurement-protocol.md is missing the {name:?} precondition; every \
             PreconditionCheck variant must appear so the document stays the harness's \
             assertion list in prose, not a separate artifact that can silently drift"
        );
    }
}

/// Every check named in the protocol document corresponds to a real
/// `PreconditionCheck` variant. Catches a typo of a real check name, or a check
/// invented in prose that the harness does not actually assert.
#[test]
fn protocol_documents_no_phantom_check() {
    let known = known_check_names();
    for word in words(PROTOCOL_DOC) {
        if looks_like_check_name(word) && !KNOWN_NON_CHECK_IDENTIFIERS.contains(&word) {
            assert!(
                known.iter().any(|name| name == word),
                "docs/measurement-protocol.md names {word:?}, which looks like a \
                 PreconditionCheck variant but is not one; either it is a typo of a \
                 real check name, or PreconditionCheck::ALL is missing a variant"
            );
        }
    }
}

/// The document is written in the project's public style: ASCII only, no em dashes,
/// no smart quotes, no emoji.
#[test]
fn protocol_is_ascii() {
    let offset = PROTOCOL_DOC.bytes().position(|b| b >= 0x80);
    assert!(
        offset.is_none(),
        "docs/measurement-protocol.md contains a non-ASCII byte at offset {offset:?}"
    );
}
