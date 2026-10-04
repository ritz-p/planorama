#[path = "terraform-large/generator.rs"]
mod generator;

fn main() -> Result<(), String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/terraform-large");
    let fixture = generator::generate(&root)?;
    std::fs::write(root.join("plan.json"), fixture).map_err(|error| error.to_string())
}
