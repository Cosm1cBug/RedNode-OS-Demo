// RedNode-OS — Smart Notification System
//
// Context-aware notification routing:
//   - Classifies notifications by urgency (critical/high/normal/low)
//   - Time-aware: don't wake at 3 AM for non-critical events
//   - Batches low-priority notifications into digests
//   - Routes through preferred channel (Signal, web dashboard, email)
//
// Urgency rules:
//   Critical: security breach, service down, hardware failure → immediate always
//   High:     failed login, CVE found, disk >90% → immediate during waking hours
//   Normal:   camera event, email summary, task reminder → batch into digest
//   Low:      software update available, pattern learned → next morning briefing

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

static NOTIFICATION_QUEUE: once_cell::sync::Lazy<Mutex<NotificationQueue>> =
    once_cell::sync::Lazy::new(|| Mutex::new(NotificationQueue::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub urgency: Urgency,
    pub source: String,   // which agent/pipeline generated this
    pub category: String, // security, camera, storage, email, system, etc.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub delivered: bool,
    pub channel: Option<String>, // signal, web, email — None = auto-select
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Urgency {
    Critical = 4, // always immediate
    High = 3,     // immediate during waking hours, queued otherwise
    Normal = 2,   // batched into digest
    Low = 1,      // next morning briefing
}

pub struct NotificationQueue {
    pub pending: VecDeque<Notification>,
    pub delivered: Vec<Notification>,
    /// Quiet hours: don't send non-critical notifications during this window
    pub quiet_start_hour: u32, // e.g., 22 (10 PM)
    pub quiet_end_hour: u32, // e.g., 7 (7 AM)
    pub timezone_offset_hours: i32,
}

impl NotificationQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            delivered: Vec::new(),
            quiet_start_hour: 22,
            quiet_end_hour: 7,
            timezone_offset_hours: 5, // IST = UTC+5:30 (approx)
        }
    }

    /// Add a notification. Returns true if it should be delivered immediately.
    pub fn push(&mut self, notif: Notification) -> bool {
        let should_deliver_now = self.should_deliver_now(&notif);
        self.pending.push_back(notif);
        should_deliver_now
    }

    /// Check if a notification should be delivered immediately based on urgency and time.
    fn should_deliver_now(&self, notif: &Notification) -> bool {
        match notif.urgency {
            Urgency::Critical => true, // always
            Urgency::High => !self.is_quiet_hours(),
            Urgency::Normal => false, // batch
            Urgency::Low => false,    // morning briefing
        }
    }

    fn is_quiet_hours(&self) -> bool {
        let now = chrono::Utc::now();
        let local_hour = ((now.timestamp() / 3600 + self.timezone_offset_hours as i64) % 24) as u32;

        if self.quiet_start_hour > self.quiet_end_hour {
            // Crosses midnight: e.g., 22-7
            local_hour >= self.quiet_start_hour || local_hour < self.quiet_end_hour
        } else {
            local_hour >= self.quiet_start_hour && local_hour < self.quiet_end_hour
        }
    }

    /// Get all pending notifications for digest delivery (Normal + Low urgency).
    pub fn drain_digest(&mut self) -> Vec<Notification> {
        let mut digest = Vec::new();
        let mut remaining = VecDeque::new();

        while let Some(mut n) = self.pending.pop_front() {
            if n.urgency <= Urgency::Normal && !n.delivered {
                n.delivered = true;
                digest.push(n.clone());
                self.delivered.push(n);
            } else {
                remaining.push_back(n);
            }
        }
        self.pending = remaining;
        digest
    }

    /// Get all undelivered high-urgency notifications (for when quiet hours end).
    pub fn drain_high_urgency(&mut self) -> Vec<Notification> {
        let mut urgent = Vec::new();
        let mut remaining = VecDeque::new();

        while let Some(mut n) = self.pending.pop_front() {
            if n.urgency >= Urgency::High && !n.delivered {
                n.delivered = true;
                urgent.push(n.clone());
                self.delivered.push(n);
            } else {
                remaining.push_back(n);
            }
        }
        self.pending = remaining;
        urgent
    }
}

/// Classify the urgency of an event based on its type and content.
pub fn classify_urgency(category: &str, event: &serde_json::Value) -> Urgency {
    match category {
        "security" => {
            let severity = event
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("medium");
            match severity {
                "critical" => Urgency::Critical,
                "high" => Urgency::High,
                _ => Urgency::Normal,
            }
        }
        "service_down" | "hardware_failure" => Urgency::Critical,
        "camera" => {
            let event_type = event.get("label").and_then(|v| v.as_str()).unwrap_or("");
            match event_type {
                "person" => Urgency::High,
                _ => Urgency::Normal,
            }
        }
        "disk" => {
            let usage_pct = event
                .get("usage_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if usage_pct > 95.0 {
                Urgency::Critical
            } else if usage_pct > 90.0 {
                Urgency::High
            } else {
                Urgency::Low
            }
        }
        "email" => Urgency::Normal,
        "update" | "pattern" | "learning" => Urgency::Low,
        _ => Urgency::Normal,
    }
}

/// Queue a notification from any part of the system.
pub fn notify(title: &str, body: &str, urgency: Urgency, source: &str, category: &str) -> bool {
    let notif = Notification {
        id: uuid_v4(),
        title: title.into(),
        body: body.into(),
        urgency,
        source: source.into(),
        category: category.into(),
        timestamp: chrono::Utc::now(),
        delivered: false,
        channel: None,
    };

    if let Ok(mut q) = NOTIFICATION_QUEUE.lock() {
        q.push(notif)
    } else {
        true // if lock fails, deliver immediately as fallback
    }
}

/// Get pending notifications for digest.
pub fn get_digest() -> Vec<Notification> {
    if let Ok(mut q) = NOTIFICATION_QUEUE.lock() {
        q.drain_digest()
    } else {
        vec![]
    }
}

/// Simple UUID v4 without external crate
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{:x}-{:x}-4{:x}-{:x}",
        t.as_nanos() & 0xFFFF_FFFF,
        (t.as_nanos() >> 32) & 0xFFFF,
        (t.as_nanos() >> 48) & 0x0FFF,
        (t.as_nanos() >> 60) & 0x3FFF | 0x8000,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urgency_ordering() {
        assert!(Urgency::Critical > Urgency::High);
        assert!(Urgency::High > Urgency::Normal);
        assert!(Urgency::Normal > Urgency::Low);
    }

    #[test]
    fn test_classify_security() {
        let event = serde_json::json!({"severity": "critical"});
        assert_eq!(classify_urgency("security", &event), Urgency::Critical);

        let event = serde_json::json!({"severity": "low"});
        assert_eq!(classify_urgency("security", &event), Urgency::Normal);
    }

    #[test]
    fn test_classify_disk() {
        let event = serde_json::json!({"usage_pct": 96.0});
        assert_eq!(classify_urgency("disk", &event), Urgency::Critical);

        let event = serde_json::json!({"usage_pct": 50.0});
        assert_eq!(classify_urgency("disk", &event), Urgency::Low);
    }

    #[test]
    fn test_queue_digest() {
        let mut q = NotificationQueue::new();
        q.push(Notification {
            id: "1".into(),
            title: "Test".into(),
            body: "body".into(),
            urgency: Urgency::Low,
            source: "test".into(),
            category: "test".into(),
            timestamp: chrono::Utc::now(),
            delivered: false,
            channel: None,
        });
        q.push(Notification {
            id: "2".into(),
            title: "Critical".into(),
            body: "urgent".into(),
            urgency: Urgency::Critical,
            source: "test".into(),
            category: "security".into(),
            timestamp: chrono::Utc::now(),
            delivered: false,
            channel: None,
        });

        let digest = q.drain_digest();
        assert_eq!(digest.len(), 1); // only Low/Normal go to digest
        assert_eq!(digest[0].id, "1");
    }
}
