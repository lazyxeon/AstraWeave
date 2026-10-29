//! Ollama `format` schema for one agent's tool registry.
//!
//! The schema is a propose-side constraint. [`crate::parse_llm_plan`] and
//! [`crate::plan_parser::parse_llm_response`] still reject a step whose tool
//! is not in the registry.

use astraweave_core::{ToolRegistry, ToolSpec};
use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::OnceLock;

static PLAN_FORMAT_CACHE: OnceLock<DashMap<String, Value>> = OnceLock::new();

/// JSON Schema for a `PlanIntent` whose `act` values are `reg`'s tool names.
///
/// Cached by a length-prefixed key of tool names and argument types, in
/// registry order. Argument maps are `BTreeMap`s, so the key does not depend
/// on hash order. The length prefix keeps `A{}|B` distinct from tools `A` and `B`.
pub fn plan_format_schema(reg: &ToolRegistry) -> Value {
    let key = registry_cache_key(reg);
    let cache = PLAN_FORMAT_CACHE.get_or_init(DashMap::new);
    if let Some(hit) = cache.get(&key) {
        return hit.clone();
    }
    let schema = build_plan_format_schema(reg);
    cache.insert(key, schema.clone());
    schema
}

/// Schema to send as Ollama `format`, or `None` when the registry names no tool.
///
/// A named registry requires `steps.minItems` of 1. A registry with no named
/// tools leaves `format` unset, so the request does not force an empty plan.
pub fn plan_format_for_request(reg: &ToolRegistry) -> Option<Value> {
    if reg.tools.iter().any(|tool| !tool.name.is_empty()) {
        Some(plan_format_schema(reg))
    } else {
        None
    }
}

/// Put `schema` on an Ollama request body as `format`.
pub fn insert_schema_format(body: &mut Value, schema: &Value) {
    body["format"] = schema.clone();
}

/// Remove an object `format`. A string such as `"json"` stays.
///
/// Returns whether an object was removed, so the caller retries once.
pub fn take_schema_format(body: &mut Value) -> bool {
    match body.get("format") {
        Some(value) if value.is_object() => {
            if let Some(map) = body.as_object_mut() {
                map.remove("format");
            }
            true
        }
        _ => false,
    }
}

/// `true` when Ollama rejected the JSON Schema in `format`.
///
/// A 400 that merely mentions the word format, such as a model-file error,
/// stays a hard failure. A match is logged before the caller retries once
/// without the schema object.
pub fn schema_format_rejected(status: u16, body: &str) -> bool {
    if status != 400 {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    let rejected = lower.contains("invalid format")
        || lower.contains("unsupported format")
        || lower.contains("unknown format")
        || lower.contains("format not supported")
        || lower.contains("does not support format")
        || lower.contains("failed to parse format")
        || lower.contains("invalid json schema")
        || lower.contains("grammar compilation");
    if rejected {
        let shown = if body.len() <= 180 {
            body
        } else {
            let mut end = 180;
            while end > 0 && !body.is_char_boundary(end) {
                end -= 1;
            }
            &body[..end]
        };
        tracing::warn!(
            status,
            body = shown,
            "Ollama rejected the JSON schema format; retrying without it"
        );
    }
    rejected
}

/// Batch schema: exactly `count` plans, each using `reg`'s tools.
///
/// `None` when `count` is 0 or the registry names no tool.
pub fn batch_format_schema(reg: &ToolRegistry, count: usize) -> Option<Value> {
    if count == 0 {
        return None;
    }
    let plan = plan_format_for_request(reg)?;
    let steps = plan.get("properties")?.get("steps")?.clone();
    Some(json!({
        "type": "array",
        "minItems": count,
        "maxItems": count,
        "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["agent_id", "plan_id", "steps"],
            "properties": {
                "agent_id": { "type": "integer" },
                "plan_id": { "type": "string" },
                "steps": steps
            }
        }
    }))
}

/// Set `format` to `schema`, or to `"json"` when `force_json` and there is no schema.
pub fn apply_chat_format(body: &mut Value, schema: Option<&Value>, force_json: bool) {
    if let Some(schema) = schema {
        insert_schema_format(body, schema);
    } else if force_json {
        body["format"] = json!("json");
    }
}

fn push_prefixed(key: &mut String, value: &str) {
    key.push_str(&value.len().to_string());
    key.push(':');
    key.push_str(value);
}

fn registry_cache_key(reg: &ToolRegistry) -> String {
    let mut key = String::new();
    for tool in &reg.tools {
        push_prefixed(&mut key, &tool.name);
        for (arg, ty) in &tool.args {
            push_prefixed(&mut key, arg);
            push_prefixed(&mut key, ty);
        }
        push_prefixed(&mut key, "\n");
    }
    key
}

fn build_plan_format_schema(reg: &ToolRegistry) -> Value {
    let mut seen = Vec::new();
    let mut branches = Vec::new();
    for tool in &reg.tools {
        if tool.name.is_empty() || seen.iter().any(|name: &String| name == &tool.name) {
            continue;
        }
        seen.push(tool.name.clone());
        branches.push(tool_branch(tool));
    }

    let steps = if branches.is_empty() {
        json!({ "type": "array", "maxItems": 0 })
    } else {
        json!({
            "type": "array",
            "minItems": 1,
            "items": { "oneOf": branches }
        })
    };

    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["plan_id", "steps"],
        "properties": {
            "plan_id": { "type": "string" },
            "steps": steps
        }
    })
}

fn tool_branch(tool: &ToolSpec) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "act".to_string(),
        json!({ "type": "string", "enum": [tool.name.as_str()] }),
    );
    let mut required = vec!["act".to_string()];
    for (name, ty) in &tool.args {
        let optional = is_optional_type(ty);
        properties.insert(name.clone(), arg_json_schema(ty));
        if !optional {
            required.push(name.clone());
        }
    }
    match_parser_fields(&tool.name, &mut properties, &mut required);
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": required,
        "properties": Value::Object(properties)
    })
}

/// Fields the plan parser accepts that a short registry entry omits or misspells.
///
/// `MovementSpeed` and `StrafeDirection` deserialize as lowercase. `MoveTo.speed`
/// is optional (`#[serde(default)]`). `Strafe` and `Dodge` require `direction`.
fn match_parser_fields(
    name: &str,
    properties: &mut serde_json::Map<String, Value>,
    required: &mut Vec<String>,
) {
    match name {
        "MoveTo" => {
            properties.insert(
                "speed".to_string(),
                json!({ "type": "string", "enum": ["walk", "run", "sprint"] }),
            );
            required.retain(|field| field != "speed");
        }
        "Strafe" | "Dodge" => {
            properties.insert(
                "direction".to_string(),
                json!({ "type": "string", "enum": ["left", "right"] }),
            );
            if !required.iter().any(|field| field == "direction") {
                required.push("direction".to_string());
            }
        }
        _ => {}
    }
}

fn is_optional_type(ty: &str) -> bool {
    let ty = ty.trim();
    ty.starts_with("Option<") && ty.ends_with('>')
}

fn arg_json_schema(ty: &str) -> Value {
    let ty = ty.trim();
    if let Some(inner) = ty
        .strip_prefix("Option<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        return arg_json_schema(inner);
    }
    if let Some(inner) = ty
        .strip_prefix("enum[")
        .and_then(|rest| rest.strip_suffix(']'))
    {
        let variants: Vec<String> = inner
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_ascii_lowercase)
            .collect();
        return json!({ "type": "string", "enum": variants });
    }
    if let Some(inner) = ty
        .strip_prefix("Vec<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        return json!({ "type": "array", "items": arg_json_schema(inner) });
    }
    match ty.to_ascii_lowercase().as_str() {
        "i32" | "i64" | "u32" | "u64" | "entity" | "int" | "integer" => {
            json!({ "type": "integer" })
        }
        "f32" | "f64" | "float" | "number" => json!({ "type": "number" }),
        "bool" | "boolean" => json!({ "type": "boolean" }),
        "ivec2" => json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["x", "y"],
            "properties": {
                "x": { "type": "integer" },
                "y": { "type": "integer" }
            }
        }),
        _ => json!({ "type": "string" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{Constraints, ToolSpec};
    use std::collections::BTreeMap;

    fn tool(name: &str, args: &[(&str, &str)]) -> ToolSpec {
        ToolSpec {
            name: name.to_string(),
            args: args
                .iter()
                .map(|(key, ty)| ((*key).to_string(), (*ty).to_string()))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    fn example_value(schema: &Value) -> Value {
        if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
            return branches
                .first()
                .map(example_value)
                .unwrap_or_else(|| json!({}));
        }
        if let Some(variants) = schema.get("enum").and_then(Value::as_array) {
            return variants.first().cloned().unwrap_or_else(|| json!(""));
        }
        match schema.get("type").and_then(Value::as_str) {
            Some("object") => {
                let mut object = serde_json::Map::new();
                if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                    for (name, property) in properties {
                        object.insert(name.clone(), example_value(property));
                    }
                }
                Value::Object(object)
            }
            Some("array") => {
                if schema.get("maxItems") == Some(&json!(0)) {
                    return json!([]);
                }
                let item = schema
                    .get("items")
                    .map(example_value)
                    .unwrap_or_else(|| json!({}));
                json!([item])
            }
            Some("integer") => json!(1),
            Some("number") => json!(1.0),
            Some("boolean") => json!(true),
            _ => json!("x"),
        }
    }

    fn registry(tools: Vec<ToolSpec>) -> ToolRegistry {
        ToolRegistry {
            tools,
            constraints: Constraints {
                enforce_cooldowns: true,
                enforce_los: true,
                enforce_stamina: true,
            },
        }
    }

    #[test]
    fn schema_lists_only_registry_tools() {
        let reg = registry(vec![
            tool("MoveTo", &[("x", "i32"), ("y", "i32")]),
            tool(
                "Throw",
                &[("item", "enum[smoke,grenade]"), ("x", "i32"), ("y", "i32")],
            ),
        ]);
        let schema = plan_format_schema(&reg);
        let again = plan_format_schema(&reg);
        assert_eq!(schema, again);
        assert_eq!(schema["properties"]["steps"]["minItems"], 1);

        let branches = schema["properties"]["steps"]["items"]["oneOf"]
            .as_array()
            .expect("oneOf");
        assert_eq!(branches.len(), 2);
        assert_eq!(
            branches[0]["properties"]["act"]["enum"][0].as_str(),
            Some("MoveTo")
        );
        assert_eq!(
            branches[0]["properties"]["x"]["type"].as_str(),
            Some("integer")
        );
        assert_eq!(
            branches[1]["properties"]["item"]["enum"]
                .as_array()
                .map(|items| items.len()),
            Some(2)
        );

        let rendered = schema.to_string();
        assert!(rendered.contains("MoveTo"));
        assert!(rendered.contains("Throw"));
        assert!(!rendered.contains("CoverFire"));
        assert!(!rendered.contains("Attack"));
        assert_eq!(plan_format_for_request(&reg).as_ref(), Some(&schema));
    }

    #[test]
    fn schema_skips_duplicate_and_empty_names() {
        let reg = registry(vec![
            tool("Scan", &[("radius", "f32")]),
            tool("Scan", &[("radius", "f32")]),
            tool("", &[]),
            tool("Wait", &[("duration", "Option<f32>")]),
        ]);
        let schema = plan_format_schema(&reg);
        let branches = schema["properties"]["steps"]["items"]["oneOf"]
            .as_array()
            .expect("oneOf");
        assert_eq!(branches.len(), 2);
        let wait = &branches[1];
        let required = wait["required"].as_array().expect("required");
        assert!(required.iter().all(|item| item != "duration"));
        assert_eq!(
            wait["properties"]["duration"]["type"].as_str(),
            Some("number")
        );
    }

    #[test]
    fn empty_registry_is_not_sent() {
        let reg = registry(vec![]);
        let schema = plan_format_schema(&reg);
        assert_eq!(schema["properties"]["steps"]["maxItems"], 0);
        assert!(plan_format_for_request(&reg).is_none());
    }

    #[test]
    fn schema_rejection_detects_a_format_400() {
        assert!(schema_format_rejected(400, "invalid format schema"));
        assert!(schema_format_rejected(
            400,
            "unsupported format: json schema"
        ));
        assert!(!schema_format_rejected(400, "model not found"));
        assert!(!schema_format_rejected(
            400,
            "the model file format is gguf"
        ));
        assert!(!schema_format_rejected(500, "invalid format"));
    }

    #[test]
    fn cache_key_does_not_merge_distinct_tool_lists() {
        let one = registry(vec![tool("A{}|B", &[])]);
        let two = registry(vec![tool("A", &[]), tool("B", &[])]);
        let first = plan_format_schema(&one);
        let second = plan_format_schema(&two);
        assert_ne!(first, second);
        assert_eq!(plan_format_schema(&one), first);
        assert_eq!(plan_format_schema(&two), second);
    }

    #[test]
    fn generated_schema_instances_parse() {
        let reg = registry(vec![
            tool("MoveTo", &[("x", "i32"), ("y", "i32")]),
            tool(
                "Strafe",
                &[("target_id", "Entity"), ("direction", "enum[Left,Right]")],
            ),
        ]);
        let schema = plan_format_schema(&reg);
        let branches = schema["properties"]["steps"]["items"]["oneOf"]
            .as_array()
            .expect("oneOf");
        let strafe = branches
            .iter()
            .find(|branch| branch["properties"]["act"]["enum"][0] == "Strafe")
            .expect("strafe");
        assert_eq!(
            strafe["properties"]["direction"]["enum"],
            json!(["left", "right"])
        );
        let move_to = branches
            .iter()
            .find(|branch| branch["properties"]["act"]["enum"][0] == "MoveTo")
            .expect("move");
        assert_eq!(
            move_to["properties"]["speed"]["enum"],
            json!(["walk", "run", "sprint"])
        );
        let required = move_to["required"].as_array().expect("required");
        assert!(required.iter().all(|field| field != "speed"));

        let steps: Vec<Value> = branches.iter().map(example_value).collect();
        let plan = json!({ "plan_id": "generated", "steps": steps });
        let text = serde_json::to_string(&plan).expect("plan json");
        let parsed = crate::parse_llm_plan(&text, &reg).expect("schema instance parses");
        assert_eq!(parsed.steps.len(), 2);
        assert!(text.contains("\"left\""));
        assert!(text.contains("\"walk\""));
        assert!(!text.contains("\"Left\""));
    }

    #[test]
    fn take_schema_format_removes_only_an_object() {
        let mut body = json!({"format": {"type": "object"}, "model": "m"});
        assert!(take_schema_format(&mut body));
        assert!(body.get("format").is_none());
        let mut json_mode = json!({"format": "json"});
        assert!(!take_schema_format(&mut json_mode));
        assert_eq!(json_mode["format"], "json");
    }

    #[test]
    fn out_of_registry_tool_is_rejected_by_both_validators() {
        let reg = registry(vec![tool("MoveTo", &[("x", "i32"), ("y", "i32")])]);
        let attack = r#"{"plan_id":"p","steps":[{"act":"Attack","target_id":7}]}"#;

        let parsed = crate::parse_llm_plan(attack, &reg);
        let message = parsed.expect_err("parse_llm_plan").to_string();
        assert!(message.contains("disallowed tool Attack"), "{message}");

        let response = crate::plan_parser::parse_llm_response(attack, &reg);
        let message = response.expect_err("parse_llm_response").to_string();
        assert!(
            message.contains("Attack") && message.contains("not in registry"),
            "{message}"
        );

        let allowed = r#"{"plan_id":"p","steps":[{"act":"MoveTo","x":1,"y":2}]}"#;
        assert!(crate::parse_llm_plan(allowed, &reg).is_ok());
        assert!(crate::plan_parser::parse_llm_response(allowed, &reg).is_ok());
    }
}
