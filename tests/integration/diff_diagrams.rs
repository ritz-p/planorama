use super::cli;
use roxmltree::{Document, Node};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn plan(after: bool) -> Value {
    let mut plan: Value =
        serde_json::from_slice(include_bytes!("../fixtures/rds-subnet-group-plan.json")).unwrap();
    let extra = if after {
        "aws_subnet.added"
    } else {
        "aws_subnet.removed"
    };
    let entries = plan["resource_changes"].as_array_mut().unwrap();
    entries.extend([
        json!({"address":extra,"type":"aws_subnet"}),
        json!({"address":"aws_ecs_service.app","type":"aws_ecs_service"}),
        json!({"address":"aws_instance.moving","type":"aws_instance"}),
        json!({"address":"aws_s3_bucket.uncertain","type":"aws_s3_bucket",
            "provider_name":if after {"private/aws"} else {"hashicorp/aws"}}),
    ]);
    plan["configuration"]["root_module"]["resources"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!({"address":extra,"expressions":{"vpc_id":{"references":["aws_vpc.main.id"]}}}),
            json!({"address":"aws_instance.moving","expressions":{"subnet_id":{"references":[
                if after {"aws_subnet.b.id"} else {"aws_subnet.a.id"}
            ]}}}),
        ]);
    plan
}

fn render(root: &Path, output: &str, flags: &[&str]) -> std::process::Output {
    cli()
        .arg("diff")
        .arg(root.join("old.json"))
        .arg(root.join("new.json"))
        .arg("--svg-dir")
        .arg(root.join(output))
        .args(flags)
        .output()
        .unwrap()
}

fn card<'a>(doc: &'a Document<'a>, address: &str) -> Node<'a, 'a> {
    let name = format!("default · {address}");
    doc.descendants()
        .find(|n| {
            n.has_tag_name("g")
                && n.attribute("id").is_some()
                && n.children()
                    .any(|c| c.has_tag_name("title") && c.text() == Some(name.as_str()))
        })
        .unwrap()
}

#[test]
fn shared_svg_positions_parents_and_relationships_follow_each_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    for (name, after) in [("old.json", false), ("new.json", true)] {
        fs::write(dir.path().join(name), plan(after).to_string()).unwrap();
    }
    let output = render(dir.path(), "pair", &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let read = |side| fs::read_to_string(dir.path().join("pair").join(side)).unwrap();
    let before = read("before.svg");
    let after = read("after.svg");
    let a = Document::parse(&before).unwrap();
    let b = Document::parse(&after).unwrap();
    assert_eq!(
        a.root_element().attribute("viewBox"),
        b.root_element().attribute("viewBox")
    );
    for address in [
        "aws_ecs_service.app",
        "aws_db_instance.main",
        "aws_vpc.main",
    ] {
        let rect = |n: Node<'_, '_>| {
            let rect = n.children().find(|c| c.has_tag_name("rect")).unwrap();
            ["x", "y", "width", "height"].map(|k| rect.attribute(k).unwrap().to_owned())
        };
        assert_eq!(rect(card(&a, address)), rect(card(&b, address)));
    }
    assert_ne!(
        card(&a, "aws_s3_bucket.uncertain").attribute("id"),
        card(&b, "aws_s3_bucket.uncertain").attribute("id")
    );
    for (doc, parent, other) in [
        (&a, "aws_subnet.a", "aws_subnet.b"),
        (&b, "aws_subnet.b", "aws_subnet.a"),
    ] {
        let child = card(doc, "aws_instance.moving");
        assert_eq!(
            child.attribute("data-parent"),
            card(doc, parent).attribute("id")
        );
        let contained_by = |addr| {
            doc.descendants().any(|e| {
                e.attribute("data-edge-kind") == Some("containment")
                    && e.attribute("data-source") == card(doc, addr).attribute("id")
                    && e.attribute("data-target") == child.attribute("id")
            })
        };
        assert!(contained_by(parent));
        assert!(!contained_by(other));
    }
    assert!(!before.contains("aws_subnet.added"));
    assert!(!after.contains("aws_subnet.removed"));
    assert!(!render(dir.path(), "pair", &[]).status.success());
    assert_eq!(read("before.svg"), before);
    assert!(
        render(dir.path(), "masked", &["--anonymize"])
            .status
            .success()
    );
    for name in ["before.svg", "after.svg"] {
        let svg = fs::read_to_string(dir.path().join("masked").join(name)).unwrap();
        assert!(!svg.contains("aws_") && !svg.contains("default") && !svg.contains("private/aws"));
        assert!(svg.contains("Resource "));
    }
}

#[test]
fn shared_svg_keeps_the_slot_for_an_explicit_move() {
    let dir = tempfile::tempdir().unwrap();
    for (file, name) in [("old.json", "before"), ("new.json", "after")] {
        let mut node = json!({"address":format!("aws_vpc.{name}"), "type":"aws_vpc"});
        if name == "after" {
            node["previous_address"] = json!("aws_vpc.before");
        }
        fs::write(
            dir.path().join(file),
            json!({"format_version":"1.2","resource_changes":[node]}).to_string(),
        )
        .unwrap();
    }
    assert!(render(dir.path(), "move", &[]).status.success());
    let geometry = |file, name| {
        let svg = fs::read_to_string(dir.path().join("move").join(file)).unwrap();
        let doc = Document::parse(&svg).unwrap();
        let g = card(&doc, name);
        let rect = g.children().find(|c| c.has_tag_name("rect")).unwrap();
        [g.attribute("id"), rect.attribute("x"), rect.attribute("y")].map(|v| v.unwrap().to_owned())
    };
    assert_eq!(
        geometry("before.svg", "aws_vpc.before"),
        geometry("after.svg", "aws_vpc.after")
    );
}
