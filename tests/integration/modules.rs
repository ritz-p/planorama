use super::cli;
use serde_json::json;

fn fixture() -> serde_json::Value {
    json!({"format_version":"1.2","resource_changes":[
        {"address":"terraform_data.root","type":"terraform_data"},
        {"address":"module.app[\"a,b:c.d\"].terraform_data.x","type":"terraform_data"},
        {"address":"module.app[\"a,b:c.d\"].terraform_data.x","type":"terraform_data"},
        {"address":"module.app[\"a,b:c.d\"].terraform_data.x","deposed":"old","type":"terraform_data"},
        {"address":"module.app[\"a,b:c.d\"].module.db.data.aws_ami.x","mode":"data","type":"aws_ami"}
    ],"planned_values":{"root_module":{"child_modules":[{"address":"module.empty[0]"}]}},
    "configuration":{"root_module":{"module_calls":{"app":{"module":{}},"unknown":{"module":{}}}}}})
}

#[test]
fn list_modules_counts_original_instances_without_rendering_and_roundtrips_selectors() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.json");
    std::fs::write(&input, fixture().to_string()).unwrap();
    let output = cli()
        .current_dir(dir.path())
        .arg(&input)
        .args(["--list-modules", "--timings"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("(root; not selectable)\tinstance\t1\t4"));
    assert!(text.contains("module.app\tconfiguration (aggregate)\t2\t3"));
    assert!(text.contains("module.app[\"a,b:c.d\"]\tinstance\t2\t3"));
    assert!(text.contains("module.empty[0]\tinstance\t0\t0"));
    assert!(text.contains("module.unknown\tconfiguration (aggregate)\t0\t0"));
    assert!(!String::from_utf8(output.stderr).unwrap().contains("layout"));
    assert!(!dir.path().join("diagram.svg").exists());
    for row in text.lines().filter(|line| line.starts_with("module.")) {
        let selector = row.split('\t').next().unwrap();
        let result = cli()
            .arg(&input)
            .args(["--modules", selector, "-o", "-"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{selector}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let named = cli()
        .args([
            "--state",
            &format!("two={}", input.display()),
            "--state",
            &format!("one={}", input.display()),
            "--list-modules",
        ])
        .output()
        .unwrap();
    assert!(named.status.success());
    let named = String::from_utf8(named.stdout).unwrap();
    assert!(named.find("one:module.app").unwrap() < named.find("two:module.app").unwrap());
    let path = dir.path().join("modules.txt");
    let saved = cli()
        .arg(&input)
        .arg("--list-modules")
        .arg("-o")
        .arg(&path)
        .output()
        .unwrap();
    assert!(saved.status.success());
    assert!(saved.stdout.is_empty());
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
}

#[test]
fn list_modules_rejects_filters_rendering_options_and_input_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.json");
    let original = fixture().to_string();
    std::fs::write(&input, &original).unwrap();
    for args in [
        vec!["--changed-only"],
        vec!["--modules", "module.app"],
        vec!["--focus", "terraform_data.root"],
        vec!["--format", "svg"],
        vec!["--diagnostics"],
        vec!["--coverage-report", "report.md"],
    ] {
        let output = cli()
            .current_dir(dir.path())
            .arg(&input)
            .arg("--list-modules")
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--list-modules cannot"));
    }
    let output = cli()
        .arg(&input)
        .args(["--list-modules", "-o"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(input).unwrap(), original);
    assert!(!dir.path().join("report.md").exists());
}
