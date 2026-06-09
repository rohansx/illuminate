//! Black-box tests for the `GET /api/dashboard` JSON aggregate endpoint.
//!
//! The dashboard front-end (`illuminate-web/dashboard.html` + `illuminate-v4.js`)
//! fetches one envelope on load to replace its static mock numbers. These tests
//! exercise the pure `route()` function against a `tempfile::tempdir()` wiki
//! fixture — no TCP listener, no mocks — and assert the envelope's keys/shape
//! are stable so the JS contract can't silently drift.

use illuminate_wiki::serve::{RouteCtx, route};
use std::fs;
use std::path::Path;

fn write_page(root: &Path, dir: &str, kind: &str, id: &str, title: &str, updated: &str) {
    let d = root.join(dir);
    fs::create_dir_all(&d).unwrap();
    let body = format!(
        "---\nid: {id}\ntitle: {title}\ntype: {kind}\nstatus: active\ncreated: 2025-01-01T00:00:00Z\nupdated: {updated}\n---\n\n## Section\nbody for {id} {title}\n"
    );
    fs::write(d.join(format!("{id}.md")), body).unwrap();
}

fn ctx_for(root: &Path) -> RouteCtx<'_> {
    RouteCtx {
        root,
        project_name: Some("testproj"),
        auditor: None,
        tokens: None,
    }
}

#[test]
fn api_dashboard_returns_stable_envelope() {
    let tmp = tempfile::tempdir().unwrap();
    write_page(
        tmp.path(),
        "decisions",
        "decision",
        "dec-1",
        "first decision",
        "2025-03-01T00:00:00Z",
    );
    write_page(
        tmp.path(),
        "decisions",
        "decision",
        "dec-2",
        "second decision",
        "2025-04-01T00:00:00Z",
    );
    write_page(
        tmp.path(),
        "patterns",
        "pattern",
        "pat-1",
        "a pattern",
        "2025-02-01T00:00:00Z",
    );
    write_page(
        tmp.path(),
        "failures",
        "failure",
        "fail-1",
        "a failure",
        "2025-05-01T00:00:00Z",
    );
    write_page(
        tmp.path(),
        "modules",
        "module",
        "mod-1",
        "a module",
        "2025-01-15T00:00:00Z",
    );

    let resp = route(&ctx_for(tmp.path()), "GET", "/api/dashboard", "");
    assert_eq!(resp.status, 200);
    assert!(
        resp.content_type.starts_with("application/json"),
        "expected json content-type, got {}",
        resp.content_type
    );

    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();

    // ── top-level envelope keys ───────────────────────────────────────────
    assert!(v.get("project").is_some(), "missing `project`");
    assert!(v.get("generated_at").is_some(), "missing `generated_at`");
    assert!(v.get("stats").is_some(), "missing `stats`");
    assert!(v.get("recent_sessions").is_some(), "missing `recent_sessions`");
    assert!(v.get("recent_decisions").is_some(), "missing `recent_decisions`");
    assert!(v.get("recent_failures").is_some(), "missing `recent_failures`");
    assert!(v.get("audit_rows").is_some(), "missing `audit_rows`");

    assert_eq!(v["project"], "testproj");

    // ── stats sub-object: parity with /api/stats ──────────────────────────
    let stats = &v["stats"];
    assert_eq!(stats["decisions"], 2);
    assert_eq!(stats["patterns"], 1);
    assert_eq!(stats["failures"], 1);
    assert_eq!(stats["modules"], 1);
    assert_eq!(stats["total"], 5);
    // entity/edge counts present for the graph KPI tile.
    assert!(stats.get("entities").is_some(), "missing stats.entities");
    assert!(stats.get("edges").is_some(), "missing stats.edges");

    // ── recent_decisions: only Decision pages, newest first ───────────────
    let decisions = v["recent_decisions"].as_array().unwrap();
    assert_eq!(decisions.len(), 2);
    // dec-2 (Apr) is newer than dec-1 (Mar).
    assert_eq!(decisions[0]["id"], "dec-2");
    assert_eq!(decisions[1]["id"], "dec-1");
    assert!(decisions[0].get("title").is_some());
    assert!(decisions[0].get("status").is_some());
    assert!(decisions[0].get("updated").is_some());
    assert_eq!(decisions[0]["type"], "decisions");

    // ── recent_failures: only Failure pages ───────────────────────────────
    let failures = v["recent_failures"].as_array().unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0]["id"], "fail-1");
    assert_eq!(failures[0]["type"], "failures");

    // ── recent_sessions: most-recent pages across all types, newest first ─
    let sessions = v["recent_sessions"].as_array().unwrap();
    assert!(!sessions.is_empty());
    // newest overall is fail-1 (May).
    assert_eq!(sessions[0]["id"], "fail-1");
    assert!(sessions[0].get("ago").is_some(), "session row needs `ago`");

    // ── audit_rows: present, array, each row has id/title/type ────────────
    let audit_rows = v["audit_rows"].as_array().unwrap();
    for row in audit_rows {
        assert!(row.get("id").is_some());
        assert!(row.get("title").is_some());
        assert!(row.get("type").is_some());
    }
}

#[test]
fn api_dashboard_empty_wiki_has_empty_arrays_not_null() {
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/api/dashboard", "");
    assert_eq!(resp.status, 200);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();

    assert_eq!(v["stats"]["total"], 0);
    assert_eq!(v["stats"]["decisions"], 0);
    // Arrays must be present-and-empty, never null, so JS `.forEach` is safe.
    assert!(v["recent_sessions"].as_array().unwrap().is_empty());
    assert!(v["recent_decisions"].as_array().unwrap().is_empty());
    assert!(v["recent_failures"].as_array().unwrap().is_empty());
    assert!(v["audit_rows"].as_array().unwrap().is_empty());
}

#[test]
fn api_dashboard_project_defaults_when_unnamed() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = RouteCtx {
        root: tmp.path(),
        project_name: None,
        auditor: None,
        tokens: None,
    };
    let resp = route(&ctx, "GET", "/api/dashboard", "");
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["project"], "illuminate");
}

#[test]
fn api_dashboard_stats_match_api_stats_endpoint() {
    // The dashboard envelope's `stats` must agree with the standalone
    // /api/stats route so the two views never disagree.
    let tmp = tempfile::tempdir().unwrap();
    write_page(
        tmp.path(),
        "decisions",
        "decision",
        "dec-1",
        "d",
        "2025-03-01T00:00:00Z",
    );
    write_page(
        tmp.path(),
        "failures",
        "failure",
        "fail-1",
        "f",
        "2025-03-02T00:00:00Z",
    );

    let dash: serde_json::Value =
        serde_json::from_str(&route(&ctx_for(tmp.path()), "GET", "/api/dashboard", "").body)
            .unwrap();
    let stats: serde_json::Value =
        serde_json::from_str(&route(&ctx_for(tmp.path()), "GET", "/api/stats", "").body).unwrap();

    assert_eq!(dash["stats"]["decisions"], stats["decisions"]);
    assert_eq!(dash["stats"]["patterns"], stats["patterns"]);
    assert_eq!(dash["stats"]["failures"], stats["failures"]);
    assert_eq!(dash["stats"]["modules"], stats["modules"]);
    assert_eq!(dash["stats"]["total"], stats["total"]);
}

#[test]
fn api_dashboard_tokens_object_is_present_and_numeric() {
    // The savings tile fetches the same envelope; `tokens` must always be a
    // stable object whose six fields are numeric-typed (never null/string).
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/api/dashboard", "");
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();

    let tokens = v.get("tokens").expect("missing `tokens` object");
    assert!(tokens.is_object(), "`tokens` must be a JSON object");
    for key in [
        "input",
        "output",
        "cache_read",
        "cache_creation",
        "cache_saved_pct",
        "sessions",
    ] {
        let field = tokens.get(key).unwrap_or_else(|| panic!("missing tokens.{key}"));
        assert!(
            field.is_number(),
            "tokens.{key} must be numeric, got {field}"
        );
    }
}

#[test]
fn api_dashboard_tokens_zero_when_no_trail_source() {
    // No tokens closure wired in (the empty-wiki / no-trail case): every count
    // must be a numeric zero — never null — so the JS savings tile renders 0
    // rather than blanking out.
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/api/dashboard", "");
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();

    let tokens = &v["tokens"];
    assert_eq!(tokens["input"], 0);
    assert_eq!(tokens["output"], 0);
    assert_eq!(tokens["cache_read"], 0);
    assert_eq!(tokens["cache_creation"], 0);
    assert_eq!(tokens["sessions"], 0);
    // cache_saved_pct is a float zero, never null/NaN.
    assert_eq!(tokens["cache_saved_pct"].as_f64(), Some(0.0));
    assert!(!tokens["cache_saved_pct"].is_null());
}

#[test]
fn api_dashboard_tokens_reflect_wired_source() {
    // When a tokens closure is wired in (the CLI path that folds captured
    // trails via aggregate_tokens), its values flow through the envelope
    // verbatim.
    let tmp = tempfile::tempdir().unwrap();
    let tokens_fn = || -> serde_json::Value {
        serde_json::json!({
            "sessions": 3,
            "input": 1000,
            "output": 250,
            "cache_creation": 40,
            "cache_read": 500,
            "cache_saved_pct": 33.33,
        })
    };
    let ctx = RouteCtx {
        root: tmp.path(),
        project_name: Some("testproj"),
        auditor: None,
        tokens: Some(&tokens_fn),
    };
    let resp = route(&ctx, "GET", "/api/dashboard", "");
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();

    let tokens = &v["tokens"];
    assert_eq!(tokens["sessions"], 3);
    assert_eq!(tokens["input"], 1000);
    assert_eq!(tokens["output"], 250);
    assert_eq!(tokens["cache_creation"], 40);
    assert_eq!(tokens["cache_read"], 500);
    assert_eq!(tokens["cache_saved_pct"].as_f64(), Some(33.33));
}
