//! Tests for `fold_rationale` — merging an optional rationale into the audited
//! plan text. Shared by the CLI `audit` command and the MCP `illuminate_audit`
//! tool so a caller-supplied rationale is actually considered by the auditor.

use illuminate_audit::fold_rationale;

#[test]
fn fold_rationale_appends_when_present() {
    let folded = fold_rationale("add a caching layer", Some("because reads are hot"));
    assert!(folded.contains("add a caching layer"));
    assert!(folded.contains("because reads are hot"));
}

#[test]
fn fold_rationale_is_noop_when_absent() {
    assert_eq!(fold_rationale("do the thing", None), "do the thing");
}

#[test]
fn fold_rationale_is_noop_when_blank() {
    assert_eq!(fold_rationale("do the thing", Some("   ")), "do the thing");
}
