#[test]
fn version_probe_needs_no_home_or_graphical_session() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ts4-mod-manager-native"))
        .arg("--version")
        .env_remove("HOME")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("ts4-mod-manager-native {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(output.stderr.is_empty());
}
