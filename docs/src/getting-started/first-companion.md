# Your First AI Companion

The in-tree companion binary is `hello_companion`. Run it before writing a new crate.

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
cargo run -p hello_companion --release
```

Rust is 1.89.0, pinned by `rust-toolchain.toml`. The default run is headless `RuleOrchestrator`. It does not need a GPU or Ollama, and it does not panic with `LosBlocked`.

Mode flags, feature gates, and what `--arbiter` actually calls are on the [Hello Companion](../examples/hello-companion.md) page.

This page used to walk through `cargo new my_first_companion` and types that are not in the current crates. Those snippets are removed. Copy `examples/hello_companion` when you want a new binary, and add the directory to the workspace `members` list.

## Related

- [Quick start](./quick-start.md)
- [Installation](./installation.md)
- [AI system](../core-systems/ai/index.md)
- [Engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md)
