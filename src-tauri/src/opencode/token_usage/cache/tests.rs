//! Pure state-machine coverage for the process-wide token usage cache.
//!
//! Every case drives `Cache` directly with an explicit clock, so no network, worker
//! thread, or filesystem is involved.

use super::*;

use crate::error::ErrorCode;

const TTL: Duration = Duration::from_secs(5 * 60);

fn start() -> Instant {
    Instant::now()
}

fn target(revision: u64) -> target::Snapshot {
    // Detached snapshots never resolve a runtime, so `ensure_current` stays a no-op
    // unless production code starts trusting revisions it does not own.
    let mut snapshot = target::Snapshot::detached(target::Selection {
        url: Some(format!("http://127.0.0.1:{}", 5000 + revision)),
        password: None,
    });
    snapshot.revision = revision;
    snapshot
}

fn records(time: u64) -> Vec<TokenUsageRecord> {
    vec![TokenUsageRecord {
        time,
        input: time,
        output: 1,
    }]
}

fn error(message: &str) -> AppError {
    AppError::configuration(message)
}

/// One cold request followed by the completion the worker would report.
fn prime(cache: &mut Cache, revision: u64, now: Instant, time: u64) {
    cache.request(target(revision), false, now);
    assert_eq!(
        cache.begin().map(|snapshot| snapshot.revision),
        Some(revision)
    );
    cache.complete(revision, Ok(records(time)), now, time);
}

#[test]
fn first_request_is_stale_refreshing_and_starts_exactly_one_worker() {
    let now = start();
    let mut cache = Cache::new();
    let view = cache.snapshot(now);

    assert!(view.records.is_empty());
    assert_eq!(view.updated_at, None);
    assert!(!view.refreshing, "an unrequested cache never scans");
    assert!(view.stale);
    assert_eq!(view.error, None);
    assert_eq!(view.cache_scope, "process");

    cache.request(target(1), false, now);
    let view = cache.snapshot(now);

    assert!(view.refreshing);
    assert!(view.stale);
    assert_eq!(view.target_revision, 1);
    assert_eq!(cache.begin().map(|snapshot| snapshot.revision), Some(1));
    assert!(cache.begin().is_none(), "a single pending scan is queued");
}

#[test]
fn completed_scan_is_fresh_and_served_without_another_scan_inside_the_ttl() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 1, now, 10);

    let view = cache.snapshot(now + TTL - Duration::from_millis(1));

    assert_eq!(view.records, records(10));
    assert_eq!(view.updated_at, Some(10));
    assert!(!view.stale);
    assert!(!view.refreshing);
    assert_eq!(view.error, None);

    // Any request inside the TTL must not enqueue work, refresh flag or not.
    cache.request(target(1), false, now + TTL - Duration::from_millis(1));
    assert!(cache.pending.is_none());
    assert!(cache.begin().is_none());
}

#[test]
fn ttl_boundary_is_inclusive_and_expiry_keeps_the_last_good_records() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 1, now, 10);

    assert!(!cache.stale(now + TTL - Duration::from_millis(1)));
    assert!(cache.stale(now + TTL));

    cache.request(target(1), false, now + TTL);
    let view = cache.snapshot(now + TTL);

    assert!(view.stale);
    assert!(view.refreshing);
    assert_eq!(
        view.records,
        records(10),
        "stale data stays visible while refreshing"
    );
    assert_eq!(view.updated_at, Some(10));
    assert_eq!(cache.begin().map(|snapshot| snapshot.revision), Some(1));
}

#[test]
fn explicit_refresh_bypasses_the_cooldown_but_never_duplicates_in_flight_work() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 1, now, 10);

    // Fresh data plus an explicit refresh still scans.
    cache.request(target(1), true, now + Duration::from_secs(1));
    assert_eq!(cache.begin().map(|snapshot| snapshot.revision), Some(1));

    // A second refresh while the first is still active is dropped.
    cache.request(target(1), true, now + Duration::from_secs(2));
    assert!(cache.pending.is_none());
    assert!(cache.snapshot(now + Duration::from_secs(2)).refreshing);

    cache.complete(1, Ok(records(11)), now + Duration::from_secs(2), 11);
    assert_eq!(cache.snapshot(now).records, records(11));
    assert!(!cache.snapshot(now).refreshing);
}

#[test]
fn failed_scan_keeps_previous_records_and_reports_the_error() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 1, now, 10);
    let later = now + TTL;

    cache.request(target(1), false, later);
    assert!(cache.begin().is_some());
    cache.complete(1, Err(error("scan failed")), later, 0);

    let view = cache.snapshot(later);

    assert_eq!(view.error, Some(error("scan failed")));
    assert_eq!(
        view.error.as_ref().map(|error| error.code),
        Some(ErrorCode::ConfigurationFailed)
    );
    assert_eq!(
        view.records,
        records(10),
        "a failed refresh must not clear good data"
    );
    assert_eq!(view.updated_at, Some(10));
    assert!(!view.refreshing);
    assert!(view.stale, "the last success is still TTL-old");
}

#[test]
fn failed_first_scan_reports_the_error_without_fabricating_data() {
    let now = start();
    let mut cache = Cache::new();

    cache.request(target(1), false, now);
    assert!(cache.begin().is_some());
    cache.complete(1, Err(error("service offline")), now, 0);

    let view = cache.snapshot(now);

    assert!(view.records.is_empty());
    assert_eq!(view.updated_at, None);
    assert_eq!(view.error, Some(error("service offline")));
    assert!(view.stale);
    assert!(!view.refreshing);
}

#[test]
fn failed_attempts_cool_down_for_the_ttl_while_explicit_refresh_bypasses_it() {
    let now = start();
    let mut cache = Cache::new();

    cache.request(target(1), false, now);
    let _ = cache.begin();
    cache.complete(1, Err(error("first failure")), now, 0);

    // Automatic callers stay quiet during the cooldown even though the data is stale.
    cache.request(target(1), false, now + TTL - Duration::from_millis(1));
    assert!(cache.pending.is_none());
    assert_eq!(cache.snapshot(now).error, Some(error("first failure")));

    // Explicit retry always scans.
    cache.request(target(1), true, now + Duration::from_millis(1));
    assert_eq!(cache.begin().map(|snapshot| snapshot.revision), Some(1));

    // The cooldown restarts from the newest attempt.
    cache.complete(
        1,
        Err(error("second failure")),
        now + Duration::from_millis(1),
        0,
    );
    cache.request(target(1), false, now + TTL);
    assert!(cache.pending.is_none());
    cache.request(target(1), false, now + TTL + Duration::from_millis(1));
    assert!(cache.pending.is_some());
}

#[test]
fn success_clears_the_error_and_restarts_the_ttl() {
    let now = start();
    let mut cache = Cache::new();

    cache.request(target(1), false, now);
    let _ = cache.begin();
    cache.complete(1, Err(error("transient")), now, 0);
    assert!(cache.snapshot(now).error.is_some());

    cache.request(target(1), true, now + Duration::from_secs(1));
    let _ = cache.begin();
    cache.complete(1, Ok(records(20)), now + Duration::from_secs(1), 20);

    let view = cache.snapshot(now + TTL);

    assert_eq!(view.error, None);
    assert_eq!(view.records, records(20));
    assert!(
        !view.stale,
        "refresh time, not the first attempt, owns the TTL"
    );
}

#[test]
fn late_result_from_another_revision_cannot_overwrite_current_data() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 1, now, 10);

    // Revision 1 is still scanning when the target changes.
    cache.request(target(1), true, now + Duration::from_secs(1));
    assert_eq!(cache.begin().map(|snapshot| snapshot.revision), Some(1));
    cache.select(2);

    let view = cache.snapshot(now + Duration::from_secs(1));
    assert!(
        view.records.is_empty(),
        "selecting a new target drops the old view"
    );
    assert_eq!(view.target_revision, 2);
    assert!(!view.refreshing);

    cache.complete(1, Ok(records(99)), now + Duration::from_secs(2), 99);
    let view = cache.snapshot(now + Duration::from_secs(2));
    assert!(view.records.is_empty());
    assert_eq!(view.updated_at, None);
    assert_eq!(view.error, None);

    cache.complete(
        1,
        Err(error("old target failed")),
        now + Duration::from_secs(3),
        0,
    );
    assert_eq!(cache.snapshot(now).error, None);
}

#[test]
fn revision_change_clears_error_cooldown_and_pending_work() {
    let now = start();
    let mut cache = Cache::new();

    cache.request(target(1), false, now);
    let _ = cache.begin();
    cache.complete(1, Err(error("old target failed")), now, 0);
    cache.request(target(1), true, now);

    cache.request(target(2), false, now + Duration::from_secs(1));
    let view = cache.snapshot(now + Duration::from_secs(1));

    assert_eq!(view.target_revision, 2);
    assert_eq!(
        view.error, None,
        "errors belong to the target that produced them"
    );
    assert!(view.refreshing, "the new target starts its own cold scan");
    assert!(view.stale);
    assert!(view.records.is_empty());
}

#[test]
fn returning_to_an_earlier_revision_number_does_not_serve_its_old_records() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 1, now, 10);
    prime(&mut cache, 2, now, 20);
    assert_eq!(cache.snapshot(now).records, records(20));

    cache.request(target(1), false, now);
    cache.complete(1, Ok(records(30)), now, 30);

    assert_eq!(
        cache.snapshot(now).records,
        records(30),
        "only the latest revision completes"
    );
}

#[test]
fn refresh_attempts_are_coalesced_per_revision_and_released_after_completion() {
    let now = start();
    let mut cache = Cache::new();

    cache.request(target(7), false, now);
    cache.request(target(7), false, now);
    cache.request(target(7), true, now);
    assert_eq!(
        cache.pending.as_ref().map(|snapshot| snapshot.revision),
        Some(7)
    );
    assert!(cache.begin().is_some());
    assert!(cache.pending.is_none());

    // While active, requests are ignored; the worker exit path clears `active_revision`.
    cache.request(target(7), true, now);
    assert!(cache.pending.is_none());
    cache.active_revision = None;
    cache.request(target(7), true, now);
    assert!(cache.pending.is_some());
}

#[test]
fn target_revision_mismatch_reports_conflict_and_keeps_records() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 1, now, 10);

    cache.request(target(1), true, now + Duration::from_secs(1));
    let _ = cache.begin();
    cache.complete(
        1,
        Err(error("target changed; retry the operation")),
        now + Duration::from_secs(1),
        0,
    );

    let view = cache.snapshot(now + Duration::from_secs(1));

    assert_eq!(
        view.error,
        Some(error("target changed; retry the operation"))
    );
    assert_eq!(view.records, records(10));
    assert_eq!(view.target_revision, 1);
}

#[test]
fn snapshot_payload_serializes_the_frontend_contract() {
    let now = start();
    let mut cache = Cache::new();
    prime(&mut cache, 3, now, 42);

    let value = serde_json::to_value(cache.snapshot(now)).unwrap();

    assert_eq!(
        value,
        serde_json::json!({
            "records": [{ "time": 42, "input": 42, "output": 1 }],
            "updatedAt": 42,
            "refreshing": false,
            "stale": false,
            "error": null,
            "targetRevision": 3,
            "cacheScope": "process",
            "resultRevision": 2,
        })
    );
}
