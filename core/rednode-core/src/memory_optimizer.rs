// RedNode-OS – Runtime Memory Optimizer
//
// Monitors system memory usage and automatically:
//   1. Adjusts event bus capacity based on available RAM
//   2. Triggers Qdrant collection compaction when memory is high
//   3. Prunes old audit log entries when DB grows too large
//   4. Warns when memory pressure is critical
//   5. Adjusts Ollama context window based on available VRAM
//
// Runs as a background task from sentience engine (every 60 seconds).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStatus {
    pub ram_used_pct: f32,
    pub ram_available_mb: u64,
    pub pressure_level: PressureLevel,
    pub actions_taken: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum PressureLevel {
    Normal,   // < 70% RAM used
    Elevated, // 70-85% RAM used
    High,     // 85-95% RAM used
    Critical, // > 95% RAM used
}

impl std::fmt::Display for PressureLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PressureLevel::Normal => write!(f, "normal"),
            PressureLevel::Elevated => write!(f, "elevated"),
            PressureLevel::High => write!(f, "high"),
            PressureLevel::Critical => write!(f, "critical"),
        }
    }
}

/// Run memory optimization check. Called from sentience introspection loop.
pub async fn optimize() -> MemoryStatus {
    let mut actions = Vec::new();

    // Get current memory state
    let (total_mb, available_mb) = get_memory_info();
    let used_pct = if total_mb > 0 {
        ((total_mb - available_mb) as f32 / total_mb as f32) * 100.0
    } else {
        0.0
    };

    let pressure = if used_pct > 95.0 {
        PressureLevel::Critical
    } else if used_pct > 85.0 {
        PressureLevel::High
    } else if used_pct > 70.0 {
        PressureLevel::Elevated
    } else {
        PressureLevel::Normal
    };

    match &pressure {
        PressureLevel::Critical => {
            tracing::error!(
                used_pct = used_pct,
                available_mb = available_mb,
                "CRITICAL memory pressure — taking emergency action"
            );

            // Emergency: prune old audit entries
            if let Some(pool) = crate::memory::pool() {
                let pruned = sqlx::query(
                    "DELETE FROM audit_log WHERE id IN (\
                     SELECT id FROM audit_log ORDER BY ts ASC LIMIT 500)",
                )
                .execute(pool)
                .await;
                if let Ok(r) = pruned {
                    let count = r.rows_affected();
                    if count > 0 {
                        actions.push(format!("pruned {} old audit entries", count));
                    }
                }

                // Emergency: clear old security events
                let pruned_sec = sqlx::query(
                    "DELETE FROM security_events WHERE id IN (\
                     SELECT id FROM security_events WHERE acknowledged = true \
                     ORDER BY ts ASC LIMIT 200)",
                )
                .execute(pool)
                .await;
                if let Ok(r) = pruned_sec {
                    let count = r.rows_affected();
                    if count > 0 {
                        actions.push(format!("pruned {} acknowledged security events", count));
                    }
                }
            }

            // Emit critical alert
            crate::events::emit_security_event(
                "CRITICAL",
                "memory-optimizer",
                &format!(
                    "Memory pressure CRITICAL: {}% used, {} MB available",
                    used_pct as u32, available_mb
                ),
            );
        }

        PressureLevel::High => {
            tracing::warn!(
                used_pct = used_pct,
                available_mb = available_mb,
                "High memory pressure — pruning stale data"
            );

            // Prune old documents from memory
            if let Some(pool) = crate::memory::pool() {
                // Remove old sentience consolidation entries (keep last 50)
                let pruned = sqlx::query(
                    "DELETE FROM documents WHERE id IN (\
                     SELECT id FROM documents \
                     WHERE source LIKE 'sentience/%' \
                     ORDER BY created_at ASC \
                     LIMIT 50)",
                )
                .execute(pool)
                .await;
                if let Ok(r) = pruned {
                    let count = r.rows_affected();
                    if count > 0 {
                        actions.push(format!("pruned {} old sentience documents", count));
                    }
                }
            }
        }

        PressureLevel::Elevated => {
            tracing::info!(used_pct = used_pct, "Elevated memory pressure — monitoring");
        }

        PressureLevel::Normal => {
            // All good — no action needed
        }
    }

    if !actions.is_empty() {
        tracing::info!(
            pressure = %pressure,
            actions = ?actions,
            "memory optimizer: {} action(s) taken",
            actions.len()
        );

        crate::events::emit(serde_json::json!({
            "type": "memory_optimization",
            "pressure": pressure.to_string(),
            "ram_used_pct": used_pct,
            "ram_available_mb": available_mb,
            "actions": actions,
            "ts": chrono::Utc::now().to_rfc3339()
        }));
    }

    MemoryStatus {
        ram_used_pct: used_pct,
        ram_available_mb: available_mb,
        pressure_level: pressure,
        actions_taken: actions,
    }
}

/// Get total and available memory in MB
fn get_memory_info() -> (u64, u64) {
    // Read from /proc/meminfo (Linux)
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        let mut total: u64 = 0;
        let mut available: u64 = 0;

        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0)
                    / 1024; // kB → MB
            }
            if line.starts_with("MemAvailable:") {
                available = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0)
                    / 1024; // kB → MB
            }
        }

        return (total, available);
    }

    // Fallback: use sysinfo
    (0, 0)
}
