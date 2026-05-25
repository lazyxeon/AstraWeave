use astraweave_gameplay::*;
use std::fs;

fn main() -> anyhow::Result<()> {
    // Load dialogue
    let dlg_txt = fs::read_to_string("assets/dialogue_intro.toml")?;
    let dialogue: dialogue::Dialogue = toml::from_str(&dlg_txt)?;
    let mut dlg_state = dialogue::DialogueState::new(&dialogue);

    println!("-- Dialogue start --");
    loop {
        let node = dlg_state.current(&dialogue);
        if let Some(line) = &node.line {
            println!("{}: {}", line.speaker, line.text);
        }
        if node.end {
            break;
        }
        for (i, c) in node.choices.iter().enumerate() {
            println!("  [{}] {}", i, c.text);
        }
        // auto-pick first choice for demo:
        dlg_state.choose(&dialogue, 0);
    }

    // Quest
    let q_txt = fs::read_to_string("assets/quests_main.toml")?;
    let q: quests::Quest = toml::from_str(&q_txt)?;
    let mut log = quests::QuestLog::default();
    log.add(q.clone());
    println!("Quest added: {}", q.title);

    // Progress: gathered 2 crystals, then 1
    log.progress_gather("q_tutorial", "Crystal", 2);
    log.progress_gather("q_tutorial", "Crystal", 1);
    println!("Quest completed? {}", log.is_done("q_tutorial"));

    // Tiny cutscene
    // C.7.A (Unified Camera campaign): `Cue::CameraTo` migrated from
    // yaw/pitch storage to look_at storage; `CutsceneState::tick`
    // returns the structured `CutsceneTickEvent` enum instead of the
    // pre-C.7.A triple-Optional tuple. Equivalent visual framing
    // preserved via the canonical spherical-to-cartesian forward
    // direction (matches `FreeFly::dir` convention).
    let forward = |yaw: f32, pitch: f32| -> glam::Vec3 {
        glam::Vec3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
    };
    let cue_pos = glam::vec3(2.0, 3.0, 6.0);
    let tl = Timeline {
        cues: vec![
            Cue::Title {
                text: "Threads Awaken".into(),
                time: 1.5,
            },
            Cue::Wait { time: 0.5 },
            Cue::CameraTo {
                pos: cue_pos,
                look_at: cue_pos + forward(-1.57, -0.4),
                fov_deg: 60.0,
                time: 2.0,
            },
        ],
    };
    let mut cs = CutsceneState::new();
    let mut t = 0.0;
    while t < 4.0 {
        match cs.tick(0.5, &tl) {
            CutsceneTickEvent::Title(txt) => {
                println!("[Cutscene Title] {}", txt);
            }
            CutsceneTickEvent::Camera(key) => {
                println!(
                    "[Cutscene Camera] to {:?} look_at={:?} fov={:.1}°",
                    key.pos, key.look_at, key.fov_deg
                );
            }
            CutsceneTickEvent::Continue => {}
            CutsceneTickEvent::Done => break,
        }
        t += 0.5;
    }

    // Banter test
    let banter = r#"
[Companion] Threads hum in the fog.
-> mood=curious
? mood == curious : goto n1
[Companion] Or maybe I'm just cold.
"#;
    let dialog2 = dialogue::compile_banter_to_nodes("banter", banter);
    let mut ds2 = dialogue::DialogueState::new(&dialog2);
    println!(
        "Banter start: {}",
        ds2.current(&dialog2)
            .line
            .as_ref()
            .expect("Dialogue node should have line text")
            .text
    );
    ds2.choose(&dialog2, 0);

    Ok(())
}
