use serde_json::json;
use std::time::Instant;

fn generate(count: usize, dense: bool) -> String {
    let mut changes = Vec::new();
    let mut resources = Vec::new();
    for i in 0..count {
        let base = i / 20 * 20;
        let kind = match i % 20 {
            0 => "aws_vpc",
            1 => "aws_subnet",
            _ => "aws_instance",
        };
        let address = format!("{kind}.n{i:04}");
        let mut expressions = serde_json::Map::new();
        match i % 20 {
            0 => {}
            1 => {
                expressions.insert(
                    "vpc_id".into(),
                    json!({"references":[format!("aws_vpc.n{base:04}.id")]}),
                );
            }
            _ => {
                expressions.insert(
                    "subnet_id".into(),
                    json!({"references":[format!("aws_subnet.n{:04}.id", base+1)]}),
                );
                let references: Vec<_> = (base + 2..i)
                    .rev()
                    .take(if dense { 4 } else { 0 })
                    .map(|j| format!("aws_instance.n{j:04}.id"))
                    .collect();
                if !references.is_empty() {
                    expressions.insert("related".into(), json!({"references": references}));
                }
            }
        }
        changes.push(json!({"address":address,"type":kind,"mode":"managed","change":{"actions":if i % 7 == 0 {vec!["update"]} else {vec!["no-op"]}}}));
        resources.push(json!({"address":address,"expressions":expressions}));
    }
    json!({"format_version":"1.2","resource_changes":changes,"configuration":{"root_module":{"resources":resources}}}).to_string()
}

fn metadata(program: &str, args: &[&str]) -> String {
    match std::process::Command::new(program).args(args).output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .trim()
            .replace(['\r', '\n'], "; "),
        Ok(out) => format!("unavailable ({})", out.status),
        Err(error) => format!("unavailable ({error})"),
    }
}

#[test]
#[ignore = "performance benchmark; run explicitly in release mode"]
fn pipeline() {
    let sizes: Vec<usize> = std::env::var("PLANORAMA_BENCH_SIZES")
        .unwrap_or_else(|_| "100,500,1000".into())
        .split(',')
        .map(|s| {
            s.trim()
                .parse()
                .expect("benchmark sizes must be positive integers")
        })
        .collect();
    assert!(sizes.iter().all(|n| *n > 0));
    let repeats: usize = std::env::var("PLANORAMA_BENCH_REPEATS")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .expect("invalid repeats");
    assert!(repeats > 0);
    println!(
        "generator=v1; revision={}; dirty={}; os={}; arch={}; debug_assertions={}; threads={:?}",
        metadata("git", &["rev-parse", "HEAD"]),
        !metadata("git", &["status", "--porcelain"]).is_empty(),
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg!(debug_assertions),
        std::thread::available_parallelism()
    );
    println!("compiler={}", metadata("rustc", &["-Vv"]));
    println!(
        "pattern,nodes,edges,run,input_bytes,input_fnv1a,svg_bytes,parse_ms,semantic_ms,layout_ms,render_ms,total_ms"
    );
    let warm = crate::semantic::transform(&crate::plan::parse(&generate(20, false)).unwrap());
    std::hint::black_box(crate::svg::render(
        &warm,
        &crate::layout::Layout::new(&warm),
    ));
    for count in sizes {
        for dense in [false, true] {
            let input = generate(count, dense);
            let hash = input.bytes().fold(0xcbf29ce484222325u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
            });
            for run in 1..=repeats {
                let start = Instant::now();
                let raw = crate::plan::parse(std::hint::black_box(&input)).unwrap();
                let parsed = Instant::now();
                let graph = crate::semantic::transform(&raw);
                let transformed = Instant::now();
                let layout = crate::layout::Layout::new(&graph);
                let laid_out = Instant::now();
                let image = std::hint::black_box(crate::svg::render(&graph, &layout));
                let rendered = Instant::now();
                let ms = |a: Instant, b: Instant| b.duration_since(a).as_secs_f64() * 1000.0;
                println!(
                    "{},{},{},{run},{},{hash:016x},{},{:.3},{:.3},{:.3},{:.3},{:.3}",
                    if dense { "dense" } else { "sparse" },
                    graph.nodes.len(),
                    graph.edges.len(),
                    input.len(),
                    image.len(),
                    ms(start, parsed),
                    ms(parsed, transformed),
                    ms(transformed, laid_out),
                    ms(laid_out, rendered),
                    ms(start, rendered)
                );
            }
        }
    }
}

#[test]
fn generator_is_repeatable_and_dense_adds_relationships() {
    assert_eq!(generate(40, true), generate(40, true));
    let sparse = crate::plan::parse(&generate(40, false)).unwrap();
    let dense = crate::plan::parse(&generate(40, true)).unwrap();
    assert_eq!(sparse.nodes.len(), 40);
    assert_eq!(dense.nodes.len(), 40);
    assert!(dense.edges.len() > sparse.edges.len() * 3);
}
