# RedNode-OS Developer SDK Specification

> How to extend RedNode without modifying the core.

## Extension Points

Developers can create:

1. **Cognitive Modules** — via the cognitive bus event system
2. **Planning Strategies** — via meta_reasoning strategy registration
3. **Debate Roles** — via the DebateRole enum extension
4. **Sensors/Perceivers** — via the perception layer
5. **Tools** — via the evolution engine tool registration
6. **Simulators** — via the simulation engine
7. **Reflection Strategies** — via the reflection system
8. **Plugins** — via the plugin manifest system

## Cognitive Bus Integration

External modules communicate through the cognitive bus:

```rust
// Subscribe to events
let mut rx = cognitive_bus::subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        match event.event_type {
            CognitiveEventType::TaskCompleted => { /* react */ }
            CognitiveEventType::ThreatDetected => { /* react */ }
            _ => {}
        }
    }
});

// Emit events
cognitive_bus::emit(
    CognitiveEventType::ObservationCreated,
    "my_module",
    json!({"data": "observation"}),
).await;
```

## Plugin Manifest

```json
{
    "name": "my-plugin",
    "version": "1.0.0",
    "description": "A custom RedNode plugin",
    "author": "developer",
    "agent": "custom-agent",
    "tools": [
        {
            "name": "custom.action",
            "description": "Does something custom",
            "risk": "low",
            "handler_type": "shell"
        }
    ],
    "config_schema": {},
    "permissions": ["network", "notifications"],
    "entry_point": "src/index.ts",
    "lifecycle": "Developing"
}
```

## Tool Registration

```bash
curl -X POST http://localhost:8787/evolve/tool \
  -H "Content-Type: application/json" \
  -d '{
    "name": "custom.hello",
    "agent": "system-agent",
    "description": "Say hello",
    "handler_type": "shell",
    "handler_command": "echo hello"
  }'
```

The evolution engine will:
1. Validate the tool definition
2. Check constitution compliance
3. Run sandbox tests
4. Check budget
5. Register in tools.json
6. Inject handler into the agent

## Safety Guarantees

All extensions operate under:
- Constitutional articles (immutable)
- Governance policies (configurable)
- Ethics values (guiding)
- Sandboxed execution (firejail/seccomp)
- Audit chain (SHA-256)
- Trust scoring (dynamic)
