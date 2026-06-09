//! Black-box tests for the `wiki serve` dashboard.
//!
//! These tests exercise the pure `route()` function rather than spinning up a
//! `tiny_http` server — far faster, no port races, no thread cleanup. The
//! `serve()` loop is a thin shim over `route()`, so coverage here is exhaustive
//! for routing, rendering and audit-playground behaviour.

use illuminate_wiki::serve::{RouteCtx, route};
use std::fs;
use std::path::Path;

fn write_decision(root: &Path, id: &str, title: &str, status: &str) {
    let dir = root.join("decisions");
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        "---\nid: {id}\ntitle: {title}\ntype: decision\nstatus: {status}\ncreated: 2025-01-01T00:00:00Z\nupdated: 2025-01-01T00:00:00Z\n---\n\n## Decision\nbody for {id} {title}\n\n## Context\ncontext\n\n## Consequences\nc\n"
    );
    fs::write(dir.join(format!("{id}.md")), body).unwrap();
}

fn write_pattern(root: &Path, id: &str, title: &str) {
    let dir = root.join("patterns");
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        "---\nid: {id}\ntitle: {title}\ntype: pattern\nstatus: active\ncreated: 2025-01-01T00:00:00Z\nupdated: 2025-01-01T00:00:00Z\n---\n\n## Problem\np\n\n## Solution\ns\n\n## Trade-offs\nt\n"
    );
    fs::write(dir.join(format!("{id}.md")), body).unwrap();
}

fn write_failure(root: &Path, id: &str, severity: &str) {
    let dir = root.join("failures");
    fs::create_dir_all(&dir).unwrap();
    let body = format!(
        "---\nid: {id}\ntitle: failure {id}\ntype: failure\nstatus: active\nseverity: {severity}\ncreated: 2025-01-01T00:00:00Z\nupdated: 2025-01-01T00:00:00Z\n---\n\n## Problem\np\n\n## Cause\nc\n\n## Fix\nf\n\n## Lesson\nl\n"
    );
    fs::write(dir.join(format!("{id}.md")), body).unwrap();
}

fn ctx_for<'a>(root: &'a Path) -> RouteCtx<'a> {
    RouteCtx {
        root,
        project_name: Some("testproj"),
        auditor: None,
        tokens: None,
        graph: None,
    }
}

#[test]
fn home_page_renders_stats() {
    let tmp = tempfile::tempdir().unwrap();
    write_decision(tmp.path(), "dec-1", "first decision", "active");
    write_decision(tmp.path(), "dec-2", "second", "active");
    write_pattern(tmp.path(), "pat-1", "first pattern");

    let resp = route(&ctx_for(tmp.path()), "GET", "/", "");
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.starts_with("text/html"));
    // Stat counts are inlined into the dashboard cards.
    assert!(resp.body.contains("2"), "expected '2' in {}", resp.body);
    assert!(resp.body.contains("decisions"));
    assert!(resp.body.contains("patterns"));
    assert!(resp.body.contains("testproj"));
}

#[test]
fn decisions_list_filters_by_status() {
    let tmp = tempfile::tempdir().unwrap();
    write_decision(tmp.path(), "dec-a", "alpha decision", "active");
    write_decision(tmp.path(), "dec-b", "beta decision", "active");
    write_decision(tmp.path(), "dec-c", "gamma decision", "superseded");

    // No filter: all three present.
    let all = route(&ctx_for(tmp.path()), "GET", "/decisions", "");
    assert!(all.body.contains("alpha decision"));
    assert!(all.body.contains("beta decision"));
    assert!(all.body.contains("gamma decision"));

    // status=active filters to two.
    let active = route(&ctx_for(tmp.path()), "GET", "/decisions?status=active", "");
    assert!(active.body.contains("alpha decision"));
    assert!(active.body.contains("beta decision"));
    assert!(!active.body.contains("gamma decision"));

    // status=superseded filters to one.
    let sup = route(
        &ctx_for(tmp.path()),
        "GET",
        "/decisions?status=superseded",
        "",
    );
    assert!(!sup.body.contains("alpha decision"));
    assert!(sup.body.contains("gamma decision"));
}

#[test]
fn audit_playground_get_renders_form() {
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/audit", "");
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("<textarea"));
    assert!(resp.body.contains("name=\"plan\""));
    assert!(resp.body.contains("Run audit") || resp.body.contains("run audit"));
}

#[test]
fn audit_playground_post_returns_response_page() {
    let tmp = tempfile::tempdir().unwrap();
    // Inject a stub auditor that returns a violation when "Redis" is in the plan.
    let auditor = |plan: &str| -> serde_json::Value {
        let status = if plan.to_lowercase().contains("redis") {
            "violation"
        } else {
            "pass"
        };
        serde_json::json!({
            "status": status,
            "violations": [],
            "policy_violations": if status == "violation" {
                serde_json::json!([{
                    "policy_name": "no-redis",
                    "expected": null,
                    "found": "Redis",
                    "reason": "Redis not allowed",
                    "severity": "error",
                    "decision_ref": null,
                    "evidence": "plan contains 'Redis'",
                    "confidence": 1.0
                }])
            } else { serde_json::json!([]) },
            "reflexions": [],
            "impact": {
                "seed_symbols": [],
                "defined_symbols": [],
                "impacted_symbols": [],
                "truncated": false
            },
            "relevant_decisions": [],
            "trace_id": "test-trace",
            "policies_applied": ["no-redis"],
            "wiki_url": null,
        })
    };
    let ctx = RouteCtx {
        root: tmp.path(),
        project_name: Some("testproj"),
        auditor: Some(&auditor),
        tokens: None,
        graph: None,
    };

    let resp = route(&ctx, "POST", "/audit", "plan=add+Redis+caching");
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.starts_with("text/html"));
    let lc = resp.body.to_lowercase();
    assert!(
        lc.contains("violation"),
        "body missing 'violation': {}",
        resp.body
    );
    assert!(lc.contains("redis"), "body missing 'redis': {}", resp.body);
}

#[test]
fn api_stats_returns_json() {
    let tmp = tempfile::tempdir().unwrap();
    write_decision(tmp.path(), "dec-1", "alpha", "active");
    write_pattern(tmp.path(), "pat-1", "p1");
    write_failure(tmp.path(), "fail-1", "high");

    let resp = route(&ctx_for(tmp.path()), "GET", "/api/stats", "");
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.starts_with("application/json"));
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["decisions"], 1);
    assert_eq!(v["patterns"], 1);
    assert_eq!(v["failures"], 1);
    assert_eq!(v["modules"], 0);
}

#[test]
fn api_audit_post_returns_audit_result_json() {
    let tmp = tempfile::tempdir().unwrap();
    let auditor = |plan: &str| -> serde_json::Value {
        let status = if plan.contains("Redis") {
            "violation"
        } else {
            "pass"
        };
        serde_json::json!({
            "status": status,
            "trace_id": "test",
        })
    };
    let ctx = RouteCtx {
        root: tmp.path(),
        project_name: None,
        auditor: Some(&auditor),
        tokens: None,
        graph: None,
    };

    let resp = route(&ctx, "POST", "/api/audit", r#"{"plan":"add Redis"}"#);
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.starts_with("application/json"));
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["status"], "violation");
}

#[test]
fn back_compat_decision_url_still_renders() {
    let tmp = tempfile::tempdir().unwrap();
    write_decision(tmp.path(), "dec-x", "back compat title", "active");
    let resp = route(&ctx_for(tmp.path()), "GET", "/decisions/dec-x", "");
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("back compat title"));
}

#[test]
fn search_returns_matches_across_pages() {
    let tmp = tempfile::tempdir().unwrap();
    write_decision(tmp.path(), "dec-redis", "redis policy", "active");
    write_pattern(tmp.path(), "pat-cache", "cache pattern");

    let resp = route(&ctx_for(tmp.path()), "GET", "/search?q=redis", "");
    assert_eq!(resp.status, 200);
    let lc = resp.body.to_lowercase();
    assert!(lc.contains("redis"));
    assert!(resp.body.contains("redis policy"));
}

#[test]
fn unknown_route_404s() {
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/no/such/path/here", "");
    assert_eq!(resp.status, 404);
}

// ── /new — quick-add form ──────────────────────────────────────────────

#[test]
fn new_form_get_renders_with_decision_default() {
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/new", "");
    assert_eq!(resp.status, 200);
    assert!(resp.body.contains("<form"));
    assert!(resp.body.contains("name=\"title\""));
    assert!(resp.body.contains("name=\"body\""));
    assert!(resp.body.contains("name=\"type\""));
    // Decision radio is pre-checked when no ?type= is supplied.
    assert!(
        resp.body
            .contains(r#"<input type="radio" name="type" value="decision" checked"#)
    );
}

#[test]
fn new_form_get_with_type_query_pre_selects_radio() {
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/new?type=pattern", "");
    assert_eq!(resp.status, 200);
    assert!(
        resp.body
            .contains(r#"<input type="radio" name="type" value="pattern" checked"#)
    );
}

#[test]
fn new_post_writes_decision_page_and_redirects() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "type=decision&title=No+Redis+for+caching&tags=caching%2Cinfra&body=%23%23+Decision%0Awe+do+not+use+Redis";
    let resp = route(&ctx_for(tmp.path()), "POST", "/new", body);
    assert_eq!(resp.status, 303);
    let written = tmp.path().join("decisions/dec-no-redis-for-caching.md");
    assert!(
        written.exists(),
        "expected new page at {}",
        written.display()
    );
    let content = std::fs::read_to_string(&written).unwrap();
    assert!(content.contains("id: dec-no-redis-for-caching"));
    assert!(content.contains("title: No Redis for caching"));
    assert!(content.contains("page_type: decision"));
    assert!(content.contains("status: active"));
    assert!(content.contains("\"caching\""));
    assert!(content.contains("\"infra\""));
    assert!(content.contains("## Decision"));
    assert!(content.contains("we do not use Redis"));
}

#[test]
fn new_post_rejects_empty_title() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "type=decision&title=&tags=&body=some+content";
    let resp = route(&ctx_for(tmp.path()), "POST", "/new", body);
    assert_eq!(resp.status, 400);
    assert!(resp.body.to_lowercase().contains("title is required"));
    // Form should be re-rendered with the body content preserved.
    assert!(resp.body.contains("some content"));
}

#[test]
fn new_post_rejects_empty_body() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "type=decision&title=Test&tags=&body=";
    let resp = route(&ctx_for(tmp.path()), "POST", "/new", body);
    assert_eq!(resp.status, 400);
    assert!(resp.body.to_lowercase().contains("body is required"));
}

#[test]
fn new_post_409s_when_page_already_exists() {
    let tmp = tempfile::tempdir().unwrap();
    write_decision(tmp.path(), "dec-already-exists", "Already Exists", "active");
    let body = "type=decision&title=Already+Exists&tags=&body=duplicate";
    let resp = route(&ctx_for(tmp.path()), "POST", "/new", body);
    assert_eq!(resp.status, 409);
    assert!(resp.body.to_lowercase().contains("already exists"));
}

#[test]
fn new_post_writes_pattern_page_in_correct_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "type=pattern&title=LRU+Cache&tags=&body=%23%23+Pattern%0Ause+LRU";
    let resp = route(&ctx_for(tmp.path()), "POST", "/new", body);
    assert_eq!(resp.status, 303);
    assert!(tmp.path().join("patterns/pat-lru-cache.md").exists());
    assert!(!tmp.path().join("decisions/pat-lru-cache.md").exists());
}

#[test]
fn new_post_writes_failure_and_module_pages() {
    let tmp = tempfile::tempdir().unwrap();

    let r1 = route(
        &ctx_for(tmp.path()),
        "POST",
        "/new",
        "type=failure&title=Cache+stampede&tags=&body=%23%23+Root+Cause%0Ano+jitter",
    );
    assert_eq!(r1.status, 303);
    assert!(tmp.path().join("failures/fail-cache-stampede.md").exists());

    let r2 = route(
        &ctx_for(tmp.path()),
        "POST",
        "/new",
        "type=module&title=Payments&tags=&body=%23%23+Module%0Apayments+service",
    );
    assert_eq!(r2.status, 303);
    assert!(tmp.path().join("modules/mod-payments.md").exists());
}

#[test]
fn topnav_includes_new_link() {
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/", "");
    assert!(
        resp.body.contains("href=\"/new\""),
        "expected '+ new' link in topnav"
    );
}
