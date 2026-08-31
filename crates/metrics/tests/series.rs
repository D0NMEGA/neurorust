use nr_manifest::{ContaminationVerdict, InstrumentClass, RunClass};
use nr_metrics::coverage::{Coverage, GapReason, WeekRecord, record_gap, record_gaps};
use nr_metrics::series::{MetricsSeries, SeriesError, StageMetrics, append};
use time::macros::datetime;

fn base_entry() -> StageMetrics {
    StageMetrics {
        run_id: "2026-08-31-precision3591-weekly-001".to_string(),
        run_class: RunClass::Weekly,
        instrument_class: InstrumentClass::HeadlineSeries,
        utc_start: datetime!(2026-08-31 06:00:00 UTC),
        iso_week: "2026-W35".to_string(),
        rig_slug: "precision3591".to_string(),
        tool: "cyclictest".to_string(),
        stage: "cyclictest.wakeup_latency".to_string(),
        sample_count: 18_000_000,
        overflow_count: 888,
        p50_us: 2,
        p95_us: 6,
        p99_us: 9,
        p999_us: 12,
        max_us: 27,
        contamination_verdict: ContaminationVerdict::Clean,
        excluded_from_series: false,
        exclusion_reason: None,
        manifest_blake3: "a".repeat(64),
    }
}

/// schema_roundtrip: a MetricsSeries with one cyclictest entry serialises, validates against
/// schemas/metrics.schema.json, and deserialises back to an equal value. Every entry carries
/// p50_us, p95_us and p99_us, and a series entry missing any of the three fields fails to
/// deserialise.
#[test]
fn schema_roundtrip() {
    let series = MetricsSeries {
        schema_version: 1,
        entries: vec![base_entry()],
    };

    let json = serde_json::to_string(&series).expect("a MetricsSeries always serialises");
    let value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
    let entry = &value["entries"][0];
    for field in ["p50_us", "p95_us", "p99_us"] {
        assert!(entry.get(field).is_some(), "missing {field} in {entry}");
    }

    let round_tripped: MetricsSeries =
        serde_json::from_str(&json).expect("a well-formed series always deserialises");
    assert_eq!(round_tripped, series);

    for field in ["p50_us", "p95_us", "p99_us"] {
        let mut broken = value.clone();
        broken["entries"][0].as_object_mut().unwrap().remove(field);
        let result: Result<MetricsSeries, _> = serde_json::from_value(broken);
        assert!(
            result.is_err(),
            "expected a series entry missing {field} to fail deserialization"
        );
    }
}

/// The committed schema is generated, never hand-authored (RESEARCH.md pattern 3). CI runs this
/// without UPDATE_SCHEMAS set, so a drifted schema fails the build.
#[test]
fn metrics_schema_up_to_date() {
    let generated = nr_metrics::schema::metrics_schema_json();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/metrics.schema.json"
    );

    if std::env::var("UPDATE_SCHEMAS").is_ok() {
        std::fs::write(path, &generated).unwrap();
    }

    let committed = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        generated, committed,
        "schemas/metrics.schema.json is stale. Regenerate with: \
         UPDATE_SCHEMAS=1 cargo test -p nr-metrics metrics_schema_up_to_date"
    );
}

/// An entry with stage "emergency_stop.abort_latency" round trips, proving the schema is not
/// cyclictest shaped (D-04).
#[test]
fn stage_names_are_open() {
    let entry = StageMetrics {
        stage: "emergency_stop.abort_latency".to_string(),
        tool: "nr-stop-harness".to_string(),
        ..base_entry()
    };

    let json = serde_json::to_string(&entry).expect("serialises");
    let round_tripped: StageMetrics = serde_json::from_str(&json).expect("deserialises");
    assert_eq!(round_tripped, entry);
    assert_eq!(round_tripped.stage, "emergency_stop.abort_latency");
}

/// Appending keeps entries sorted by utc_start regardless of insertion order, and a duplicate
/// run_id is rejected rather than written as a second copy.
#[test]
fn append_is_ordered_and_idempotent() {
    let mut series = MetricsSeries {
        schema_version: 1,
        entries: Vec::new(),
    };

    let later = StageMetrics {
        run_id: "run-later".to_string(),
        utc_start: datetime!(2026-08-31 12:00:00 UTC),
        ..base_entry()
    };
    let earlier = StageMetrics {
        run_id: "run-earlier".to_string(),
        utc_start: datetime!(2026-08-31 06:00:00 UTC),
        ..base_entry()
    };

    append(&mut series, later.clone()).expect("first append succeeds");
    append(&mut series, earlier.clone()).expect("second append succeeds");

    assert_eq!(series.entries.len(), 2);
    assert_eq!(series.entries[0].run_id, "run-earlier");
    assert_eq!(series.entries[1].run_id, "run-later");

    let result = append(&mut series, earlier);
    assert_eq!(
        result,
        Err(SeriesError::DuplicateRunId("run-earlier".to_string()))
    );
    assert_eq!(
        series.entries.len(),
        2,
        "a duplicate run_id must not add a second copy"
    );
}

/// BENCH-06: a run whose manifest verdict is Contaminated is still appended, with
/// excluded_from_series true and a non-empty exclusion_reason. It is not skipped.
#[test]
fn contaminated_run_retained() {
    let mut series = MetricsSeries {
        schema_version: 1,
        entries: Vec::new(),
    };

    let contaminated = StageMetrics {
        run_id: "run-contaminated".to_string(),
        contamination_verdict: ContaminationVerdict::Contaminated,
        excluded_from_series: true,
        exclusion_reason: Some("CAL IPI count on cpu6 exceeded threshold".to_string()),
        ..base_entry()
    };

    append(&mut series, contaminated).expect("a contaminated run is appended, not skipped");

    assert_eq!(series.entries.len(), 1);
    assert_eq!(
        series.entries[0].contamination_verdict,
        ContaminationVerdict::Contaminated
    );
    assert!(series.entries[0].excluded_from_series);
    assert!(series.entries[0].exclusion_reason.is_some());
}

/// Given a series whose last entry is in ISO week 2026-W35 and a new entry in 2026-W38,
/// record_gaps writes gap entries for 2026-W36 and 2026-W37 with reason "no run recorded".
#[test]
fn missed_week_records_gap() {
    let mut coverage = Coverage {
        schema_version: 1,
        weeks: vec![WeekRecord::Run {
            iso_week: "2026-W35".to_string(),
            run_ids: vec!["run-w35".to_string()],
        }],
    };

    let new_entry = StageMetrics {
        iso_week: "2026-W38".to_string(),
        ..base_entry()
    };

    let filled = record_gaps(&mut coverage, &new_entry);

    assert_eq!(filled, vec!["2026-W36".to_string(), "2026-W37".to_string()]);
    assert_eq!(coverage.weeks.len(), 3);
    assert!(matches!(
        &coverage.weeks[1],
        WeekRecord::Gap { iso_week, reason: GapReason::NoRunRecorded } if iso_week == "2026-W36"
    ));
    assert!(matches!(
        &coverage.weeks[2],
        WeekRecord::Gap { iso_week, reason: GapReason::NoRunRecorded } if iso_week == "2026-W37"
    ));
}

/// record_gaps writes a Gap entry only; it never synthesises metric values for a missing week.
/// This is enforced by the type itself: WeekRecord::Gap has no field that could hold one.
#[test]
fn gaps_are_never_backfilled() {
    let mut coverage = Coverage {
        schema_version: 1,
        weeks: vec![WeekRecord::Run {
            iso_week: "2026-W35".to_string(),
            run_ids: vec!["run-w35".to_string()],
        }],
    };
    let new_entry = StageMetrics {
        iso_week: "2026-W37".to_string(),
        ..base_entry()
    };

    record_gaps(&mut coverage, &new_entry);

    for week in &coverage.weeks[1..] {
        match week {
            WeekRecord::Gap { reason, .. } => assert_eq!(*reason, GapReason::NoRunRecorded),
            WeekRecord::Run { .. } => panic!("record_gaps must never write a Run entry"),
        }
    }
}

/// A run refused on a precondition violation produces a Gap entry with reason naming the failed
/// check, per D-08.
#[test]
fn refused_run_records_gap() {
    let mut coverage = Coverage::new();

    record_gap(
        &mut coverage,
        "2026-W36",
        GapReason::RefusedOnPrecondition {
            check: "governor-is-performance-on-all-cpus".to_string(),
        },
    );

    assert_eq!(coverage.weeks.len(), 1);
    match &coverage.weeks[0] {
        WeekRecord::Gap { iso_week, reason } => {
            assert_eq!(iso_week, "2026-W36");
            match reason {
                GapReason::RefusedOnPrecondition { check } => {
                    assert_eq!(check, "governor-is-performance-on-all-cpus");
                }
                other => panic!("expected RefusedOnPrecondition, got {other:?}"),
            }
        }
        other => panic!("expected a Gap entry, got {other:?}"),
    }
}
