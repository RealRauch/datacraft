//! Low-power/RDP rendering mode (step 92.6, item L51; ROADMAP §23).
//!
//! RDP sessions and software rasterizers re-encode every frame, so the
//! mode trims the shell's frame policy: zero animations, no blurred
//! shadows and interaction repaints capped at ~30 fps — while an idle
//! frame schedules nothing (idle repaint stays zero). It is a policy
//! switch only: no renderer or drawing-path change.
//!
//! Precedence: the `DATACRAFT_LOW_POWER` override beats the `ui.json`
//! setting, which beats the detection (software adapter string and the
//! Windows `SESSIONNAME` heuristic starting with `RDP-`).
//!
//! The adapter-string input stays pure: reading `GL_RENDERER` needs
//! unsafe GL access (`glow::Context::get_parameter_string` is unsafe),
//! which the workspace's `unsafe_code = "forbid"` forbids — the live
//! composition therefore feeds `None` for the renderer today and the
//! RDP-session heuristic carries the detection; the matching rules are
//! unit-tested and activate the moment a safe accessor appears.

/// The rendering mode of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// The default frame policy.
    #[default]
    Normal,
    /// The trimmed policy for RDP sessions and software rasterizers.
    LowPower,
}

/// The interaction repaint cap of the low-power policy (~30 fps).
pub const INTERACTION_REPAINT_SECS: f32 = 1.0 / 30.0;

/// Whether the inputs indicate a low-power session: a software adapter
/// string (GL_RENDERER) or an RDP session name.
pub fn detect(renderer: Option<&str>, session_name: Option<&str>) -> bool {
    const SOFTWARE_MARKERS: &[&str] = &[
        "llvmpipe",
        "swrast",
        "softpipe",
        "software",
        "basic render",
        "gdi generic",
    ];
    let renderer_hit = renderer.is_some_and(|renderer| {
        let renderer = renderer.to_lowercase();
        SOFTWARE_MARKERS.iter().any(|marker| renderer.contains(marker))
    });
    let session_hit = session_name
        .is_some_and(|session| session.to_uppercase().starts_with("RDP-"));
    renderer_hit || session_hit
}

/// Resolves the mode by precedence: override > setting > detection. An
/// override that is neither `1` nor `0` falls through to the next source.
pub fn resolve(
    override_env: Option<&str>,
    setting: Option<bool>,
    detected: bool,
) -> RenderMode {
    match override_env {
        Some("1") => return RenderMode::LowPower,
        Some("0") => return RenderMode::Normal,
        _ => {}
    }
    if let Some(low_power) = setting {
        return if low_power {
            RenderMode::LowPower
        } else {
            RenderMode::Normal
        };
    }
    if detected {
        RenderMode::LowPower
    } else {
        RenderMode::Normal
    }
}

/// The frame policy a mode pins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Policy {
    /// Whether egui animations run (low power pins them off).
    pub animations: bool,
    /// The interaction repaint cap in seconds (`None` = egui's default).
    pub interaction_repaint_secs: f32,
}

/// The policy of one mode.
pub fn policy(mode: RenderMode) -> Policy {
    match mode {
        RenderMode::Normal => Policy {
            animations: true,
            interaction_repaint_secs: 0.0,
        },
        RenderMode::LowPower => Policy {
            animations: false,
            interaction_repaint_secs: INTERACTION_REPAINT_SECS,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RDP session names and software adapter strings detect the
    /// low-power mode (step 92.6, item L51).
    #[test]
    fn rdp_sessions_and_software_adapters_detect_low_power() {
        assert!(detect(None, Some("RDP-Tcp#0")));
        assert!(detect(None, Some("rdp-tcp#3")));
        assert!(detect(Some("llvmpipe (LLVM 15.0.7)"), None));
        assert!(detect(Some("Mesa X11 / swrast"), None));
        assert!(detect(Some("Microsoft Basic Render Driver"), None));
        assert!(detect(Some("GDI Generic"), None));
    }

    /// Hardware adapters and local sessions stay normal.
    #[test]
    fn hardware_adapters_and_local_sessions_stay_normal() {
        assert!(!detect(
            Some("NVIDIA GeForce RTX 4080/PCIe/SSE2"),
            None
        ));
        assert!(!detect(Some("ANGLE (Intel, Mesa)"), None));
        assert!(!detect(None, Some("Console")));
        assert!(!detect(None, None));
    }

    /// The override beats the setting, the setting beats the detection;
    /// an invalid override falls through (step 92.6, item L51).
    #[test]
    fn the_env_override_beats_setting_and_detection() {
        assert_eq!(resolve(Some("1"), Some(false), false), RenderMode::LowPower);
        assert_eq!(resolve(Some("0"), Some(true), true), RenderMode::Normal);
        assert_eq!(
            resolve(Some("on"), Some(true), false),
            RenderMode::LowPower,
            "an invalid override falls through to the setting"
        );
        assert_eq!(resolve(None, Some(false), true), RenderMode::Normal);
        assert_eq!(resolve(None, Some(true), false), RenderMode::LowPower);
        assert_eq!(resolve(None, None, true), RenderMode::LowPower);
        assert_eq!(resolve(None, None, false), RenderMode::Normal);
    }

    /// The low-power policy pins zero animations and the ~30 fps
    /// interaction cap; the normal policy keeps egui's defaults
    /// (step 92.6, item L51).
    #[test]
    fn the_policy_pins_zero_animations_and_the_interaction_cap() {
        let low = policy(RenderMode::LowPower);
        assert!(!low.animations);
        assert!((low.interaction_repaint_secs - 1.0 / 30.0).abs() < 1e-6);
        let normal = policy(RenderMode::Normal);
        assert!(normal.animations);
        assert!((normal.interaction_repaint_secs - 0.0).abs() < 1e-6);
    }
}
