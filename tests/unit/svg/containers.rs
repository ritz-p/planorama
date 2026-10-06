use super::*;
use crate::model::{Edge, EdgeKind, EntityMode, Node};

#[test]
fn five_container_shades_fill_both_headers_and_bounds_and_clamp_deeper_nesting() {
    for action in [
        Action::Create,
        Action::Update,
        Action::Delete,
        Action::Replace,
        Action::Read,
        Action::Unchanged,
        Action::Other,
    ] {
        let graph = Graph {
            nodes: (0..7)
                .map(|index| Node {
                    address: format!("test.container{index}"),
                    resource_type: "test".into(),
                    provider: crate::model::ProviderIdentity::inferred("test"),
                    module: "root".into(),
                    action,
                    mode: EntityMode::Managed,
                    role: match index {
                        6 => ResourceRole::Node,
                        _ => ResourceRole::Container,
                    },
                })
                .collect(),
            edges: (0..6)
                .map(|index| Edge {
                    kind: EdgeKind::Containment,
                    ..Edge::from((index, index + 1))
                })
                .collect(),
        };
        let layout = Layout::new(&graph);
        let shades: Vec<_> = (0..7)
            .map(|node| background(&graph, &layout, node))
            .collect();
        assert_eq!(shades[0], color(action).0);
        assert_eq!(shades[4], shades[5]);
        assert_eq!(shades[6], color(action).0);
        let brightness = |shade: &str| {
            (1..7)
                .step_by(2)
                .map(|index| u32::from_str_radix(&shade[index..index + 2], 16).unwrap())
                .sum::<u32>()
        };
        assert!(
            shades[..5]
                .windows(2)
                .all(|pair| brightness(pair[0]) > brightness(pair[1]))
        );
        let output = crate::svg::render(&graph, &layout);
        for (node, shade) in shades.iter().enumerate().take(6) {
            let boundary = output
                .split(&format!("data-container=\"resource-{node}\""))
                .nth(1)
                .unwrap()
                .split("</g>")
                .next()
                .unwrap();
            let header = output
                .split(&format!("id=\"resource-{node}\""))
                .nth(1)
                .unwrap()
                .split("</g>")
                .next()
                .unwrap();
            assert!(boundary.contains(&format!("fill=\"{shade}\"")));
            assert_eq!(boundary.matches("<rect ").count(), 1);
            assert!(!header.contains("<rect "));
        }
    }
}
