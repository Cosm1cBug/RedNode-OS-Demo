// RedNode-OS — Autonomous Pipelines
//
// Cross-agent pipelines that chain multiple tools across agents.
// These are higher-order workflows that the Sentience Engine or
// scheduled triggers can invoke without human interaction.
//
// Pipelines:
//   1. Threat Response      — detect → analyze → block → audit → notify
//   2. Morning Briefing     — weather + cameras + email + calendar + health
//   3. Smart Notification   — classify → priority → batch/immediate → deliver
//   4. Predictive Maint     — collect metrics → trend → predict → alert
//   5. Presence Detection   — camera + network + phone → occupancy → automate
//   6. Email Triage         — fetch → classify → draft replies → digest
//   7. RSS Digest           — fetch feeds → deduplicate → summarize → deliver
//   8. Photo Ingest         — detect new → classify → tag → thumbnail → index

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A pipeline is a named sequence of steps, each targeting an agent+tool.
/// Steps can reference results of previous steps via `$step_N.field`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    pub description: String,
    pub trigger: PipelineTrigger,
    pub steps: Vec<PipelineStep>,
    pub on_failure: FailureAction,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineTrigger {
    /// Run on a cron schedule (e.g., "0 7 * * *" = daily 7 AM)
    Cron(String),
    /// Run when an event matching this pattern arrives on NATS
    Event { subject: String, filter: Option<String> },
    /// Run when a Sentience drive drops below threshold
    DriveBelow { drive: String, threshold: f32 },
    /// Run manually via API or intent
    Manual,
    /// Run on system boot (after self-heal completes)
    Boot,
    /// Run at fixed interval (seconds)
    Interval(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub id: String,
    pub tool: String,
    pub args: serde_json::Value,
    /// If true, pipeline continues even if this step fails
    pub continue_on_error: bool,
    /// Condition: only run if this expression is true (simple field checks)
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureAction {
    Abort,
    Continue,
    Retry { max_attempts: u32, delay_secs: u64 },
    Notify { message: String },
}

/// Pipeline execution result
#[derive(Debug, Serialize)]
pub struct PipelineResult {
    pub pipeline: String,
    pub success: bool,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub results: Vec<StepResult>,
    pub duration_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct StepResult {
    pub id: String,
    pub tool: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u128,
}

/// Registry of all built-in pipelines
pub fn builtin_pipelines() -> Vec<Pipeline> {
    vec![
        // ── 1. Threat Response Pipeline ──
        // Triggered by security events or drive drops.
        // Flow: detect IOCs → check against threat intel → block on pfSense → block on Pi-hole → audit → notify
        Pipeline {
            name: "threat_response".into(),
            description: "Auto-detect and block threats across the entire network stack".into(),
            trigger: PipelineTrigger::Event {
                subject: "rednode.security.threat".into(),
                filter: None,
            },
            steps: vec![
                PipelineStep {
                    id: "check_ioc".into(),
                    tool: "sec.ioc_check".into(),
                    args: serde_json::json!({"value": "$event.indicator"}),
                    continue_on_error: false,
                    condition: None,
                },
                PipelineStep {
                    id: "block_firewall".into(),
                    tool: "fw.block_ip".into(),
                    args: serde_json::json!({"ip": "$event.indicator", "reason": "auto-threat-response"}),
                    continue_on_error: true,
                    condition: Some("$check_ioc.match != null".into()),
                },
                PipelineStep {
                    id: "block_dns".into(),
                    tool: "pihole.add_block".into(),
                    args: serde_json::json!({"domain": "$event.indicator"}),
                    continue_on_error: true,
                    condition: Some("$event.type == 'domain'".into()),
                },
                PipelineStep {
                    id: "scan_network".into(),
                    tool: "net.scan".into(),
                    args: serde_json::json!({"target": "$event.indicator"}),
                    continue_on_error: true,
                    condition: None,
                },
                PipelineStep {
                    id: "notify".into(),
                    tool: "signal.send".into(),
                    args: serde_json::json!({"message": "🚨 Threat blocked: $event.indicator — source: $check_ioc.match.source"}),
                    continue_on_error: true,
                    condition: None,
                },
            ],
            on_failure: FailureAction::Notify { message: "Threat response pipeline failed — manual review needed".into() },
            enabled: true,
        },

        // ── 2. Morning Briefing Pipeline ──
        Pipeline {
            name: "morning_briefing".into(),
            description: "Daily morning brief: weather, cameras, email, calendar, health".into(),
            trigger: PipelineTrigger::Cron("30 7 * * *".into()), // 7:30 AM daily
            steps: vec![
                PipelineStep { id: "weather".into(), tool: "research.weather".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "news".into(), tool: "research.news".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "health".into(), tool: "service.status".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "cameras".into(), tool: "cam.events".into(), args: serde_json::json!({"period": "overnight"}), continue_on_error: true, condition: None },
                PipelineStep { id: "security".into(), tool: "sec.triage".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "dns".into(), tool: "pihole.stats".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "storage".into(), tool: "nas.health".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "email".into(), tool: "email.triage".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "calendar".into(), tool: "calendar.view".into(), args: serde_json::json!({"period": "today"}), continue_on_error: true, condition: None },
                PipelineStep { id: "tasks".into(), tool: "tasks.list".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "rss".into(), tool: "rss.digest".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "notify".into(), tool: "signal.send".into(), args: serde_json::json!({"message": "compile_briefing"}), continue_on_error: true, condition: None },
            ],
            on_failure: FailureAction::Continue,
            enabled: true,
        },

        // ── 3. Predictive Maintenance Pipeline ──
        Pipeline {
            name: "predictive_maintenance".into(),
            description: "Collect hardware metrics, detect trends, predict failures".into(),
            trigger: PipelineTrigger::Cron("0 3 * * *".into()), // 3 AM daily
            steps: vec![
                PipelineStep { id: "smart".into(), tool: "nas.smart".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "disks".into(), tool: "nas.disks".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "pools".into(), tool: "nas.pools".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "predict".into(), tool: "predict.maintenance".into(), args: serde_json::json!({"data": "$smart.output"}), continue_on_error: false, condition: None },
                PipelineStep { id: "alert".into(), tool: "signal.send".into(), args: serde_json::json!({"message": "$predict.alerts"}), continue_on_error: true, condition: Some("$predict.has_alerts == true".into()) },
            ],
            on_failure: FailureAction::Continue,
            enabled: true,
        },

        // ── 4. Presence Detection Pipeline ──
        Pipeline {
            name: "presence_detection".into(),
            description: "Determine occupancy from cameras + network + phone presence".into(),
            trigger: PipelineTrigger::Interval(300), // every 5 minutes
            steps: vec![
                PipelineStep { id: "camera_people".into(), tool: "cam.person_detect".into(), args: serde_json::json!({"period": "5m"}), continue_on_error: true, condition: None },
                PipelineStep { id: "network_devices".into(), tool: "net.devices".into(), args: serde_json::json!({}), continue_on_error: true, condition: None },
                PipelineStep { id: "evaluate".into(), tool: "presence.evaluate".into(), args: serde_json::json!({"cameras": "$camera_people", "network": "$network_devices"}), continue_on_error: false, condition: None },
                PipelineStep { id: "automate".into(), tool: "home.scenes".into(), args: serde_json::json!({"scene": "$evaluate.recommended_scene"}), continue_on_error: true, condition: Some("$evaluate.changed == true".into()) },
            ],
            on_failure: FailureAction::Continue,
            enabled: false, // user enables after connecting HA
        },

        // ── 5. IDS Response Pipeline ──
        Pipeline {
            name: "ids_response".into(),
            description: "Respond to Suricata/Snort IDS alerts".into(),
            trigger: PipelineTrigger::Event {
                subject: "rednode.security.ids".into(),
                filter: Some("severity >= high".into()),
            },
            steps: vec![
                PipelineStep { id: "analyze".into(), tool: "sec.triage".into(), args: serde_json::json!({"alert": "$event"}), continue_on_error: false, condition: None },
                PipelineStep { id: "block".into(), tool: "fw.block_ip".into(), args: serde_json::json!({"ip": "$event.src_ip", "reason": "IDS alert: $event.signature"}), continue_on_error: true, condition: Some("$analyze.action == 'block'".into()) },
                PipelineStep { id: "isolate".into(), tool: "fw.isolate_device".into(), args: serde_json::json!({"ip": "$event.dst_ip"}), continue_on_error: true, condition: Some("$analyze.action == 'isolate'".into()) },
                PipelineStep { id: "notify".into(), tool: "signal.send".into(), args: serde_json::json!({"message": "🛡️ IDS: $event.signature — $analyze.action taken"}), continue_on_error: true, condition: None },
            ],
            on_failure: FailureAction::Notify { message: "IDS response pipeline failed".into() },
            enabled: true,
        },
    ]
}

/// Execute a pipeline given step results from previous steps.
/// Returns the compiled result of all steps.
pub async fn execute_pipeline(pipeline: &Pipeline) -> PipelineResult {
    let start = std::time::Instant::now();
    let mut results: Vec<StepResult> = Vec::new();
    let mut step_outputs: HashMap<String, serde_json::Value> = HashMap::new();

    for (i, step) in pipeline.steps.iter().enumerate() {
        let step_start = std::time::Instant::now();

        // Check condition
        if let Some(ref cond) = step.condition {
            if !evaluate_condition(cond, &step_outputs) {
                results.push(StepResult {
                    id: step.id.clone(),
                    tool: step.tool.clone(),
                    success: true,
                    output: serde_json::json!({"skipped": true, "reason": "condition not met"}),
                    duration_ms: 0,
                });
                continue;
            }
        }

        // Resolve variable references in args
        let resolved_args = resolve_variables(&step.args, &step_outputs);

        // Execute via coordinator
        let (ok, errs) = crate::coordinator::coordinate(
            &format!("pipeline:{} tool:{} args:{}", pipeline.name, step.tool, resolved_args),
            &format!("pipeline-{}", pipeline.name),
        ).await;

        let success = errs.is_empty();
        let output = if !ok.is_empty() {
            ok.into_iter().next().unwrap_or(serde_json::json!({}))
        } else if !errs.is_empty() {
            errs.into_iter().next().unwrap_or(serde_json::json!({"error": "unknown"}))
        } else {
            serde_json::json!({})
        };

        step_outputs.insert(step.id.clone(), output.clone());

        results.push(StepResult {
            id: step.id.clone(),
            tool: step.tool.clone(),
            success,
            output,
            duration_ms: step_start.elapsed().as_millis(),
        });

        if !success && !step.continue_on_error {
            break;
        }

        // Circuit breaker: total time
        if start.elapsed().as_secs() > 300 {
            tracing::warn!(pipeline = pipeline.name, "pipeline timeout after 300s");
            break;
        }
    }

    let completed = results.iter().filter(|r| r.success).count();

    PipelineResult {
        pipeline: pipeline.name.clone(),
        success: results.iter().all(|r| r.success),
        steps_completed: completed,
        steps_total: pipeline.steps.len(),
        results,
        duration_ms: start.elapsed().as_millis(),
    }
}

/// Evaluate a simple condition string against step outputs.
/// Supports: $step_id.field == "value", $step_id.field != null, $step_id.field >= N
fn evaluate_condition(cond: &str, outputs: &HashMap<String, serde_json::Value>) -> bool {
    // Parse: "$step.field op value"
    let parts: Vec<&str> = cond.splitn(3, ' ').collect();
    if parts.len() < 3 { return true; } // malformed → run anyway

    let path = parts[0];
    let op = parts[1];
    let expected = parts[2];

    // Resolve $step.field
    if let Some(val) = resolve_path(path, outputs) {
        match op {
            "==" => {
                if expected == "true" { return val.as_bool().unwrap_or(false); }
                if expected == "false" { return !val.as_bool().unwrap_or(true); }
                if expected == "null" { return val.is_null(); }
                val.as_str().map(|s| s == expected).unwrap_or(false)
            }
            "!=" => {
                if expected == "null" { return !val.is_null(); }
                val.as_str().map(|s| s != expected).unwrap_or(true)
            }
            ">=" => {
                if let (Some(a), Ok(b)) = (val.as_f64(), expected.parse::<f64>()) {
                    a >= b
                } else { false }
            }
            _ => true,
        }
    } else {
        false
    }
}

/// Resolve a $step.field path to a JSON value
fn resolve_path(path: &str, outputs: &HashMap<String, serde_json::Value>) -> Option<serde_json::Value> {
    if !path.starts_with('$') { return None; }
    let path = &path[1..]; // strip $
    let parts: Vec<&str> = path.splitn(2, '.').collect();
    let step_id = parts[0];
    let field = parts.get(1).copied();

    let val = outputs.get(step_id)?;
    if let Some(field) = field {
        Some(val.get(field).cloned().unwrap_or(serde_json::Value::Null))
    } else {
        Some(val.clone())
    }
}

/// Replace $step.field references in args with actual values
fn resolve_variables(args: &serde_json::Value, outputs: &HashMap<String, serde_json::Value>) -> serde_json::Value {
    match args {
        serde_json::Value::String(s) => {
            if s.starts_with('$') {
                resolve_path(s, outputs).unwrap_or(serde_json::Value::String(s.clone()))
            } else if s.contains('$') {
                // Template substitution: "message: $step.field"
                let mut result = s.clone();
                for (key, val) in outputs {
                    let placeholder = format!("${}", key);
                    if result.contains(&placeholder) {
                        let replacement = match val {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        result = result.replace(&placeholder, &replacement);
                    }
                    // Also handle $key.field
                    if let serde_json::Value::Object(map) = val {
                        for (field, fval) in map {
                            let ph = format!("${}.{}", key, field);
                            if result.contains(&ph) {
                                let replacement = match fval {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                };
                                result = result.replace(&ph, &replacement);
                            }
                        }
                    }
                }
                serde_json::Value::String(result)
            } else {
                serde_json::Value::String(s.clone())
            }
        }
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), resolve_variables(v, outputs));
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| resolve_variables(v, outputs)).collect())
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_condition_eq() {
        let mut outputs = HashMap::new();
        outputs.insert("check".into(), serde_json::json!({"action": "block", "found": true}));

        assert!(evaluate_condition("$check.action == block", &outputs));
        assert!(!evaluate_condition("$check.action == allow", &outputs));
        assert!(evaluate_condition("$check.found == true", &outputs));
        assert!(evaluate_condition("$check.action != null", &outputs));
        assert!(!evaluate_condition("$check.missing != null", &outputs));
    }

    #[test]
    fn test_resolve_variables() {
        let mut outputs = HashMap::new();
        outputs.insert("step1".into(), serde_json::json!({"ip": "192.168.1.100", "count": 5}));

        let args = serde_json::json!({"target": "$step1.ip", "msg": "Found $step1.count items"});
        let resolved = resolve_variables(&args, &outputs);

        assert_eq!(resolved["target"], "192.168.1.100");
        assert_eq!(resolved["msg"], "Found 5 items");
    }

    #[test]
    fn test_builtin_pipelines_valid() {
        let pipelines = builtin_pipelines();
        assert!(pipelines.len() >= 5);
        for p in &pipelines {
            assert!(!p.name.is_empty());
            assert!(!p.steps.is_empty());
        }
    }
}
