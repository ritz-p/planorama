use super::*;

fn separate_graph(groups: usize, lanes: usize) -> Graph {
    let mut graph = corridor_graph(groups * (lanes + 1) - 1);
    graph.edges = (0..groups)
        .flat_map(|g| {
            (0..lanes).map(move |lane| {
                Edge::from((
                    g * (lanes * 2 + 2) + lane * 2,
                    g * (lanes * 2 + 2) + lane * 2 + 1,
                ))
            })
        })
        .collect();
    graph.relationships.clear();
    graph
}

fn separate_layout(graph: &Graph, groups: usize, lanes: usize, transpose: bool) -> Layout<'_> {
    let prototype = corridor_graph(lanes);
    let mut bounds = Vec::new();
    let mut paths = Vec::new();
    for group in 0..groups {
        let mut fixture = narrow(&prototype, false);
        let gap_y = 200 + lanes * 100;
        for (i, path) in fixture.paths.iter_mut().enumerate() {
            path[1].x = 493 + i;
            path[2].x = 493 + i;
            fixture.bounds[i * 2 + 1].origin.y = gap_y + 400 + i * 100;
            path[2].y = gap_y + 430 + i * 100;
            path[3].y = path[2].y;
        }
        fixture.bounds[lanes * 2].origin.y = gap_y;
        fixture.bounds[lanes * 2 + 1].origin.y = gap_y;
        for b in &mut fixture.bounds {
            b.origin.x += group * 1400;
        }
        for p in fixture.paths.iter_mut().flatten() {
            p.x += group * 1400;
        }
        bounds.extend(fixture.bounds);
        paths.extend(fixture.paths);
    }
    let (mut width, mut height) = (groups * 1400, 3000);
    if transpose {
        for b in &mut bounds {
            std::mem::swap(&mut b.origin.x, &mut b.origin.y);
            std::mem::swap(&mut b.width, &mut b.height);
        }
        for p in paths.iter_mut().flatten() {
            std::mem::swap(&mut p.x, &mut p.y);
        }
        std::mem::swap(&mut width, &mut height);
    }
    Layout {
        header_heights: bounds.iter().map(|b| b.height).collect(),
        positions: bounds.iter().map(|b| b.origin).collect(),
        bounds,
        paths,
        width,
        height,
        containment: ContainmentTree::new(graph),
        containers: vec![],
        scopes: vec![],
        bands: vec![],
        junctions: vec![],
    }
}

#[test]
fn separate_gaps_use_more_than_six_attempts_two_sweeps_and_64px_on_both_axes() {
    let graph = separate_graph(7, 7);
    for transpose in [false, true] {
        let mut layout = separate_layout(&graph, 7, 7, transpose);
        let original = expansion::Snapshot::capture(&layout);
        let stats = refine(&graph, &mut layout);
        eprintln!(
            "separate transpose={transpose}: {stats:?} growth={}x{}",
            layout.width - original.width,
            layout.height - original.height
        );
        assert!(stats.attempts > 6 && stats.sweeps > 2 && stats.expansions >= 7);
        assert!(
            if transpose {
                layout.height - original.height
            } else {
                layout.width - original.width
            } > 640
        );
        assert_eq!(measure(&graph, &layout.paths).close_length, 0);
        assert!(layout.paths.iter().all(|p| Shape::measure(p).bends == 2));
        assert_ne!(layout.bounds, original.bounds);
        for (i, path) in layout.paths.iter().enumerate() {
            assert!(ports::valid_with_clearances(
                path,
                terminal(path, 0),
                terminal(path, 1),
                &barriers(&graph, &layout, i),
                [crate::layout::relationship_markers::Terminal::plain(16); 2]
            ));
        }
        let mut repeated = separate_layout(&graph, 7, 7, transpose);
        refine(&graph, &mut repeated);
        assert_eq!(layout.paths, repeated.paths);
        assert_eq!(layout.bounds, repeated.bounds);
    }
}

#[test]
fn large_expansion_updates_enclosing_containers_scopes_and_bands_in_both_axes() {
    let mut graph = separate_graph(1, 7);
    let parent = graph.nodes.len();
    let raw = crate::plan::parse(r#"{"format_version":"1.2","resource_changes":[{"address":"test.container","type":"test"}]}"#).unwrap();
    let mut node = crate::semantic::transform(&raw).0.nodes.remove(0);
    node.role = crate::model::ResourceRole::Container;
    graph.nodes.push(node);
    graph.edges.extend((0..parent).map(|to| Edge {
        from: parent,
        to,
        kind: crate::model::EdgeKind::Containment,
        change: None,
    }));
    for transpose in [false, true] {
        let mut layout = separate_layout(&graph, 1, 7, transpose);
        layout.bounds.push(Bounds {
            origin: Point { x: 20, y: 50 },
            width: layout.width - 40,
            height: layout.height - 100,
        });
        layout.header_heights.push(40);
        layout.paths.resize(graph.edges.len(), vec![]);
        layout.containers.push(parent);
        layout.scopes.push(crate::layout::ScopePanel {
            account_panel: false,
            scope: crate::model::architecture::DeploymentScope {
                provider: "test".into(),
                region: None,
                account: None,
                global: true,
            },
            bounds: Bounds {
                origin: Point { x: 0, y: 0 },
                width: layout.width,
                height: layout.height,
            },
        });
        layout.bands.push(crate::layout::Band {
            label: "test",
            top: 40,
            height: layout.height - 40,
        });
        let original = expansion::Snapshot::capture(&layout);
        let stats = refine(&graph, &mut layout);
        assert!(stats.expansions > 0);
        let growth = if transpose {
            layout.height - original.height
        } else {
            layout.width - original.width
        };
        assert!(growth > 64);
        assert_eq!(
            if transpose {
                layout.bounds[parent].height - original.bounds[parent].height
            } else {
                layout.bounds[parent].width - original.bounds[parent].width
            },
            growth
        );
        assert_eq!(
            (
                layout.scopes[0].bounds.width,
                layout.scopes[0].bounds.height
            ),
            (layout.width, layout.height)
        );
        assert_eq!(layout.bands[0].height, layout.height - 40);
    }
}

#[test]
fn necessary_endpoint_growth_over_128px_preserves_simple_shapes_and_fixed_detour_guard() {
    let graph = separate_graph(1, 10);
    for transpose in [false, true] {
        let mut layout = separate_layout(&graph, 1, 10, transpose);
        let original = layout.paths.clone();
        let initial_deficit = measure(&graph, &layout.paths).deficit;
        let stats = refine(&graph, &mut layout);
        eprintln!("ten lanes {transpose}: {stats:?}");
        assert!(stats.expansions > 0);
        assert!(
            layout
                .paths
                .iter()
                .zip(&original)
                .all(|(after, before)| length(after) > length(before) + 128)
        );
        assert!(measure(&graph, &layout.paths).deficit < initial_deficit);
        for path in &layout.paths {
            assert_eq!(Shape::measure(path).bends, 2);
        }
        let base = Shape::measure(&layout.paths[0]);
        assert!(
            !Shape {
                length: base.length + 129,
                ..base
            }
            .fits(base)
        );
        assert!(
            !Shape {
                s_jogs: base.s_jogs + 1,
                ..base
            }
            .fits(base)
        );
    }
}

#[test]
fn unchanged_failures_terminate_without_repeated_attempts() {
    let graph = corridor_graph(7);
    let mut layout = narrow(&graph, false);
    for path in &layout.paths {
        layout.junctions.push(path[1]);
    }
    let original = expansion::Snapshot::capture(&layout);
    let stats = refine(&graph, &mut layout);
    assert_eq!(stats.sweeps, 1);
    assert_eq!(stats.attempts, 0);
    assert_eq!(stats.reconsidered, 0);
    assert_eq!(layout.paths, original.paths);
    assert_eq!(layout.bounds, original.bounds);
}

#[test]
fn a_failed_corridor_is_retried_after_another_gap_frees_its_unchanged_tracks() {
    let graph = separate_graph(2, 2);
    let mut layout = separate_layout(&graph, 2, 2, false);
    layout.width = 1200;
    layout.height = 9000;
    for group in 0..2 {
        let first = group * 6;
        let (top, bottom, wall_y, wall_height, left, tracks) = if group == 0 {
            (100, 6000, 1000, 4000, 480, [486, 494])
        } else {
            (7000, 7600, 7400, 50, 496, [497, 499])
        };
        for (lane, x) in tracks.into_iter().enumerate() {
            layout.bounds[first + lane * 2] = Bounds {
                origin: Point {
                    x: 100,
                    y: top + lane * 100,
                },
                width: 100,
                height: 60,
            };
            layout.bounds[first + lane * 2 + 1] = Bounds {
                origin: Point {
                    x: 800,
                    y: bottom + lane * 100,
                },
                width: 100,
                height: 60,
            };
            layout.paths[group * 2 + lane] = vec![
                Point {
                    x: 200,
                    y: top + lane * 100 + 30,
                },
                Point {
                    x,
                    y: top + lane * 100 + 30,
                },
                Point {
                    x,
                    y: bottom + lane * 100 + 30,
                },
                Point {
                    x: 800,
                    y: bottom + lane * 100 + 30,
                },
            ];
        }
        layout.bounds[first + 4] = Bounds {
            origin: Point { x: 100, y: wall_y },
            width: left - 100,
            height: wall_height,
        };
        layout.bounds[first + 5] = Bounds {
            origin: Point { x: 500, y: wall_y },
            width: 400,
            height: wall_height,
        };
    }
    layout.positions = layout.bounds.iter().map(|b| b.origin).collect();
    let initial = measure(&graph, &layout.paths).deficit;
    let stats = refine(&graph, &mut layout);
    eprintln!("newly feasible: {stats:?}");
    assert!(stats.reconsidered > 0);
    assert!(stats.accepted_reconsiderations > 0);
    assert!(measure(&graph, &layout.paths).deficit < initial);
}

#[test]
#[ignore = "explicit CI spacing calibration across budget candidates"]
fn budget_calibration() {
    let raw = crate::plan::parse(include_str!(
        "../../../../examples/terraform-large/plan.json"
    ))
    .unwrap();
    let large = crate::semantic::transform(&raw);
    let dense = separate_graph(7, 7);
    let constrained = corridor_graph(7);
    let mut rows = Vec::new();
    for (name, graph) in [
        ("terraform-large", &large.0),
        ("dense-corridors", &dense),
        ("constrained", &constrained),
    ] {
        let mut layout = match name {
            "terraform-large" => Layout::unspaced(graph),
            "dense-corridors" => separate_layout(graph, 7, 7, false),
            _ => narrow(graph, false),
        };
        if name == "constrained" {
            for path in &layout.paths {
                layout.junctions.push(path[1]);
            }
        }
        let default = Budget::for_layout(&layout);
        for (label, budget) in [
            (
                "previous-budgets",
                Budget {
                    sweeps: 2,
                    attempts: 6,
                    growth: [64; 2],
                },
            ),
            ("10x", default),
            (
                "20x",
                Budget {
                    sweeps: 40,
                    attempts: default.attempts * 2,
                    growth: default.growth.map(|g| g * 2),
                },
            ),
        ] {
            let before = expansion::Snapshot::capture(&layout);
            let start = std::time::Instant::now();
            let stats = refine_with_budget(graph, &mut layout, budget);
            let millis = start.elapsed().as_secs_f64() * 1000.0;
            let proximity = measure(graph, &layout.paths);
            let borders = border_metrics(graph, &layout);
            let edges: Vec<_> = graph.edges.iter().zip(&layout.paths).map(|(edge, path)| {
                let shape = Shape::measure(path);
                serde_json::json!({"from": graph.nodes[edge.from].entity.id.as_str(), "to": graph.nodes[edge.to].entity.id.as_str(),
                    "length": shape.length, "bends": shape.bends, "offsets": shape.offsets, "reversals": shape.reversals, "backtracking": shape.backtracking, "s_jogs": shape.s_jogs})
            }).collect();
            let row = serde_json::json!({"fixture": name, "candidate": label, "sweeps": stats.sweeps, "attempts": stats.attempts,
                "accepted": stats.accepted, "expansions": stats.expansions, "reconsidered": stats.reconsidered, "accepted_reconsiderations": stats.accepted_reconsiderations,
                "width": layout.width, "height": layout.height, "parallel_deficit": proximity.deficit,
                "border_minimum_deficit": borders.iter().map(|b| b.minimum_deficit).sum::<u128>(),
                "border_preferred_deficit": borders.iter().map(|b| b.preferred_deficit).sum::<u128>(), "refinement_ms": millis, "edges": edges});
            let mut summary = row.clone();
            summary.as_object_mut().unwrap().remove("edges");
            eprintln!("spacing calibration: {summary}");
            if let Ok(directory) = std::env::var("PLANORAMA_SPACING_REPORT_DIR") {
                std::fs::create_dir_all(&directory).unwrap();
                std::fs::write(
                    std::path::Path::new(&directory).join(format!("{name}-{label}.svg")),
                    crate::svg::render(graph, &layout),
                )
                .unwrap();
            }
            rows.push(row);
            before.restore(&mut layout);
        }
    }
    if let Ok(directory) = std::env::var("PLANORAMA_SPACING_REPORT_DIR") {
        std::fs::write(
            std::path::Path::new(&directory).join("spacing-calibration.json"),
            serde_json::to_string_pretty(&rows).unwrap(),
        )
        .unwrap();
    }
}
