use illuminate_trail::normalize::topic_slug;
use illuminate_trail::record::{Message, MessageRole};

#[test]
fn topic_slug_uses_first_user_message() {
    let messages = vec![
        Message {
            role: MessageRole::System,
            timestamp: chrono::Utc::now(),
            text: "ignore me".into(),
        },
        Message {
            role: MessageRole::User,
            timestamp: chrono::Utc::now(),
            text: "Add Redis caching to the txn lookup".into(),
        },
    ];
    let slug = topic_slug(&messages);
    assert!(slug.contains("redis"));
    assert!(slug.contains("caching"));
}

#[test]
fn topic_slug_handles_empty_messages() {
    assert_eq!(topic_slug(&[]), "session");
}
