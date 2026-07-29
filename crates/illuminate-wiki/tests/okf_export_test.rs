//! Exporting the wiki as a conformant OKF v0.2 bundle.
//!
//! The mapping is documented in `docs/COMPANY_BRAIN.md` §6. What these tests
//! guard is the part that is easy to get wrong: OKF requires unknown keys to
//! survive a round trip, so illuminate-specific fields (`id`, `confidence`,
//! `modules`) must be *emitted*, not dropped, even though OKF has no slot for
//! them.

use illuminate_wiki::okf_export::export_bundle;
use illuminate_wiki::page::{WikiPage, parse_page};

const PRODUCER: &str = "illuminate/0.31.0";

fn page(front: &str, body: &str) -> WikiPage {
    parse_page(&format!("---\n{front}---\n\n{body}")).expect("test page parses")
}

fn decision(front_extra: &str) -> WikiPage {
    page(
        &format!(
            "id: dec-2026-07-no-redis\n\
             title: No Redis\n\
             type: decision\n\
             status: active\n\
             created: 2026-07-01T00:00:00Z\n\
             updated: 2026-07-02T00:00:00Z\n\
             {front_extra}"
        ),
        "## Decision\n\nUse an in-memory LRU.\n",
    )
}

/// Find one emitted file by its bundle-relative path.
fn file<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map(|(_, c)| c.as_str())
        .unwrap_or_else(|| {
            panic!(
                "no file at {path}; got {:?}",
                files.iter().map(|(p, _)| p).collect::<Vec<_>>()
            )
        })
}

// ------------------------------------------------------------- layout ---

#[test]
fn pages_land_under_their_category_directory() {
    let out = export_bundle(&[decision("")], PRODUCER);
    file(&out.files, "decisions/dec-2026-07-no-redis.md");
}

#[test]
fn the_bundle_declares_its_okf_version_in_a_reserved_index() {
    let out = export_bundle(&[decision("")], PRODUCER);
    let index = file(&out.files, "index.md");
    assert!(index.contains("okf_version: \"0.2\""), "got:\n{index}");
    assert!(index.contains("dec-2026-07-no-redis"), "got:\n{index}");
}

#[test]
fn export_is_byte_stable_for_identical_input() {
    // Determinism is a standing constraint: a re-export that reshuffles output
    // makes every sync a spurious diff.
    let a = export_bundle(&[decision("")], PRODUCER);
    let b = export_bundle(&[decision("")], PRODUCER);
    assert_eq!(a.files, b.files);
}

// ------------------------------------------------------- field mapping ---

#[test]
fn illuminate_page_types_become_okf_type_values() {
    for (page_type, expected, dir) in [
        ("decision", "Decision", "decisions"),
        ("pattern", "Pattern", "patterns"),
        ("failure", "Failure", "failures"),
        ("module", "Module", "modules"),
    ] {
        let p = page(
            &format!(
                "id: dec-x\ntitle: T\ntype: {page_type}\nstatus: active\n\
                 created: 2026-07-01T00:00:00Z\nupdated: 2026-07-01T00:00:00Z\n"
            ),
            "body\n",
        );
        let out = export_bundle(&[p], PRODUCER);
        let contents = file(&out.files, &format!("{dir}/dec-x.md"));
        assert!(
            contents.contains(&format!("type: {expected}")),
            "{page_type} should map to {expected}, got:\n{contents}"
        );
    }
}

#[test]
fn status_maps_onto_the_okf_lifecycle_vocabulary() {
    for (illuminate_status, okf_status) in [
        ("active", "stable"),
        ("superseded", "deprecated"),
        ("retired", "deprecated"),
    ] {
        let p = page(
            &format!(
                "id: dec-x\ntitle: T\ntype: decision\nstatus: {illuminate_status}\n\
                 created: 2026-07-01T00:00:00Z\nupdated: 2026-07-01T00:00:00Z\n"
            ),
            "body\n",
        );
        let out = export_bundle(&[p], PRODUCER);
        let contents = file(&out.files, "decisions/dec-x.md");
        assert!(
            contents.contains(&format!("status: {okf_status}")),
            "{illuminate_status} should map to {okf_status}, got:\n{contents}"
        );
    }
}

#[test]
fn generated_records_the_producer_and_the_last_update() {
    let out = export_bundle(&[decision("")], PRODUCER);
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(doc.contains("generated:"), "got:\n{doc}");
    assert!(doc.contains(&format!("by: {PRODUCER}")), "got:\n{doc}");
    assert!(doc.contains("2026-07-02T00:00:00Z"), "got:\n{doc}");
}

#[test]
fn tags_carry_over() {
    let out = export_bundle(&[decision("tags: [caching, infra]\n")], PRODUCER);
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(doc.contains("caching"), "got:\n{doc}");
    assert!(doc.contains("infra"), "got:\n{doc}");
}

#[test]
fn trust_fields_pass_through_unchanged() {
    // `verified` / `stale_after` are already OKF-shaped in illuminate, so the
    // exporter must not rewrite them.
    let out = export_bundle(
        &[decision(
            "verified:\n  - by: human:priya\n    at: 2026-07-03T00:00:00Z\nstale_after: 2026-12-31\n",
        )],
        PRODUCER,
    );
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(doc.contains("human:priya"), "got:\n{doc}");
    assert!(doc.contains("stale_after: 2026-12-31"), "got:\n{doc}");
}

#[test]
fn sources_map_ref_onto_resource_and_keep_kind() {
    let out = export_bundle(
        &[decision(
            "sources:\n  - kind: pr\n    ref: github.com/acme/p/pull/8\n",
        )],
        PRODUCER,
    );
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(
        doc.contains("resource: github.com/acme/p/pull/8"),
        "ref must become resource, got:\n{doc}"
    );
    assert!(
        doc.contains("kind: pr"),
        "kind survives as an extension key, got:\n{doc}"
    );
}

#[test]
fn illuminate_only_fields_survive_as_extension_keys() {
    // OKF §9: producers may add arbitrary keys and consumers must preserve
    // them. Dropping `id` here would make the export lossy and break re-import.
    let out = export_bundle(
        &[decision("confidence: 0.92\nmodules: [payments]\n")],
        PRODUCER,
    );
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(doc.contains("id: dec-2026-07-no-redis"), "got:\n{doc}");
    assert!(doc.contains("confidence: 0.92"), "got:\n{doc}");
    assert!(doc.contains("payments"), "got:\n{doc}");
}

// --------------------------------------------------------------- links ---

#[test]
fn related_ids_become_bundle_relative_links() {
    // OKF expresses relationships as markdown links in the body, not as
    // frontmatter id references.
    let other = page(
        "id: dec-2026-06-eks\ntitle: EKS\ntype: decision\nstatus: active\n\
         created: 2026-06-01T00:00:00Z\nupdated: 2026-06-01T00:00:00Z\n",
        "## Decision\n\nx\n",
    );
    let out = export_bundle(&[decision("related: [dec-2026-06-eks]\n"), other], PRODUCER);
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(
        doc.contains("(/decisions/dec-2026-06-eks.md)"),
        "expected a bundle-relative link, got:\n{doc}"
    );
}

#[test]
fn supersession_links_are_emitted_too() {
    let other = page(
        "id: dec-2026-06-eks\ntitle: EKS\ntype: decision\nstatus: active\n\
         created: 2026-06-01T00:00:00Z\nupdated: 2026-06-01T00:00:00Z\n",
        "## Decision\n\nx\n",
    );
    let out = export_bundle(
        &[decision("supersedes: [dec-2026-06-eks]\n"), other],
        PRODUCER,
    );
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(doc.contains("/decisions/dec-2026-06-eks.md"), "got:\n{doc}");
}

#[test]
fn an_unresolvable_related_id_does_not_break_the_export() {
    // OKF §9: consumers must tolerate broken cross-links, so producing one for
    // a dangling reference is legal — dropping the relationship silently is
    // worse than emitting a link that does not resolve yet.
    let out = export_bundle(&[decision("related: [dec-does-not-exist]\n")], PRODUCER);
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(doc.contains("dec-does-not-exist"), "got:\n{doc}");
}

#[test]
fn a_page_with_no_relationships_gains_no_related_section() {
    let out = export_bundle(&[decision("")], PRODUCER);
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(
        !doc.contains("## Related"),
        "empty relationship set should not add a section, got:\n{doc}"
    );
}

#[test]
fn the_original_body_is_preserved() {
    let out = export_bundle(&[decision("")], PRODUCER);
    let doc = file(&out.files, "decisions/dec-2026-07-no-redis.md");
    assert!(doc.contains("Use an in-memory LRU."), "got:\n{doc}");
}

// ---------------------------------------------------------- conformance ---

#[test]
fn every_emitted_concept_has_a_non_empty_type() {
    // OKF §9.1–9.2 conformance: every non-reserved .md must have parseable
    // frontmatter with a non-empty `type`.
    let out = export_bundle(&[decision(""), decision("")], PRODUCER);
    for (path, contents) in &out.files {
        if path == "index.md" {
            continue;
        }
        assert!(contents.starts_with("---\n"), "{path} lacks frontmatter");
        let front = contents
            .split("\n---")
            .next()
            .expect("frontmatter block")
            .to_string();
        assert!(
            front
                .lines()
                .any(|l| l.starts_with("type: ") && l.len() > 6),
            "{path} has no non-empty type:\n{front}"
        );
    }
}
