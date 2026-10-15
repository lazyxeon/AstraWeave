use astraweave_llm::parse_bounded_model_json;
use astraweave_quests::ValidationCriteria;

#[test]
fn validation_script_stays_data() {
    let raw = r#"{"required_conditions":[],"forbidden_conditions":[],"validation_script":"process::exit(1)"}"#;
    let parsed: ValidationCriteria = parse_bounded_model_json(raw).expect("small json");
    assert_eq!(
        parsed.validation_script.as_deref(),
        Some("process::exit(1)")
    );
}
