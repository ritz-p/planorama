#[path = "multi-account/generator.rs"]
mod generator;

fn main() -> Result<(), String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let destination = root.join("examples/multi-account");
    std::fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    let base = std::fs::read_to_string(root.join("examples/terraform-large/plan.json"))
        .map_err(|error| error.to_string())?;
    for (state, input) in generator::generate(&base)? {
        std::fs::write(destination.join(format!("{state}.json")), input)
            .map_err(|e| e.to_string())?;
    }
    for mapping in generator::MAPPINGS {
        println!("--remote-state {mapping}");
    }
    Ok(())
}
