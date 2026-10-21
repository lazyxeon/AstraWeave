# Authoring Tools

- **Level & Encounter Editor** (`aw_editor`): Save `.level.toml` for geometry, NPC spawns, & fate threads.
- **Rhai scripts**: budgets/phases hot-reload on save.
- **Asset CLI** (`aw_asset_cli`): Converts textures/models/audio, writes `manifest.json`.

Recommended workflow:
1. Edit level in `aw_editor` → saves into `content/levels/`.
2. Update encounter scripts → hot-reload.
3. Run `aw_asset_cli` to stage assets for build packaging.