---
layout: default
title: SDK (C ABI)
---

# C ABI SDK (astraweave-sdk)

AstraWeave exposes a stable C ABI for embedding the engine in non-Rust applications (C, C++, C#, Python, etc.). The crate has **70 tests** (17 unit + 53 integration) and **8 Kani formal verification proofs**.

## Features

- **cbindgen-generated header**: Automatic `.h` generation from Rust source (cbindgen 0.29)
- **Opaque handles**: Engine objects represented as `#[repr(C)]` structs
- **Error codes**: Integer return values instead of exceptions
- **Miri-validated**: 17 tests confirming FFI safety (zero UB)
- **Kani-verified**: 8 formal proofs for version string safety and struct layout
- **Triple library target**: `rlib` + `cdylib` + `staticlib`

## Types

### Rust-Side Types

```rust
#[non_exhaustive]
#[must_use]
pub enum SdkError {
    Schema(String),
    Io(std::io::Error),
}

pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

pub trait GameAdapter {
    fn version(&self) -> Version;
}
```

### C ABI Types

```c
// #[repr(C)] — 6 bytes, 2-byte aligned
typedef struct AWVersion {
    uint16_t major;
    uint16_t minor;
    uint16_t patch;
} AWVersion;

// Opaque world handle wrapping internal AwWorldWrap
typedef struct AWWorld {
    void* _inner;  // *mut AwWorldWrap
} AWWorld;

// String callback for async results
typedef void (*CStringCallback)(const char*);
```

### Error Codes

| Constant | Value | Meaning |
|----------|-------|---------|
| `AW_OK` | `0` | Success |
| `AW_ERR_NULL` | `-1` | Null pointer argument |
| `AW_ERR_PARAM` | `-2` | Invalid parameter |
| `AW_ERR_PARSE` | `-3` | JSON parse failure |
| `AW_ERR_EXEC` | `-4` | Execution error |

## Exported Functions (10 total)

### Version API

```c
// Get engine version as struct
AWVersion ver = aw_version();

// Get version as null-terminated string
char buf[32];
size_t len = aw_version_string(buf, sizeof(buf));  // unsafe
```

### World Lifecycle

```c
// Create world instance
AWWorld world = aw_world_create();

// Run one tick
aw_world_tick(world, delta_time);

// Destroy
aw_world_destroy(world);
```

### World Interaction

```c
// Set snapshot callback (called each tick with JSON)
aw_world_set_snapshot_callback(world, my_snapshot_handler);

// Set delta callback (called on state changes)
aw_world_set_delta_callback(world, my_delta_handler);

// Get snapshot as JSON into buffer
size_t written = aw_world_snapshot_json(world, buf, buf_len);

// Submit AI intent as JSON
int32_t result = aw_world_submit_intent_json(  // unsafe
    world, actor_id, intent_json_cstr, result_callback
);
```

### Error Handling

```c
// Get last error message
char err_buf[256];
size_t err_len = aw_last_error_string(err_buf, sizeof(err_buf));
```

### Function Summary

| Function | Safety | Description |
|----------|--------|-------------|
| `aw_version` | safe | Returns `AWVersion` struct |
| `aw_version_string` | unsafe | Writes version string into buffer |
| `aw_world_create` | safe | Creates new world instance |
| `aw_world_destroy` | safe | Destroys world |
| `aw_world_tick` | safe | Advances simulation by dt |
| `aw_world_set_snapshot_callback` | safe | Sets snapshot listener |
| `aw_world_set_delta_callback` | safe | Sets delta listener |
| `aw_world_snapshot_json` | safe | Serializes world state to JSON |
| `aw_world_submit_intent_json` | unsafe | Submits AI intent from C string |
| `aw_last_error_string` | safe | Retrieves last error message |

## Safety Guarantees

- All exported functions check for null pointers before dereferencing
- Handle validation prevents use-after-free
- No panics cross the FFI boundary (all caught and converted to error codes)
- Thread safety documented per function
- `#[repr(C)]` layout verified by Kani proofs

## Formal Verification

### Miri (17 tests, 0 UB)

- Raw pointer construction and dereferencing
- C ABI calling convention correctness
- Handle lifecycle (create → use → destroy)
- Error path coverage (null, invalid, parse)

### Kani (8 proofs)

| Proof | Verifies |
|-------|----------|
| `version_returns_valid_struct` | Version fields within reasonable ranges |
| `version_string_null_buffer_returns_size` | Null buf returns required size (6–50 bytes) |
| `version_string_no_buffer_overflow` | No writes past buffer end (symbolic len ≤ 30) |
| `version_string_writes_valid_utf8` | Output is valid ASCII |
| `version_string_format` | String contains ≥ 2 dots (x.y.z format) |
| `aw_version_struct_layout` | `AWVersion` is exactly 6 bytes, 2-byte aligned |
| `version_consistency` | At least one non-zero version component |
| `version_string_minimum_buffers` | Buffers of size 1, 4, 8 are null-terminated |

## Dependencies

| Crate | Purpose |
|-------|---------|
| `astraweave-core` | Core engine types |
| `serde` / `serde_json` | JSON serialization for snapshots |
| `anyhow` | Error handling |
| `thiserror` | Typed error variants |

## Building

```bash
# Build the shared library + static library + header
cargo build -p astraweave-sdk

# Output: target/debug/libastraweave_sdk.so (or .dll / .dylib)
# Header: target/astraweave_sdk.h (via cbindgen)
```

[← Back to Home](index.html) · [Architecture](architecture.html) · [Crate Index](crates.html)
