# Build & Packaging

Use `aw_build`:

```bash
# Bundle binaries + assets
cargo run -p aw_build -- bundle --name astraweave-${TARGET} --version 0.4.0 \
  --bin_dir target/release --assets assets --out dist

# Create delta patch between two manifests
cargo run -p aw_build -- patch \
  --old_manifest dist/astraweave-${TARGET}-0.4.0.manifest.json \
  --new_manifest dist/astraweave-${TARGET}-0.4.1.manifest.json \
  --out dist --name patch-0.4.0-to-0.4.1
```

## Outputs

- `${name}-${version}.manifest.json` (hashes, sizes, timestamp)
- `${name}-${version}.zip` (bin + assets)
- `patch-*.zip` (changed files only)