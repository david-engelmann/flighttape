//! The timestamp grammar, pinned against the
//! interpreter the Python validator calls: every
//! line of the corpus names a string and either the
//! exact instant Python's fromisoformat reads from
//! it (microseconds since the epoch, after the
//! validator's Z replacement) or REFUSE, its refusal.
//! The corpus was generated from the interpreter
//! itself; a line that disagrees is a grammar
//! divergence, not a test to update.

use flighttape::timeparse::parse_timestamp;

#[test]
fn the_timestamp_corpus_agrees_with_the_interpreter() {
    let corpus = include_str!("fixtures/timestamp_corpus.txt");
    let mut checked = 0usize;
    for (index, line) in corpus.lines().enumerate() {
        let (expected, probe) = line
            .split_once('\t')
            .unwrap_or_else(|| panic!("corpus line {} has no tab", index + 1));
        let actual = parse_timestamp(probe);
        match expected {
            "REFUSE" => assert_eq!(
                actual,
                None,
                "corpus line {}: {:?} must be refused",
                index + 1,
                probe
            ),
            _ => {
                let micros: i128 = expected
                    .parse()
                    .unwrap_or_else(|_| panic!("corpus line {} has a bad count", index + 1));
                assert_eq!(
                    actual,
                    Some(micros),
                    "corpus line {}: {:?}",
                    index + 1,
                    probe
                );
            }
        }
        checked += 1;
    }
    assert!(
        checked >= 200,
        "the corpus carries {} probes, expected at least 200",
        checked
    );
}
