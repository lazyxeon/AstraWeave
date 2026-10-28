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
/// Cached by a stable key of tool names and argument types, in registry order.
/// Argument maps are `BTreeMap`s, so the key does not depend on hash order.
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
/// An empty `steps` array is a successful parse. Forcing that shape would skip
/// the heuristic tier, so a registry with no named tools leaves `format` unset.
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

/// `true` when a 400 body is about the schema grammar rather than the model.
pub fn schema_format_rejected(status: u16, body: &str) -> bool {
    if status != 400 {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("format") || lower.contains("schema") || lower.contains("grammar")
}

/// Set `format` to `schema`, or to `"json"` when `force_json` and there is no schema.
pub fn apply_chat_format(body: &mut Value, schema: Option<&Value>, force_json: bool) {
    if let Some(schema) = schema {
        insert_schema_format(body, schema);
    } else if force_json {
        body["format"] = json!("json");
    }
}

fn registry_cache_key(reg: &ToolRegistry) -> String {
    let mut key = String::new();
    for tool in &reg.tools {
        key.push_str(&tool.name);
        key.push('{');
        for (arg, ty) in &tool.args {
            key.push_str(arg);
            key.push(':');
            key.push_str(ty);
            key.push(',');
        }
        key.push('}');
        key.push('|');
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
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": required,
        "properties": Value::Object(properties)
    })
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
        let variants: Vec<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
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
        assert!(!schema_format_rejected(400, "model not found"));
        assert!(!schema_format_rejected(500, "format exploded"));
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
