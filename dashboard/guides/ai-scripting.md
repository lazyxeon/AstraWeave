# AI Scripting (Rhai)

Checked against `main` at `3971cbe1` (#318).

Gameplay scripts run on `script_system` (`astraweave-scripting`). They emit `ScriptCommands`. They do not receive `PhysicsProxy` or `NavMeshProxy`. A script that names `physics` or `nav` errors, and the statement after that call does not run. The host can still read `PhysicsWorld` for collision events and for bodies a command spawned. The trace is [scripting.md](../architecture/scripting.md).

Two examples changed with that tick:

- `examples/scripting_advanced_demo` still inserts `PhysicsWorld` and `NavMesh`. `assets/advanced_ai.rhai` does not call them. Sight is distance under 20. A finished patrol point sets `path` to an empty array.
- `examples/scripting_playground` still builds a physics world, a dynamic box, and a sensor. The player script handles `on_trigger_enter` and `commands.apply_damage`. It does not raycast.

Encounter, boss, and persona scripts, where they exist, are a different surface: a read-only snapshot in, allowed verbs out, and the deterministic sim validates the result. Do not treat `docs/src/game-dev/scripting.md` as this API. That page describes callbacks and a `raycast` function the gameplay tick does not provide.