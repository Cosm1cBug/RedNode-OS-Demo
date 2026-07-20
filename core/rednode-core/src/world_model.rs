// RedNode-OS — World Model
//
// A unified, relationship-aware model of everything RedNode knows about.
// Not just a flat inventory — a living graph of infrastructure, people,
// projects, services, and the dependencies between them.
//
// The world model answers questions like:
//   - "What depends on TrueNAS?"
//   - "If pfSense goes down, what breaks?"
//   - "Show me all devices on VLAN 20"
//   - "What changed in my infrastructure this week?"
//
// Data sources:
//   - Network scans (ARP, mDNS, SNMP)
//   - Agent reports (heartbeats, service checks)
//   - Manual registration via API / dashboard
//   - Home Assistant entity discovery
//   - Docker/Podman container introspection
//   - NVR camera feed metadata
//
// Persistence: PostgreSQL (world_model_store table)
// Graph queries: in-memory adjacency list + optional Kuzu
//
// API:
//   GET    /world                        — full world model snapshot
//   GET    /world/machines               — all machines
//   GET    /world/machines/:id           — single machine detail
//   POST   /world/machines               — register/update a machine
//   DELETE /world/machines/:id           — remove a machine
//   GET    /world/services               — all service nodes
//   POST   /world/services               — register a service
//   GET    /world/topology               — network topology (VLANs + links)
//   GET    /world/dependencies/:id       — dependency chain for an entity
//   GET    /world/impact/:id             — impact analysis ("what breaks?")
//   GET    /world/diff                   — changes since last snapshot
//   POST   /world/scan                   — trigger a network scan

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

static WORLD: once_cell::sync::Lazy<Arc<RwLock<WorldModel>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(WorldModel::default())));

// ─── Core Types ───

/// The complete world model — everything RedNode knows about its environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldModel {
    pub machines: Vec<Machine>,
    pub services: Vec<ServiceNode>,
    pub vlans: Vec<Vlan>,
    pub containers: Vec<Container>,
    pub cameras: Vec<Camera>,
    pub iot_devices: Vec<IoTDevice>,
    pub people: Vec<Person>,
    pub projects: Vec<Project>,

    /// Directed edges: entity_id -> list of (target_id, relation)
    pub edges: HashMap<String, Vec<Edge>>,

    /// Threat layer: active threats mapped to infrastructure entities
    pub threat_layer: Vec<ThreatMapping>,
    /// Economic layer: cost per service/machine
    pub economic_layer: Vec<ServiceCost>,

    pub last_scan: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
    pub snapshot_history: VecDeque<WorldSnapshot>,
}

/// A threat mapped to a specific infrastructure entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatMapping {
    pub entity_id: String,
    pub entity_name: String,
    pub threat_type: String,
    pub severity: String,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub mitigated: bool,
}

/// Cost associated with running a service or machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCost {
    pub entity_id: String,
    pub entity_name: String,
    pub cost_per_month: f64,
    pub currency: String,
    pub category: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub role: MachineRole,
    pub ip: Option<String>,
    pub mac: Option<String>,
    pub os: Option<String>,
    pub cpu: Option<String>,
    pub ram_gb: Option<u32>,
    pub disk_gb: Option<u32>,
    pub services: Vec<String>,
    pub vlan_id: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub health: f32,
    pub tags: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MachineRole {
    Server,
    Desktop,
    Nas,
    Firewall,
    Router,
    Switch,
    RaspberryPi,
    Nvr,
    IoTHub,
    Workstation,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNode {
    pub id: String,
    pub name: String,
    pub service_type: ServiceType,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub host_machine_id: Option<String>,
    pub status: ServiceStatus,
    pub last_checked: DateTime<Utc>,
    pub version: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    Database,
    MessageBroker,
    WebApp,
    Api,
    Dns,
    Dhcp,
    Firewall,
    VPN,
    FileStorage,
    MediaServer,
    HomeAutomation,
    Monitoring,
    Container,
    LLM,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Down,
    Unknown,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vlan {
    pub id: String,
    pub vlan_id: u16,
    pub name: String,
    pub subnet: String,
    pub gateway: Option<String>,
    pub purpose: String,
    pub machine_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub host_machine_id: String,
    pub status: ContainerStatus,
    pub ports: Vec<PortMapping>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Restarting,
    Paused,
    Exited,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub id: String,
    pub name: String,
    pub location: String,
    pub nvr_id: Option<String>,
    pub rtsp_url: Option<String>,
    pub status: ServiceStatus,
    pub resolution: Option<String>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoTDevice {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub protocol: String,
    pub ip: Option<String>,
    pub ha_entity_id: Option<String>,
    pub vlan_id: Option<String>,
    pub last_seen: DateTime<Utc>,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub role: String,
    pub devices: Vec<String>,
    pub notification_channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub repo_url: Option<String>,
    pub services: Vec<String>,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Active,
    Archived,
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub target: String,
    pub relation: EdgeRelation,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeRelation {
    DependsOn,
    Hosts,
    RunsOn,
    ConnectedTo,
    Monitors,
    BacksUp,
    RoutesThrough,
    OwnedBy,
    PartOf,
    Custom(String),
}

/// A point-in-time snapshot for diff comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub taken_at: DateTime<Utc>,
    pub machine_count: usize,
    pub service_count: usize,
    pub container_count: usize,
    pub camera_count: usize,
    pub iot_count: usize,
    pub machine_ids: Vec<String>,
    pub service_ids: Vec<String>,
}

/// Diff between two snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldDiff {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub machines_added: Vec<String>,
    pub machines_removed: Vec<String>,
    pub services_added: Vec<String>,
    pub services_removed: Vec<String>,
    pub health_changes: Vec<HealthChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChange {
    pub entity_id: String,
    pub entity_name: String,
    pub old_health: f32,
    pub new_health: f32,
}

/// Result of impact analysis — what breaks if entity X goes down
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub source_id: String,
    pub source_name: String,
    pub directly_affected: Vec<AffectedEntity>,
    pub transitively_affected: Vec<AffectedEntity>,
    pub total_impact_count: usize,
    pub severity: ImpactSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub relation: String,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactSeverity {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl Default for WorldModel {
    fn default() -> Self {
        Self {
            machines: Vec::new(),
            services: Vec::new(),
            vlans: Vec::new(),
            containers: Vec::new(),
            cameras: Vec::new(),
            iot_devices: Vec::new(),
            people: Vec::new(),
            projects: Vec::new(),
            edges: HashMap::new(),
            threat_layer: Vec::new(),
            economic_layer: Vec::new(),
            last_scan: None,
            last_updated: Utc::now(),
            snapshot_history: VecDeque::with_capacity(30),
        }
    }
}

// ─── Public API: Read ───

/// Get the full world model (read-only clone)
pub async fn get_world() -> WorldModel {
    WORLD.read().await.clone()
}

/// Get all machines
pub async fn get_machines() -> Vec<Machine> {
    WORLD.read().await.machines.clone()
}

/// Get a specific machine by ID
pub async fn get_machine(id: &str) -> Option<Machine> {
    WORLD.read().await.machines.iter().find(|m| m.id == id).cloned()
}

/// Get all services
pub async fn get_services() -> Vec<ServiceNode> {
    WORLD.read().await.services.clone()
}

/// Get the network topology (VLANs with machine assignments)
pub async fn get_topology() -> Vec<Vlan> {
    WORLD.read().await.vlans.clone()
}

/// Get all containers
pub async fn get_containers() -> Vec<Container> {
    WORLD.read().await.containers.clone()
}

/// Get entity counts for the consciousness layer
pub async fn entity_counts() -> (usize, usize, usize, usize, usize) {
    let world = WORLD.read().await;
    (
        world.machines.len(),
        world.services.len(),
        world.containers.len(),
        world.cameras.len(),
        world.iot_devices.len(),
    )
}

// ─── Public API: Write ───

/// Register or update a machine
pub async fn upsert_machine(machine: Machine) {
    let mut world = WORLD.write().await;
    if let Some(existing) = world.machines.iter_mut().find(|m| m.id == machine.id) {
        *existing = machine;
    } else {
        let name = machine.name.clone();
        let id = machine.id.clone();
        world.machines.push(machine);
        tracing::info!(id = %id, name = %name, "World model: machine registered");
    }
    world.last_updated = Utc::now();
}

/// Remove a machine by ID
pub async fn remove_machine(id: &str) -> bool {
    let mut world = WORLD.write().await;
    let before = world.machines.len();
    world.machines.retain(|m| m.id != id);
    // Also remove edges involving this machine
    world.edges.remove(id);
    for edges in world.edges.values_mut() {
        edges.retain(|e| e.target != id);
    }
    world.last_updated = Utc::now();
    world.machines.len() < before
}

/// Register or update a service
pub async fn upsert_service(service: ServiceNode) {
    let mut world = WORLD.write().await;
    if let Some(existing) = world.services.iter_mut().find(|s| s.id == service.id) {
        *existing = service;
    } else {
        let name = service.name.clone();
        let id = service.id.clone();

        // Auto-create edge: service -> host machine
        if let Some(ref host_id) = service.host_machine_id {
            world.edges
                .entry(id.clone())
                .or_default()
                .push(Edge {
                    target: host_id.clone(),
                    relation: EdgeRelation::RunsOn,
                    metadata: None,
                });
        }

        world.services.push(service);
        tracing::info!(id = %id, name = %name, "World model: service registered");
    }
    world.last_updated = Utc::now();
}

/// Register or update a container
pub async fn upsert_container(container: Container) {
    let mut world = WORLD.write().await;
    if let Some(existing) = world.containers.iter_mut().find(|c| c.id == container.id) {
        *existing = container;
    } else {
        // Auto-create edge: container -> host machine
        let id = container.id.clone();
        let host = container.host_machine_id.clone();
        world.edges
            .entry(id.clone())
            .or_default()
            .push(Edge {
                target: host,
                relation: EdgeRelation::RunsOn,
                metadata: None,
            });
        world.containers.push(container);
    }
    world.last_updated = Utc::now();
}

/// Register or update a camera
pub async fn upsert_camera(camera: Camera) {
    let mut world = WORLD.write().await;
    if let Some(existing) = world.cameras.iter_mut().find(|c| c.id == camera.id) {
        *existing = camera;
    } else {
        // Auto-create edge: camera -> NVR
        if let Some(ref nvr_id) = camera.nvr_id {
            let id = camera.id.clone();
            world.edges
                .entry(id)
                .or_default()
                .push(Edge {
                    target: nvr_id.clone(),
                    relation: EdgeRelation::ConnectedTo,
                    metadata: Some("video feed".into()),
                });
        }
        world.cameras.push(camera);
    }
    world.last_updated = Utc::now();
}

/// Register or update an IoT device
pub async fn upsert_iot(device: IoTDevice) {
    let mut world = WORLD.write().await;
    if let Some(existing) = world.iot_devices.iter_mut().find(|d| d.id == device.id) {
        *existing = device;
    } else {
        world.iot_devices.push(device);
    }
    world.last_updated = Utc::now();
}

/// Register or update a VLAN
pub async fn upsert_vlan(vlan: Vlan) {
    let mut world = WORLD.write().await;
    if let Some(existing) = world.vlans.iter_mut().find(|v| v.id == vlan.id) {
        *existing = vlan;
    } else {
        world.vlans.push(vlan);
    }
    world.last_updated = Utc::now();
}

/// Add a directed edge (dependency/relationship) between two entities
pub async fn add_edge(from_id: &str, to_id: &str, relation: EdgeRelation, metadata: Option<String>) {
    let mut world = WORLD.write().await;
    let edges = world.edges.entry(from_id.to_string()).or_default();

    // Prevent duplicate edges
    let exists = edges.iter().any(|e| e.target == to_id && e.relation == relation);
    if !exists {
        edges.push(Edge {
            target: to_id.to_string(),
            relation,
            metadata,
        });
        world.last_updated = Utc::now();
    }
}

/// Remove an edge
pub async fn remove_edge(from_id: &str, to_id: &str, relation: &EdgeRelation) {
    let mut world = WORLD.write().await;
    if let Some(edges) = world.edges.get_mut(from_id) {
        edges.retain(|e| !(e.target == to_id && e.relation == *relation));
        world.last_updated = Utc::now();
    }
}

/// Update machine health score
pub async fn update_machine_health(machine_id: &str, health: f32) {
    let mut world = WORLD.write().await;
    if let Some(m) = world.machines.iter_mut().find(|m| m.id == machine_id) {
        m.health = health.clamp(0.0, 1.0);
        m.last_seen = Utc::now();
    }
}

/// Update service status
pub async fn update_service_status(service_id: &str, status: ServiceStatus) {
    let mut world = WORLD.write().await;
    if let Some(s) = world.services.iter_mut().find(|s| s.id == service_id) {
        s.status = status;
        s.last_checked = Utc::now();
    }
}

// ─── Graph Queries ───

/// Get the dependency chain for an entity — everything it depends on (recursive)
pub async fn get_dependencies(entity_id: &str) -> Vec<AffectedEntity> {
    let world = WORLD.read().await;
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    collect_dependencies(&world, entity_id, &EdgeRelation::DependsOn, 0, &mut visited, &mut result);
    result
}

/// Impact analysis: what breaks if this entity goes down?
/// Walks reverse edges to find everything that depends on this entity.
pub async fn impact_analysis(entity_id: &str) -> ImpactReport {
    let world = WORLD.read().await;

    let source_name = find_entity_name(&world, entity_id)
        .unwrap_or_else(|| entity_id.to_string());

    // Build reverse edge map: target -> list of (source, relation)
    let mut reverse: HashMap<&str, Vec<(&str, &EdgeRelation)>> = HashMap::new();
    for (from, edges) in &world.edges {
        for edge in edges {
            reverse
                .entry(edge.target.as_str())
                .or_default()
                .push((from.as_str(), &edge.relation));
        }
    }

    let mut directly_affected = Vec::new();
    let mut transitively_affected = Vec::new();
    let mut visited = HashSet::new();
    visited.insert(entity_id.to_string());

    // BFS for impact
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((entity_id.to_string(), 0));

    while let Some((current, depth)) = queue.pop_front() {
        if let Some(dependents) = reverse.get(current.as_str()) {
            for (dep_id, relation) in dependents {
                if visited.contains(*dep_id) {
                    continue;
                }
                visited.insert(dep_id.to_string());

                let name = find_entity_name(&world, dep_id)
                    .unwrap_or_else(|| dep_id.to_string());
                let entity_type = find_entity_type(&world, dep_id);

                let affected = AffectedEntity {
                    id: dep_id.to_string(),
                    name,
                    entity_type,
                    relation: format!("{:?}", relation),
                    depth: depth + 1,
                };

                if depth == 0 {
                    directly_affected.push(affected);
                } else {
                    transitively_affected.push(affected);
                }

                queue.push_back((dep_id.to_string(), depth + 1));
            }
        }
    }

    let total = directly_affected.len() + transitively_affected.len();
    let severity = match total {
        0 => ImpactSeverity::None,
        1..=2 => ImpactSeverity::Low,
        3..=5 => ImpactSeverity::Medium,
        6..=10 => ImpactSeverity::High,
        _ => ImpactSeverity::Critical,
    };

    ImpactReport {
        source_id: entity_id.to_string(),
        source_name,
        directly_affected,
        transitively_affected,
        total_impact_count: total,
        severity,
    }
}

/// Collect dependencies recursively (DFS)
fn collect_dependencies(
    world: &WorldModel,
    entity_id: &str,
    relation_filter: &EdgeRelation,
    depth: usize,
    visited: &mut HashSet<String>,
    result: &mut Vec<AffectedEntity>,
) {
    if visited.contains(entity_id) || depth > 20 {
        return;
    }
    visited.insert(entity_id.to_string());

    if let Some(edges) = world.edges.get(entity_id) {
        for edge in edges {
            if &edge.relation == relation_filter || matches!(edge.relation, EdgeRelation::RunsOn) {
                let name = find_entity_name(world, &edge.target)
                    .unwrap_or_else(|| edge.target.clone());
                let entity_type = find_entity_type(world, &edge.target);

                result.push(AffectedEntity {
                    id: edge.target.clone(),
                    name,
                    entity_type,
                    relation: format!("{:?}", edge.relation),
                    depth,
                });

                collect_dependencies(world, &edge.target, relation_filter, depth + 1, visited, result);
            }
        }
    }
}

/// Look up the human-readable name for any entity ID
fn find_entity_name(world: &WorldModel, id: &str) -> Option<String> {
    if let Some(m) = world.machines.iter().find(|m| m.id == id) {
        return Some(m.name.clone());
    }
    if let Some(s) = world.services.iter().find(|s| s.id == id) {
        return Some(s.name.clone());
    }
    if let Some(c) = world.containers.iter().find(|c| c.id == id) {
        return Some(c.name.clone());
    }
    if let Some(c) = world.cameras.iter().find(|c| c.id == id) {
        return Some(c.name.clone());
    }
    if let Some(d) = world.iot_devices.iter().find(|d| d.id == id) {
        return Some(d.name.clone());
    }
    None
}

/// Look up the entity type string for any entity ID
fn find_entity_type(world: &WorldModel, id: &str) -> String {
    if world.machines.iter().any(|m| m.id == id) { return "machine".into(); }
    if world.services.iter().any(|s| s.id == id) { return "service".into(); }
    if world.containers.iter().any(|c| c.id == id) { return "container".into(); }
    if world.cameras.iter().any(|c| c.id == id) { return "camera".into(); }
    if world.iot_devices.iter().any(|d| d.id == id) { return "iot_device".into(); }
    "unknown".into()
}

// ─── Threat & Economic Layers ───

/// Map a threat to an infrastructure entity
pub async fn add_threat_mapping(entity_id: &str, threat_type: &str, severity: &str, description: &str) {
    let mut world = WORLD.write().await;
    let entity_name = find_entity_name(&world, entity_id)
        .unwrap_or_else(|| entity_id.to_string());
    world.threat_layer.push(ThreatMapping {
        entity_id: entity_id.into(), entity_name, threat_type: threat_type.into(),
        severity: severity.into(), description: description.into(),
        detected_at: Utc::now(), mitigated: false,
    });
    world.last_updated = Utc::now();
}

/// Mark a threat mapping as mitigated
pub async fn mitigate_threat_mapping(entity_id: &str, threat_type: &str) {
    let mut world = WORLD.write().await;
    for tm in &mut world.threat_layer {
        if tm.entity_id == entity_id && tm.threat_type == threat_type {
            tm.mitigated = true;
        }
    }
}

/// Get active (unmitigated) threats
pub async fn get_active_threat_mappings() -> Vec<ThreatMapping> {
    WORLD.read().await.threat_layer.iter().filter(|t| !t.mitigated).cloned().collect()
}

/// Set the cost for a service/machine
pub async fn set_service_cost(entity_id: &str, cost_per_month: f64, currency: &str, category: &str, notes: Option<String>) {
    let mut world = WORLD.write().await;
    let entity_name = find_entity_name(&world, entity_id)
        .unwrap_or_else(|| entity_id.to_string());
    // Update existing or add new
    if let Some(sc) = world.economic_layer.iter_mut().find(|s| s.entity_id == entity_id) {
        sc.cost_per_month = cost_per_month;
        sc.currency = currency.into();
        sc.category = category.into();
        sc.notes = notes;
    } else {
        world.economic_layer.push(ServiceCost {
            entity_id: entity_id.into(), entity_name,
            cost_per_month, currency: currency.into(), category: category.into(), notes,
        });
    }
    world.last_updated = Utc::now();
}

/// Get total monthly cost across all services
pub async fn total_monthly_cost() -> f64 {
    WORLD.read().await.economic_layer.iter().map(|s| s.cost_per_month).sum()
}

/// Cross-layer query: entities that are both expensive AND have active threats
pub async fn expensive_threatened_entities() -> Vec<(ServiceCost, ThreatMapping)> {
    let world = WORLD.read().await;
    let mut results = Vec::new();
    for cost in &world.economic_layer {
        for threat in &world.threat_layer {
            if threat.entity_id == cost.entity_id && !threat.mitigated {
                results.push((cost.clone(), threat.clone()));
            }
        }
    }
    results
}

/// Predict the future state of infrastructure based on current health trends
pub async fn predict_state(days_ahead: u32) -> serde_json::Value {
    let world = WORLD.read().await;
    let mut predictions = Vec::new();

    for machine in &world.machines {
        // Project health based on current health and staleness
        let age_hours = (Utc::now() - machine.last_seen).num_hours() as f32;
        let projected_health = (machine.health - age_hours * 0.001 * days_ahead as f32).max(0.0);

        if projected_health < 0.5 {
            predictions.push(serde_json::json!({
                "entity": machine.name,
                "type": "machine",
                "current_health": machine.health,
                "projected_health": projected_health,
                "days_ahead": days_ahead,
                "risk": if projected_health < 0.2 { "critical" } else { "warning" },
            }));
        }
    }

    // Predict service issues from stale checks
    for service in &world.services {
        let hours_since_check = (Utc::now() - service.last_checked).num_hours();
        if hours_since_check > 24 * days_ahead as i64 {
            predictions.push(serde_json::json!({
                "entity": service.name,
                "type": "service",
                "status": format!("{:?}", service.status),
                "hours_unchecked": hours_since_check,
                "risk": "unknown_state",
            }));
        }
    }

    // Economic projection
    let monthly_cost: f64 = world.economic_layer.iter().map(|s| s.cost_per_month).sum();
    let projected_cost = monthly_cost * days_ahead as f64 / 30.0;

    serde_json::json!({
        "days_ahead": days_ahead,
        "at_risk_entities": predictions.len(),
        "predictions": predictions,
        "projected_cost": projected_cost,
        "active_threats": world.threat_layer.iter().filter(|t| !t.mitigated).count(),
    })
}

/// Get a versioned snapshot — the world model tracks a logical version
pub async fn get_version() -> u64 {
    let world = WORLD.read().await;
    world.snapshot_history.len() as u64
}

// ─── Snapshots & Diffs ───

/// Take a snapshot of the current state (for later diff)
pub async fn take_snapshot() {
    let mut world = WORLD.write().await;
    let snapshot = WorldSnapshot {
        taken_at: Utc::now(),
        machine_count: world.machines.len(),
        service_count: world.services.len(),
        container_count: world.containers.len(),
        camera_count: world.cameras.len(),
        iot_count: world.iot_devices.len(),
        machine_ids: world.machines.iter().map(|m| m.id.clone()).collect(),
        service_ids: world.services.iter().map(|s| s.id.clone()).collect(),
    };

    // Keep last 30 snapshots
    if world.snapshot_history.len() >= 30 {
        world.snapshot_history.pop_front();
    }
    world.snapshot_history.push_back(snapshot);
}

/// Get the diff between the current state and the most recent snapshot
pub async fn get_diff() -> Option<WorldDiff> {
    let world = WORLD.read().await;
    let last_snapshot = world.snapshot_history.back()?;

    let current_machine_ids: HashSet<_> = world.machines.iter().map(|m| m.id.clone()).collect();
    let snapshot_machine_ids: HashSet<_> = last_snapshot.machine_ids.iter().cloned().collect();

    let current_service_ids: HashSet<_> = world.services.iter().map(|s| s.id.clone()).collect();
    let snapshot_service_ids: HashSet<_> = last_snapshot.service_ids.iter().cloned().collect();

    let machines_added: Vec<_> = current_machine_ids.difference(&snapshot_machine_ids).cloned().collect();
    let machines_removed: Vec<_> = snapshot_machine_ids.difference(&current_machine_ids).cloned().collect();
    let services_added: Vec<_> = current_service_ids.difference(&snapshot_service_ids).cloned().collect();
    let services_removed: Vec<_> = snapshot_service_ids.difference(&current_service_ids).cloned().collect();

    Some(WorldDiff {
        from: last_snapshot.taken_at,
        to: Utc::now(),
        machines_added,
        machines_removed,
        services_added,
        services_removed,
        health_changes: Vec::new(),
    })
}

// ─── Background Tick ───

/// Called periodically by the consciousness loop to refresh stale data.
/// Marks machines/services as degraded if not seen recently.
pub async fn tick() {
    let mut world = WORLD.write().await;
    let now = Utc::now();
    let stale_threshold_secs = 300; // 5 minutes without heartbeat = stale

    for machine in &mut world.machines {
        let age_secs = (now - machine.last_seen).num_seconds();
        if age_secs > stale_threshold_secs && machine.health > 0.5 {
            machine.health = (machine.health - 0.1).max(0.0);
        }
    }

    for service in &mut world.services {
        let age_secs = (now - service.last_checked).num_seconds();
        if age_secs > stale_threshold_secs && service.status == ServiceStatus::Healthy {
            service.status = ServiceStatus::Unknown;
        }
    }

    for container in &mut world.containers {
        let age_secs = (now - container.last_seen).num_seconds();
        if age_secs > stale_threshold_secs && container.status == ContainerStatus::Running {
            container.status = ContainerStatus::Unknown;
        }
    }
}

// ─── Persistence ───

/// Persist the world model to PostgreSQL
pub async fn persist() {
    let world = get_world().await;
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&world) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize world model: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO world_model_store (id, model, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET model = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

/// Restore world model from PostgreSQL (called on boot)
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT model FROM world_model_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((model_json,)) = row {
            if let Ok(restored) = serde_json::from_value::<WorldModel>(model_json) {
                let machine_count = restored.machines.len();
                let service_count = restored.services.len();
                let container_count = restored.containers.len();

                let mut world = WORLD.write().await;
                *world = restored;

                tracing::info!(
                    machines = machine_count,
                    services = service_count,
                    containers = container_count,
                    "World model restored from database"
                );
            }
        }
    }
}

/// Create the world_model_store table if it does not exist
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS world_model_store (\
                id INTEGER PRIMARY KEY, \
                model JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        )
        .execute(pool)
        .await;
    }
}

// ─── API Helpers ───

/// Get world model formatted for API response
pub async fn get_for_api() -> serde_json::Value {
    let world = get_world().await;
    serde_json::json!({
        "machines": world.machines.len(),
        "services": world.services.len(),
        "containers": world.containers.len(),
        "cameras": world.cameras.len(),
        "iot_devices": world.iot_devices.len(),
        "vlans": world.vlans.len(),
        "people": world.people.len(),
        "projects": world.projects.len(),
        "edges": world.edges.len(),
        "last_scan": world.last_scan,
        "last_updated": world.last_updated,
        "model": world,
    })
}

/// Get a human-readable summary (for consciousness layer)
pub async fn summary() -> String {
    let world = get_world().await;
    let healthy_machines = world.machines.iter().filter(|m| m.health > 0.7).count();
    let healthy_services = world.services.iter().filter(|s| s.status == ServiceStatus::Healthy).count();
    let running_containers = world.containers.iter().filter(|c| c.status == ContainerStatus::Running).count();

    let mut lines = Vec::new();
    lines.push(format!(
        "Infrastructure: {} machines ({} healthy), {} services ({} healthy), {} containers ({} running)",
        world.machines.len(), healthy_machines,
        world.services.len(), healthy_services,
        world.containers.len(), running_containers,
    ));
    lines.push(format!(
        "IoT: {} cameras, {} devices across {} VLANs",
        world.cameras.len(), world.iot_devices.len(), world.vlans.len(),
    ));
    if let Some(scan) = world.last_scan {
        let age_mins = (Utc::now() - scan).num_minutes();
        lines.push(format!("Last scan: {}m ago", age_mins));
    }
    lines.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_world_model() {
        let world = WorldModel::default();
        assert!(world.machines.is_empty());
        assert!(world.services.is_empty());
        assert!(world.edges.is_empty());
    }

    #[test]
    fn test_machine_serialization() {
        let machine = Machine {
            id: "m_1".into(),
            name: "truenas".into(),
            role: MachineRole::Nas,
            ip: Some("10.0.50.20".into()),
            mac: None,
            os: Some("TrueNAS SCALE".into()),
            cpu: Some("Intel i5-12400".into()),
            ram_gb: Some(32),
            disk_gb: Some(8000),
            services: vec!["smb".into(), "nfs".into()],
            vlan_id: Some("vlan_50".into()),
            last_seen: Utc::now(),
            health: 0.95,
            tags: vec!["storage".into()],
            notes: None,
        };
        let json = serde_json::to_string(&machine).unwrap();
        assert!(json.contains("truenas"));
        assert!(json.contains("Nas"));
    }

    #[test]
    fn test_edge_relation_equality() {
        assert_eq!(EdgeRelation::DependsOn, EdgeRelation::DependsOn);
        assert_ne!(EdgeRelation::DependsOn, EdgeRelation::Hosts);
    }

    #[test]
    fn test_impact_severity() {
        // Just validate enum variants
        let s = ImpactSeverity::Critical;
        assert_eq!(s, ImpactSeverity::Critical);
    }

    #[test]
    fn test_find_entity_name() {
        let mut world = WorldModel::default();
        world.machines.push(Machine {
            id: "m_test".into(),
            name: "TestMachine".into(),
            role: MachineRole::Server,
            ip: None,
            mac: None,
            os: None,
            cpu: None,
            ram_gb: None,
            disk_gb: None,
            services: vec![],
            vlan_id: None,
            last_seen: Utc::now(),
            health: 1.0,
            tags: vec![],
            notes: None,
        });
        assert_eq!(find_entity_name(&world, "m_test"), Some("TestMachine".into()));
        assert_eq!(find_entity_name(&world, "nonexistent"), None);
    }

    #[test]
    fn test_find_entity_type() {
        let mut world = WorldModel::default();
        world.services.push(ServiceNode {
            id: "s_test".into(),
            name: "TestSvc".into(),
            service_type: ServiceType::Api,
            url: None,
            port: Some(8080),
            host_machine_id: None,
            status: ServiceStatus::Healthy,
            last_checked: Utc::now(),
            version: None,
            tags: vec![],
        });
        assert_eq!(find_entity_type(&world, "s_test"), "service");
        assert_eq!(find_entity_type(&world, "nonexistent"), "unknown");
    }
}
