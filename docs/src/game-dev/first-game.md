# Building Your First Game

There is no `companion_quest` package and no `examples/companion_quest` directory. `cargo run -p companion_quest` fails.

Start from binaries that are in the workspace:

```bash
cargo run -p hello_companion --release
cargo run -p quest_dialogue_demo --release
cargo run -p combat_physics_demo --release
cargo run -p adaptive_boss --release
cargo run -p aravath_runtime --bin aravath_greybox --release
```

Rust is 1.89.0, pinned by `rust-toolchain.toml`. The default companion run does not connect to Ollama. LLM modes need the feature flags on the [Hello Companion](../examples/hello-companion.md) page.

The greybox is the windowed slice. Fetch assets before opening it:

```bash
cargo xtask fetch-assets
cargo run -p aravath_runtime --bin aravath_greybox --release
```

A previous version of this page printed a sample log (`AstraWeave v0.1.0`, Rapier 0.17, Hermes, a companion named Spark). That log is not the output of a current binary. Rapier in the workspace is 0.22. The orchestrator default model is `phi3:medium`.

## Related

- [AI companions](./companions.md)
- [Crafting and combat](./crafting-combat.md)
- [Dialogue](./dialogue.md)
- [Save and load](./save-load.md)
- [Examples](../examples/index.md)
