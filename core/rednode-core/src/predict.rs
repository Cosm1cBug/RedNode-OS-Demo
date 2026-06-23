// RedNode-OS — Predictive Maintenance Engine
//
// Collects hardware telemetry over time and uses trend analysis to
// predict failures before they happen.
//
// Tracks:
//   - Disk SMART attributes (reallocated sectors, pending sectors, temperature)
//   - RAM errors (error-correcting memory counts if available, MCE events)
//   - CPU temperature trends
//   - GPU temperature / VRAM errors
//   - Fan RPM degradation
//   - Network interface error rates
//   - Power supply voltage fluctuations (UPS data)
//
// Prediction method: linear regression on time-series data.
//   If projected value crosses threshold within 30 days → alert.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    pub timestamp: i64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetric {
    pub name: String,
    pub device: String,
    pub samples: Vec<MetricSample>,
    pub threshold_warn: f64,
    pub threshold_critical: f64,
    pub higher_is_worse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub metric: String,
    pub device: String,
    pub current_value: f64,
    pub trend_per_day: f64,
    pub days_to_warn: Option<f64>,
    pub days_to_critical: Option<f64>,
    pub status: PredictionStatus,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PredictionStatus {
    Healthy,
    Degrading,
    Warning,
    Critical,
    FailureImminent,
}

/// Linear regression: y = a + bx. Returns (slope, intercept).
fn linear_regression(samples: &[MetricSample]) -> Option<(f64, f64)> {
    let n = samples.len() as f64;
    if n < 3.0 {
        return None;
    } // need at least 3 points

    let sum_x: f64 = samples.iter().map(|s| s.timestamp as f64).sum();
    let sum_y: f64 = samples.iter().map(|s| s.value).sum();
    let sum_xy: f64 = samples.iter().map(|s| (s.timestamp as f64) * s.value).sum();
    let sum_x2: f64 = samples.iter().map(|s| (s.timestamp as f64).powi(2)).sum();

    let denom = n * sum_x2 - sum_x.powi(2);
    if denom.abs() < 1e-10 {
        return None;
    } // perfectly flat or degenerate

    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;

    Some((slope, intercept))
}

/// Predict when a metric will cross a threshold.
/// Returns days from now until crossing, or None if trend is away from threshold.
fn days_until_threshold(
    samples: &[MetricSample],
    threshold: f64,
    higher_is_worse: bool,
) -> Option<f64> {
    let (slope, intercept) = linear_regression(samples)?;
    let now = chrono::Utc::now().timestamp() as f64;
    let current = slope * now + intercept;

    // Check if trend is heading toward threshold
    if higher_is_worse {
        if slope <= 0.0 {
            return None;
        } // improving
        if current >= threshold {
            return Some(0.0);
        } // already past
        let seconds = (threshold - current) / slope;
        Some(seconds / 86400.0) // convert to days
    } else {
        if slope >= 0.0 {
            return None;
        } // improving
        if current <= threshold {
            return Some(0.0);
        } // already past
        let seconds = (current - threshold) / slope.abs();
        Some(seconds / 86400.0)
    }
}

/// Analyze a set of health metrics and produce predictions.
pub fn analyze_health(metrics: &[HealthMetric]) -> Vec<PredictionResult> {
    let mut results = Vec::new();

    for metric in metrics {
        if metric.samples.is_empty() {
            continue;
        }

        let current = metric.samples.last().map(|s| s.value).unwrap_or(0.0);
        let trend = linear_regression(&metric.samples)
            .map(|(slope, _)| slope * 86400.0) // convert per-second to per-day
            .unwrap_or(0.0);

        let days_warn = days_until_threshold(
            &metric.samples,
            metric.threshold_warn,
            metric.higher_is_worse,
        );
        let days_crit = days_until_threshold(
            &metric.samples,
            metric.threshold_critical,
            metric.higher_is_worse,
        );

        let status = if let Some(d) = days_crit {
            if d <= 0.0 {
                PredictionStatus::Critical
            } else if d <= 7.0 {
                PredictionStatus::FailureImminent
            } else if d <= 30.0 {
                PredictionStatus::Warning
            } else {
                PredictionStatus::Degrading
            }
        } else if let Some(d) = days_warn {
            if d <= 0.0 {
                PredictionStatus::Warning
            } else if d <= 30.0 {
                PredictionStatus::Degrading
            } else {
                PredictionStatus::Healthy
            }
        } else {
            PredictionStatus::Healthy
        };

        let recommendation = match &status {
            PredictionStatus::Healthy => format!("{} is healthy — no action needed", metric.name),
            PredictionStatus::Degrading => format!(
                "{} is slowly degrading (trend: {:.2}/day) — monitor closely",
                metric.name, trend
            ),
            PredictionStatus::Warning => format!(
                "⚠️ {} approaching warning threshold — predicted in {:.0} days — plan replacement",
                metric.name,
                days_warn.unwrap_or(0.0)
            ),
            PredictionStatus::Critical => format!(
                "🔴 {} at critical level ({:.1}) — immediate attention required",
                metric.name, current
            ),
            PredictionStatus::FailureImminent => format!(
                "🚨 {} predicted to fail within {:.0} days — replace NOW",
                metric.name,
                days_crit.unwrap_or(0.0)
            ),
        };

        results.push(PredictionResult {
            metric: metric.name.clone(),
            device: metric.device.clone(),
            current_value: current,
            trend_per_day: trend,
            days_to_warn: days_warn,
            days_to_critical: days_crit,
            status,
            recommendation,
        });
    }

    // Sort: most urgent first
    results.sort_by(|a, b| {
        let urgency = |s: &PredictionStatus| match s {
            PredictionStatus::FailureImminent => 0,
            PredictionStatus::Critical => 1,
            PredictionStatus::Warning => 2,
            PredictionStatus::Degrading => 3,
            PredictionStatus::Healthy => 4,
        };
        urgency(&a.status).cmp(&urgency(&b.status))
    });

    results
}

/// Build SMART health metrics from raw smartctl data.
pub fn smart_to_metrics(smart_data: &serde_json::Value) -> Vec<HealthMetric> {
    let mut metrics = Vec::new();
    let device = smart_data
        .get("device")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let now = chrono::Utc::now().timestamp();

    // Key SMART attributes that predict failure
    let attrs = [
        ("Reallocated_Sector_Ct", 0.0, 5.0, 50.0, true),
        ("Current_Pending_Sector", 0.0, 1.0, 10.0, true),
        ("Offline_Uncorrectable", 0.0, 1.0, 5.0, true),
        ("Temperature_Celsius", 0.0, 55.0, 65.0, true),
        ("Airflow_Temperature_Cel", 0.0, 55.0, 65.0, true),
        ("UDMA_CRC_Error_Count", 0.0, 10.0, 100.0, true),
        ("Wear_Leveling_Count", 100.0, 20.0, 5.0, false), // lower is worse for SSD
        ("Media_Wearout_Indicator", 100.0, 20.0, 5.0, false),
    ];

    if let Some(ata_attrs) = smart_data
        .get("ata_smart_attributes")
        .and_then(|v| v.get("table"))
        .and_then(|v| v.as_array())
    {
        for attr in ata_attrs {
            let name = attr.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let raw_value = attr
                .get("raw")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            for (target_name, _default, warn, crit, higher_is_worse) in &attrs {
                if name == *target_name {
                    metrics.push(HealthMetric {
                        name: name.to_string(),
                        device: device.to_string(),
                        samples: vec![MetricSample {
                            timestamp: now,
                            value: raw_value,
                        }],
                        threshold_warn: *warn,
                        threshold_critical: *crit,
                        higher_is_worse: *higher_is_worse,
                    });
                }
            }
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_regression() {
        let samples = vec![
            MetricSample {
                timestamp: 100,
                value: 10.0,
            },
            MetricSample {
                timestamp: 200,
                value: 20.0,
            },
            MetricSample {
                timestamp: 300,
                value: 30.0,
            },
        ];
        let (slope, _intercept) = linear_regression(&samples).unwrap();
        assert!((slope - 0.1).abs() < 0.01); // 10 per 100 seconds
    }

    #[test]
    fn test_days_until_threshold() {
        let now = chrono::Utc::now().timestamp();
        let day = 86400;
        let samples = vec![
            MetricSample {
                timestamp: now - 10 * day,
                value: 0.0,
            },
            MetricSample {
                timestamp: now - 5 * day,
                value: 5.0,
            },
            MetricSample {
                timestamp: now,
                value: 10.0,
            },
        ];

        let days = days_until_threshold(&samples, 20.0, true);
        assert!(days.is_some());
        let d = days.unwrap();
        assert!(d > 5.0 && d < 15.0, "Expected ~10 days, got {}", d);
    }

    #[test]
    fn test_healthy_metric() {
        let now = chrono::Utc::now().timestamp();
        let day = 86400;
        let metrics = vec![HealthMetric {
            name: "Temperature".into(),
            device: "sda".into(),
            samples: vec![
                MetricSample {
                    timestamp: now - 30 * day,
                    value: 35.0,
                },
                MetricSample {
                    timestamp: now - 15 * day,
                    value: 35.0,
                },
                MetricSample {
                    timestamp: now,
                    value: 35.0,
                },
            ],
            threshold_warn: 55.0,
            threshold_critical: 65.0,
            higher_is_worse: true,
        }];

        let results = analyze_health(&metrics);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, PredictionStatus::Healthy);
    }
}
