use astraweave_security::default_allowed_llm_domains;

#[test]
fn default_llm_domains_are_local_only() {
    let domains = default_allowed_llm_domains();
    assert_eq!(domains, vec!["localhost".to_string()]);
}
