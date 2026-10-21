# Tool Validation System

The AstraWeave LLM integration includes a comprehensive tool validation system that ensures LLM-generated plans are safe and comply with the game's allowed actions.

## Overview

The tool validation system works in three layers:

1. **Schema Validation**: Ensures JSON structure matches expected format
2. **Tool Registry Validation**: Verifies all actions are in the allowed tool set
3. **Engine Validation**: Runtime checks for cooldowns, line-of-sight, and other constraints

## Components

### ToolRegistry

The `ToolRegistry` defines which tools are available to the LLM:

```rust
pub struct ToolRegistry {
    pub tools: Vec<ToolSpec>,
    pub constraints: Constraints,
}

pub struct ToolSpec {
    pub name: String,
    pub args: BTreeMap<String, String>, // argument name -> type
}
```

### Validation Process

1. **LLM Response Parsing**: Parse the JSON response into a `PlanIntent`
2. **Tool Allowlist Check**: Verify each action is in the registry
3. **Type Validation**: Ensure arguments match expected types (future enhancement)
4. **Runtime Validation**: Engine checks constraints during execution

### Supported Actions

- **MoveTo**: Move companion to specified coordinates
- **Throw**: Throw items (smoke, grenade) at target location  
- **CoverFire**: Provide covering fire at target for duration
- **Revive**: Revive allied units

### Error Handling

The system provides clear error messages for validation failures:

- Invalid JSON format
- Disallowed tools used by LLM
- Missing required arguments
- Type mismatches (future)

### Security Features

- **Allowlist-only**: Only pre-approved actions can be executed
- **No dynamic code execution**: All actions are statically defined
- **Input sanitization**: JSON parsing prevents injection attacks
- **Constraint enforcement**: Runtime validation prevents illegal moves

## Usage Example

```rust
use astraweave_llm::{parse_llm_plan, MockLlm, plan_from_llm};

// Create tool registry
let registry = ToolRegistry {
    tools: vec![
        ToolSpec {
            name: "move_to".into(),
            args: [("x", "i32"), ("y", "i32")].into_iter().collect(),
        }
    ],
    constraints: Constraints::default(),
};

// Parse and validate LLM response
let plan = parse_llm_plan(llm_response, &registry)?;

// Full end-to-end validation
let plan = plan_from_llm(&client, &world_snapshot, &registry).await?;
```

## Testing

The validation system includes comprehensive tests covering:

- Valid plan parsing
- Invalid JSON handling
- Disallowed tool detection
- Empty plan handling
- All action types
- Error message verification

Run tests with:
```bash
cargo test -p astraweave-llm
```

## Future Enhancements

- Argument type validation
- Parameter range checking
- Cost/resource validation
- Complex constraint evaluation
- Custom validation plugins
