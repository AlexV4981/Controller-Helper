use controller_studio::runtime::{application_phase, identify_titles};
#[test]
fn detects_game_switch_and_refuses_ambiguous_windows() {
    assert_eq!(
        identify_titles(&["Dolphin | SMNE01 | New Super Mario Bros. Wii".into()]),
        Some("SMNE01".into())
    );
    assert_eq!(
        identify_titles(&["Dolphin | RSBE01 | Super Smash Bros. Brawl".into()]),
        Some("RSBE01".into())
    );
    assert_eq!(identify_titles(&["SMNE01".into(), "RSBE01".into()]), None);
    assert_eq!(identify_titles(&["SMNE010 fake".into()]), None);
    assert_eq!(identify_titles(&["Dolphin 2609a".into()]), None);
    assert_eq!(
        identify_titles(&["Dolphin | Super Mario Galaxy 2".into()]),
        Some("SB4E01".into())
    );
    assert_eq!(
        identify_titles(&["Dolphin | New SUPER MARIO BROS. Wii".into()]),
        Some("SMNE01".into())
    );
}
#[test]
fn never_announces_success_while_waiting_or_after_failure() {
    assert_eq!(application_phase(true, true, false), "waiting");
    assert_eq!(application_phase(true, false, false), "applying");
    assert_eq!(application_phase(true, false, true), "error");
    assert_eq!(application_phase(false, false, false), "idle");
}

#[test]
fn melonds_fps_title_never_invents_a_loaded_game() {
    assert_eq!(identify_titles(&["[52/60] melonDS 1.1".into()]), None);
    assert_eq!(identify_titles(&["melonDS 1.1".into()]), None);
}
