use crate::{layout, model, plan, semantic, view};
use std::{collections::BTreeMap, time::Instant};

#[test]
fn input_reordering_preserves_multi_account_facts() {
    for input in [
        include_str!("../../examples/multi-account/application-a.json"),
        include_str!("../../examples/multi-account/application-b.json"),
        include_str!("../../examples/multi-account/shared-services.json"),
    ] {
        let expected = plan::parse(input).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(input).unwrap();
        value["resource_changes"].as_array_mut().unwrap().reverse();
        fn reverse(module: &mut serde_json::Value) {
            if let Some(resources) = module["resources"].as_array_mut() {
                resources.reverse();
            }
            for call in module["module_calls"]
                .as_object_mut()
                .into_iter()
                .flat_map(|m| m.values_mut())
            {
                reverse(&mut call["module"]);
            }
        }
        reverse(&mut value["configuration"]["root_module"]);
        assert_eq!(expected, plan::parse(&value.to_string()).unwrap());
    }
}

#[test]
#[ignore = "maintained multi-account baseline; run explicitly for layout measurements"]
fn baseline() {
    println!(
        "revision={}; compiler={}; environment={}; optimized=true; threads={:?}",
        super::metadata("git", &["rev-parse", "HEAD"]),
        super::metadata("rustc", &["--version"]),
        super::metadata("uname", &["-sm"]),
        std::thread::available_parallelism()
    );
    let sources = [
        (
            "application-a",
            include_str!("../../examples/multi-account/application-a.json"),
        ),
        (
            "application-b",
            include_str!("../../examples/multi-account/application-b.json"),
        ),
        (
            "shared-services",
            include_str!("../../examples/multi-account/shared-services.json"),
        ),
    ];
    let start = Instant::now();
    let plans = plan::parse_inputs(
        sources
            .iter()
            .map(|(id, input)| model::PlanInput {
                state_id: model::StateId::new(id).unwrap(),
                plan_json: (*input).into(),
            })
            .collect(),
    )
    .unwrap();
    let mappings = [
        "application-a:module.application.data.terraform_remote_state.shared=shared-services",
        "application-b:module.application.data.terraform_remote_state.shared=shared-services",
        "application-b:data.terraform_remote_state.peer=application-a",
    ]
    .map(|m| model::cross_state::Mapping::parse(m).unwrap());
    let cross = semantic::cross_state::resolve(&plans, &mappings);
    assert!(cross.diagnostics.is_empty(), "{:?}", cross.diagnostics);
    assert_eq!(cross.edges.len(), 15);
    let architecture = model::architecture::Architecture {
        states: plans
            .states
            .iter()
            .map(|(id, p)| (id.clone(), semantic::transform(p)))
            .collect::<BTreeMap<_, _>>(),
        relationships: cross.edges,
    };
    let viewed = view::apply(&architecture, &Default::default()).unwrap();
    println!(
        "parse_semantic_view_ms={:.3}; cross_references={}",
        start.elapsed().as_secs_f64() * 1000.0,
        viewed.relationships().len()
    );
    for (id, graph) in &viewed.states {
        let start = Instant::now();
        let layout = layout::Layout::new(graph);
        let elapsed = start.elapsed();
        let metrics = layout::metrics::measure(&layout.paths);
        let borders: usize = layout
            .paths
            .iter()
            .map(|p| layout::metrics::border_overlap(p, &layout.bounds))
            .sum();
        println!("border_overlap_px={borders}");
        println!(
            "state={id:?}; input={}; displayed={}; paths={}; accounts={}; regions={}; width={}; height={}; layout_ms={:.3}; metrics={metrics:?}",
            plans.states[id].nodes.len(),
            graph.nodes.len(),
            layout.paths.iter().filter(|p| p.len() > 1).count(),
            layout.scopes.iter().filter(|s| s.account_panel).count(),
            layout
                .scopes
                .iter()
                .filter(|s| s.scope.region.is_some())
                .count(),
            layout.width,
            layout.height,
            elapsed.as_secs_f64() * 1000.0
        );
        assert!(!layout.scopes.is_empty());
    }
}
