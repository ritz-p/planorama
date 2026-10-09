use std::path::Path;

/// Canonical paths do not detect hard links. Compare platform file identities
/// before opening the output for truncation, and fail closed on lookup errors.
pub(super) fn reject_same_file(input: &Path, output: &Path) -> Result<(), String> {
    let input_handle = same_file::Handle::from_path(input)
        .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
    let output_handle = match same_file::Handle::from_path(output) {
        Ok(handle) => handle,
        // A new output has no identity to compare yet.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "cannot inspect output {}: {error}",
                output.display()
            ));
        }
    };
    if input_handle == output_handle {
        return Err("input and output must be different files".into());
    }
    Ok(())
}
