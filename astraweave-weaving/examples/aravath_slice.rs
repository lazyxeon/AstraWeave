fn main() -> anyhow::Result<()> {
    let scenario = astraweave_weaving::load_bundled_scenario()?;
    match astraweave_weaving::run_scenario(&scenario) {
        astraweave_weaving::RunOutcome::Pass(report) => {
            println!(
                "aravath sword hp {} weave hp {} cover intact {} cover damage {}",
                report.slice.sword_hp,
                report.slice.weave_hp,
                report.slice.cover_intact,
                report.slice.cover_damage
            );
            println!("gameplay_qa_aravath_slice {}", report.trace_sha256);
            Ok(())
        }
        astraweave_weaving::RunOutcome::Fail(artifact) => {
            let json = astraweave_weaving::gameplay_qa::failure_json(&artifact)?;
            eprintln!("{json}");
            anyhow::bail!(
                "scenario {} failed: {}",
                artifact.scenario_id,
                artifact.failure.kind
            );
        }
    }
}
