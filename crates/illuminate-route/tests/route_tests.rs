//! Tests for illuminate-route: search routing and reading plans.

use illuminate_route::{route, ReadingPlan};

fn make_graph_with_episodes() -> illuminate::Graph {
    let graph = illuminate::Graph::in_memory().unwrap();

    // Add some decision episodes
    let ep1 = illuminate::Episode::builder("Chose Postgres over MongoDB for ACID compliance in billing")
        .source("git")
        .tag("database")
        .build();
    graph.add_episode(ep1).unwrap();

    let ep2 = illuminate::Episode::builder("Use Memcached for caching, not Redis, due to VPC overhead")
        .source("github-pr")
        .tag("caching")
        .build();
    graph.add_episode(ep2).unwrap();

    let ep3 = illuminate::Episode::builder("Auth module frozen for PCI compliance audit until April")
        .source("manual")
        .tag("security")
        .build();
    graph.add_episode(ep3).unwrap();

    graph
}

#[test]
fn route_returns_matching_decisions() {
    let graph = make_graph_with_episodes();
    let plan = route(&graph, None, "database", 10).unwrap();

    assert!(
        !plan.decisions.is_empty(),
        "should find decisions about database"
    );
}

#[test]
fn route_returns_empty_for_no_match() {
    let graph = make_graph_with_episodes();
    let plan = route(&graph, None, "xyznonexistent", 10).unwrap();

    assert!(
        plan.decisions.is_empty(),
        "should find no decisions for nonexistent topic"
    );
}

#[test]
fn route_respects_limit() {
    let graph = make_graph_with_episodes();
    let plan = route(&graph, None, "for", 1).unwrap();

    assert!(plan.decisions.len() <= 1, "should respect limit=1");
}

#[test]
fn route_estimates_tokens() {
    let graph = make_graph_with_episodes();
    let plan = route(&graph, None, "caching", 10).unwrap();

    assert!(plan.estimated_tokens > 0, "should estimate tokens");
}

#[test]
fn route_empty_graph() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let plan = route(&graph, None, "anything", 10).unwrap();

    assert!(plan.decisions.is_empty());
    assert!(plan.code_files.is_empty());
    assert_eq!(plan.estimated_tokens, 0);
}

#[test]
fn reading_plan_serialization() {
    let plan = ReadingPlan {
        decisions: vec![],
        code_files: vec![],
        estimated_tokens: 0,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let deserialized: ReadingPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.estimated_tokens, 0);
}
