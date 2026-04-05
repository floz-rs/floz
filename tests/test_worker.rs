use floz::worker::TaskMessage;
use floz::worker::retry::exponential_backoff;
use chrono::Utc;
use serde_json::json;

#[test]
fn test_task_message_creation() {
    let args = json!({ "user_id": 123 });
    let msg = TaskMessage::new("test_task", "high_priority", args.clone(), 3);

    assert_eq!(msg.name, "test_task");
    assert_eq!(msg.queue, "high_priority");
    assert_eq!(msg.args, args);
    assert_eq!(msg.retries, 0);
    assert_eq!(msg.max_retries, 3);
    assert!(msg.id.starts_with("task_"));
    assert!(msg.eta.is_none());
}

#[test]
fn test_task_message_eta() {
    let args = json!([1, 2, 3]);
    let msg = TaskMessage::new("demo", "default", args, 1);
    let future_eta = Utc::now() + chrono::Duration::hours(1);
    
    let msg_delayed = msg.with_eta(future_eta);
    assert_eq!(msg_delayed.eta, Some(future_eta));
}

#[test]
fn test_task_message_serialization() {
    let args = json!(["param1"]);
    let msg = TaskMessage::new("my_task", "q1", args, 5);

    let serialized = serde_json::to_string(&msg).expect("Serialize failed");
    assert!(serialized.contains("\"name\":\"my_task\""));
    assert!(serialized.contains("\"queue\":\"q1\""));

    let deserialized: TaskMessage = serde_json::from_str(&serialized).expect("Deserialize failed");
    assert_eq!(msg.id, deserialized.id);
    assert_eq!(msg.name, deserialized.name);
}

#[test]
fn test_exponential_backoff_base() {
    let d1 = exponential_backoff(1);
    // Base is 2s, jitter is up to 0.4s -> 0..1 max jitter. 
    // Actual is 2s or 3s.
    assert!(d1.as_secs() >= 2 && d1.as_secs() <= 3);

    let d2 = exponential_backoff(5);
    // Base is 2^5 = 32s. Jitter is up to 6.4s (6s).
    // Actual is 32s..38s.
    assert!(d2.as_secs() >= 32 && d2.as_secs() <= 38);
}

#[test]
fn test_exponential_backoff_cap() {
    let cap_retry = exponential_backoff(20);
    let max_cap = exponential_backoff(14);
    
    // They both use exp = 14 so max base is 16384s.
    assert!(cap_retry.as_secs() >= 16384 && cap_retry.as_secs() <= 16384 + (16384.0 * 0.2) as u64);
    assert!(max_cap.as_secs() >= 16384 && max_cap.as_secs() <= 16384 + (16384.0 * 0.2) as u64);
}
