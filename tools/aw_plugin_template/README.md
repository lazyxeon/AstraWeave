# aw_plugin_template

This directory is a **non-runtime scaffold**. The source uses `{{ProjectName}}` placeholders and does not typecheck as a loadable AstraWeave mod.

Mods are data: a `mod.toml` plus `.rhai` scripts loaded by `astraweave-security::sandbox` and executed by `astraweave-scripting::sim_mod`. They are not native crates generated from this template. The public command set is `astraweave_security::sandbox::COMMANDS` at `MOD_COMMAND_API_VERSION`.
