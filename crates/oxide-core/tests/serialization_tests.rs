//! Serialization roundtrip tests for oxide-core types.

use chrono::Utc;
use oxide_core::events::*;
use oxide_core::ids::*;
use oxide_core::pipeline::*;
use oxide_core::run::*;
use std::collections::HashMap;

#[test]
fn test_run_queued_payload_roundtrip() {
    let payload = RunQueuedPayload {
        run_id: RunId::new(),
        pipeline_id: PipelineId::new(),
        pipeline_name: "test-pipeline".to_string(),
        run_number: 42,
        trigger: TriggerType::Push,
        git_ref: Some("refs/heads/main".to_string()),
        git_sha: Some("abc123".to_string()),
        queued_at: Utc::now(),
        queued_by: Some("user@example.com".to_string()),
        license_id: None,
    };

    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: RunQueuedPayload = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(payload.run_id, parsed.run_id);
    assert_eq!(payload.pipeline_name, parsed.pipeline_name);
    assert_eq!(payload.run_number, parsed.run_number);
}

#[test]
fn test_run_completed_payload_roundtrip() {
    let payload = RunCompletedPayload {
        run_id: RunId::new(),
        pipeline_id: PipelineId::new(),
        pipeline_name: "test-pipeline".to_string(),
        run_number: 1,
        status: RunStatus::Success,
        duration_ms: 12345,
        stages_passed: 3,
        stages_failed: 0,
        completed_at: Utc::now(),
        billable_minutes: Some(0.21),
    };

    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: RunCompletedPayload = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(payload.status, parsed.status);
    assert_eq!(payload.duration_ms, parsed.duration_ms);
}

#[test]
fn test_stage_started_payload_roundtrip() {
    let payload = StageStartedPayload {
        run_id: RunId::new(),
        stage_name: "build".to_string(),
        stage_index: 0,
        step_count: 5,
        started_at: Utc::now(),
    };

    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: StageStartedPayload = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(payload.stage_name, parsed.stage_name);
    assert_eq!(payload.step_count, parsed.step_count);
}

#[test]
fn test_step_completed_payload_roundtrip() {
    let payload = StepCompletedPayload {
        run_id: RunId::new(),
        stage_name: "build".to_string(),
        step_id: "step-1".to_string(),
        step_name: "compile".to_string(),
        plugin: None,
        status: StepStatus::Success,
        exit_code: 0,
        duration_ms: 5000,
        completed_at: Utc::now(),
    };

    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: StepCompletedPayload = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(payload.step_name, parsed.step_name);
    assert_eq!(payload.exit_code, parsed.exit_code);
}

#[test]
fn test_agent_registered_payload_roundtrip() {
    let payload = AgentRegisteredPayload {
        agent_id: AgentId::new(),
        name: "agent-1".to_string(),
        labels: vec!["linux".to_string(), "docker".to_string()],
        version: Some("1.0.0".to_string()),
        registered_at: Utc::now(),
    };

    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: AgentRegisteredPayload = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(payload.name, parsed.name);
    assert_eq!(payload.labels, parsed.labels);
}

#[test]
fn test_cache_hit_payload_roundtrip() {
    let payload = CacheHitPayload {
        run_id: RunId::new(),
        step_id: Some("step-1".to_string()),
        cache_key: "cargo-deps-abc123".to_string(),
        cache_id: CacheEntryId::new(),
        size_bytes: 1024 * 1024 * 50,
        restore_duration_ms: 2500,
        paths: vec!["target/".to_string(), ".cargo/".to_string()],
        restored_at: Utc::now(),
    };

    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: CacheHitPayload = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(payload.cache_key, parsed.cache_key);
    assert_eq!(payload.size_bytes, parsed.size_bytes);
}

#[test]
fn test_pipeline_definition_roundtrip() {
    let definition = PipelineDefinition {
        version: "1".to_string(),
        name: "test-pipeline".to_string(),
        description: Some("A test pipeline".to_string()),
        triggers: vec![TriggerConfig::Push {
            push: Some(TriggerFilter {
                branches: vec!["main".to_string()],
                paths: vec![],
                paths_ignore: vec![],
                tags: vec![],
            }),
        }],
        variables: HashMap::new(),
        stages: vec![],
        cache: None,
        artifacts: None,
        timeout_minutes: 60,
        concurrency: None,
    };

    let json = serde_json::to_string(&definition).expect("serialize");
    let parsed: PipelineDefinition = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(definition.name, parsed.name);
    assert_eq!(definition.version, parsed.version);
}

#[test]
fn test_run_status_serialization() {
    assert_eq!(
        serde_json::to_string(&RunStatus::Queued).unwrap(),
        "\"queued\""
    );
    assert_eq!(
        serde_json::to_string(&RunStatus::Running).unwrap(),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&RunStatus::Success).unwrap(),
        "\"success\""
    );
    assert_eq!(
        serde_json::to_string(&RunStatus::Failure).unwrap(),
        "\"failure\""
    );
}

#[test]
fn test_trigger_type_serialization() {
    assert_eq!(
        serde_json::to_string(&TriggerType::Push).unwrap(),
        "\"push\""
    );
    assert_eq!(
        serde_json::to_string(&TriggerType::PullRequest).unwrap(),
        "\"pull_request\""
    );
    assert_eq!(
        serde_json::to_string(&TriggerType::Cron).unwrap(),
        "\"cron\""
    );
    assert_eq!(
        serde_json::to_string(&TriggerType::Manual).unwrap(),
        "\"manual\""
    );
}

#[test]
fn test_event_enum_roundtrip() {
    let event = Event::RunQueued(RunQueuedPayload {
        run_id: RunId::new(),
        pipeline_id: PipelineId::new(),
        pipeline_name: "test".to_string(),
        run_number: 1,
        trigger: TriggerType::Manual,
        git_ref: None,
        git_sha: None,
        queued_at: Utc::now(),
        queued_by: None,
        license_id: None,
    });

    let json = serde_json::to_string(&event).expect("serialize");
    let parsed: Event = serde_json::from_str(&json).expect("deserialize");

    match parsed {
        Event::RunQueued(p) => assert_eq!(p.pipeline_name, "test"),
        _ => panic!("Wrong event type"),
    }
}
