use super::render;

#[test]
fn region_panels_are_deterministic_and_resources_are_not_duplicated() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/regions-plan.json")).unwrap();
    let svg = render(input.to_string().as_bytes());
    let document = roxmltree::Document::parse(&svg).unwrap();
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.attribute("data-deployment-scope").is_some())
            .count(),
        3
    );
    for address in input["resource_changes"].as_array().unwrap() {
        assert_eq!(
            document
                .descendants()
                .filter(|n| n.attribute("data-terraform-address") == address["address"].as_str())
                .count(),
            1
        );
    }
    assert!(
        svg.contains("AWS ap-northeast-1")
            && svg.contains("AWS us-east-1")
            && svg.contains("AWS Global")
    );
    assert!(!svg.contains("us-west-2"));
    input["resource_changes"].as_array_mut().unwrap().reverse();
    input["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(svg, render(input.to_string().as_bytes()));
    assert_eq!(
        svg,
        include_str!("../../examples/regions.svg").replace("\r\n", "\n")
    );
}

#[test]
fn named_states_keep_distinct_region_panels() {
    let path = format!(
        "{}/tests/fixtures/regions-plan.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_planorama"))
        .args([
            "--state",
            &format!("first={path}"),
            "--state",
            &format!("second={path}"),
            "-o",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let svg = String::from_utf8(output.stdout).unwrap();
    let document = roxmltree::Document::parse(&svg).unwrap();
    for id in ["first", "second"] {
        let state = document
            .descendants()
            .find(|n| n.attribute("data-state-id") == Some(id))
            .unwrap();
        assert_eq!(
            state
                .descendants()
                .filter(|n| n.attribute("data-region") == Some("ap-northeast-1"))
                .count(),
            1
        );
        assert_eq!(
            state
                .descendants()
                .filter(|n| n.attribute("data-deployment-scope").is_some())
                .count(),
            3
        );
    }
}
