# Persona Packs

**Manifest**: `persona_manifest.toml`  
**Contents**: profile JSON/TOML + voice map + facts/episodes.

Validation:
- Hashes of content files
- Required keys: `id`, `name`, `skills`, `voice_map[]`

Loading:
- `persona_loader` example demonstrates how packs mount into profile DB.