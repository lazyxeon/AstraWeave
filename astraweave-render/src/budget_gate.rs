//! Command-line gate for `gpu_frame_budget`.
//!
//! An enabled gated flag is rejected before the scene is built.
//! `--name=false` and `--name false` are off and do not bail.

/// True when `name` is enabled on this command line.
///
/// Bare `--name` is on, including when the next token is another flag.
/// `--name=true` is on. The third clause is the `--name <value>` pair:
/// `true` is on and `false` is off.
pub fn flag_on(args: &[String], name: &str) -> bool {
    let equals_true = format!("{name}=true");
    args.iter().enumerate().any(|(index, arg)| {
        let next = args.get(index + 1).map(String::as_str);
        (arg == name && bare_flag(next))
            || arg == &equals_true
            || (arg == name && next == Some("true"))
    })
}

fn bare_flag(next: Option<&str>) -> bool {
    match next {
        None => true,
        Some(token) if token.starts_with('-') => true,
        Some(_) => false,
    }
}

/// Scaffolding only. No enabled gated flag draws a frame, so no cell can be filled.
pub fn reject_gated_flags(args: &[String]) -> anyhow::Result<()> {
    const BLOCKED: &[(&str, &str)] = &[
        (
            "--atmosphere",
            "enabling it skips the HDR and depth clear, so the heavy scene does not land",
        ),
        (
            "--ssgi",
            "normal and velocity targets are allocated and never written, and ssgi_history is sampled before copy_to_history; the pass writes denoised_view, and record_gated_presentation never copies it onto hdr_tex, so the post pass samples a frame that does not show it",
        ),
        (
            "--taa",
            "normal and velocity targets are allocated and never written, and taa_history is sampled before copy_to_history; the pass writes resolved_view and sharpened_view, and record_gated_presentation never copies them onto hdr_tex, so the post pass samples a frame that does not show them",
        ),
        (
            "--upscale",
            "normal and velocity targets are allocated and never written, and upscale_history is sampled before copy_to_history; the pass writes sharpened_view, and record_gated_presentation never copies it onto hdr_tex, so the post pass samples a frame that does not show it",
        ),
        (
            "--dof",
            "the depth-of-field history target is uncleared and the result is not copied onto HDR",
        ),
        (
            "--clouds",
            "the raymarch samples cloud_history before writing it, and the composite is not copied onto HDR",
        ),
    ];
    let hits: Vec<String> = BLOCKED
        .iter()
        .filter(|(flag, _)| flag_on(args, flag))
        .map(|(flag, why)| format!("{flag} ({why})"))
        .collect();
    if hits.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "gated flags cannot fill a measurement cell: {}",
        hits.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::{flag_on, reject_gated_flags};

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

    fn rejected(parts: &[&str]) -> String {
        reject_gated_flags(&args(parts))
            .expect_err("gated flag must fail before a frame")
            .to_string()
    }

    #[test]
    fn reject_gated_flags_covers_each_form() {
        assert!(reject_gated_flags(&args(&["--scene", "heavy"])).is_ok());
        for flag in [
            "--atmosphere",
            "--ssgi",
            "--taa",
            "--upscale",
            "--dof",
            "--clouds",
        ] {
            let message = rejected(&[flag]);
            assert!(message.contains(flag), "{message}");
            assert!(flag_on(&args(&[flag]), flag));
            assert!(flag_on(&args(&[&format!("{flag}=true")]), flag));
            assert!(flag_on(&args(&[flag, "true"]), flag));
            assert!(!flag_on(&args(&[&format!("{flag}=false")]), flag));
            assert!(!flag_on(&args(&[flag, "false"]), flag));
            assert!(reject_gated_flags(&args(&[&format!("{flag}=false")])).is_ok());
            assert!(reject_gated_flags(&args(&[flag, "false"])).is_ok());
        }
        let equals = rejected(&["--atmosphere=true"]);
        assert!(equals.contains("--atmosphere"), "{equals}");
        let two_token = rejected(&["--atmosphere", "true"]);
        assert!(two_token.contains("--atmosphere"), "{two_token}");
        let pair = rejected(&["--atmosphere", "--ssgi"]);
        assert!(pair.contains("--atmosphere"), "{pair}");
        assert!(pair.contains("--ssgi"), "{pair}");
        assert!(pair.contains("ssgi_history"), "{pair}");
        let ssgi = rejected(&["--ssgi"]);
        assert!(ssgi.contains("copy_to_history"), "{ssgi}");
        assert!(ssgi.contains("denoised_view"), "{ssgi}");
        assert!(ssgi.contains("hdr_tex"), "{ssgi}");
        assert!(ssgi.contains("record_gated_presentation"), "{ssgi}");
        let taa = rejected(&["--taa"]);
        assert!(taa.contains("taa_history"), "{taa}");
        assert!(taa.contains("resolved_view"), "{taa}");
        assert!(taa.contains("sharpened_view"), "{taa}");
        assert!(taa.contains("hdr_tex"), "{taa}");
        let upscale = rejected(&["--upscale"]);
        assert!(upscale.contains("upscale_history"), "{upscale}");
        assert!(upscale.contains("sharpened_view"), "{upscale}");
        assert!(upscale.contains("hdr_tex"), "{upscale}");
        let false_then_ssgi = rejected(&["--atmosphere", "false", "--ssgi"]);
        assert!(
            !false_then_ssgi.contains("--atmosphere"),
            "{false_then_ssgi}"
        );
        assert!(
            false_then_ssgi.contains("ssgi_history"),
            "{false_then_ssgi}"
        );
        assert!(rejected(&["--atmosphere", "false", "--atmosphere"]).contains("--atmosphere"));
    }
}
