use super::Error;

pub const ALLOWED_MIB: [u32; 4] = [512, 1024, 2048, 4096];
#[cfg(any(test, target_arch = "wasm32"))]
pub const DEFAULT_MIB: u32 = 512;
#[cfg(any(test, target_arch = "wasm32"))]
pub const MAX_BYTES: u64 = 4096 * 1024 * 1024;

pub fn validate(mib: u32) -> Result<u64, Error> {
    if ALLOWED_MIB.contains(&mib) {
        Ok(u64::from(mib) * 1024 * 1024)
    } else {
        Err(Error::InvalidOptions)
    }
}

pub fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<u32>, D::Error> {
    use serde::Deserialize;
    let mib = u32::deserialize(deserializer)?;
    validate(mib).map_err(serde::de::Error::custom)?;
    Ok(Some(mib))
}

#[cfg(target_arch = "wasm32")]
std::thread_local! {
    static LIMIT: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

#[cfg(target_arch = "wasm32")]
pub fn configure(mib: u32) -> Result<(), Error> {
    let bytes = validate(mib)?;
    LIMIT.with(|limit| {
        if limit.get().is_some() {
            return Err(Error::MemoryBudgetMismatch);
        }
        limit.set(Some(bytes));
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
pub fn check_options(mib: Option<u32>) -> Result<(), Error> {
    let bytes = validate(mib.unwrap_or(DEFAULT_MIB))?;
    if LIMIT.get() != Some(bytes) {
        return Err(Error::MemoryBudgetMismatch);
    }
    if available(0) {
        Ok(())
    } else {
        Err(Error::MemoryLimit)
    }
}

#[cfg(target_arch = "wasm32")]
pub fn available(additional: u64) -> bool {
    let capacity = core::arch::wasm32::memory_size::<0>() as u64 * 65536;
    LIMIT
        .get()
        .is_some_and(|limit| fits(capacity, additional, limit))
}

#[cfg(any(test, target_arch = "wasm32"))]
pub fn fits(capacity: u64, additional: u64, limit: u64) -> bool {
    capacity
        .checked_add(additional)
        .and_then(|bytes| bytes.checked_add(1024 * 1024))
        .is_some_and(|bytes| bytes <= limit)
}

#[cfg(test)]
#[path = "../../tests/unit/generation_memory.rs"]
mod tests;
