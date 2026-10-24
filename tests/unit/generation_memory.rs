use super::*;

#[test]
fn options_are_strict_and_conversion_preserves_four_gib() {
    assert_eq!(
        serde_json::from_str::<crate::Options>("{}")
            .unwrap()
            .memory_budget_mib,
        None
    );
    assert_eq!(validate(DEFAULT_MIB), Ok(536_870_912));
    for mib in ALLOWED_MIB {
        let options: crate::Options =
            serde_json::from_value(serde_json::json!({"memory_budget_mib": mib})).unwrap();
        assert_eq!(options.memory_budget_mib, Some(mib));
        assert_eq!(validate(mib).unwrap(), u64::from(mib) * 1_048_576);
        assert!(matches!(
            crate::generate("{}", &options),
            Err(Error::InvalidOptions)
        ));
        assert!(matches!(
            crate::Generator::new("{}".into(), options),
            Err(Error::InvalidOptions)
        ));
    }
    assert_eq!(validate(2048).unwrap(), 2_147_483_648);
    assert_eq!(validate(4096).unwrap(), MAX_BYTES);
    for value in [
        "null",
        "true",
        "\"512\"",
        "[]",
        "{}",
        "0",
        "-512",
        "512.0",
        "512.5",
        "513",
        "8192",
        "4294967296",
    ] {
        assert!(
            serde_json::from_str::<crate::Options>(&format!("{{\"memory_budget_mib\":{value}}}"))
                .is_err(),
            "{value}"
        );
    }
}

#[test]
fn memory_preflight_keeps_headroom_and_never_wraps() {
    let mib = 1024 * 1024;
    assert!(fits(mib, mib, 3 * mib));
    assert!(!fits(mib, mib + 1, 3 * mib));
    assert!(!fits(3 * mib, 0, 3 * mib));
    assert!(fits(MAX_BYTES - mib, 0, MAX_BYTES));
    assert!(!fits(MAX_BYTES - mib, 1, MAX_BYTES));
    assert!(!fits(u64::MAX, 1, MAX_BYTES));
}

#[test]
fn allocation_failure_inside_a_step_discards_the_job_and_allows_retry() {
    let mut job = crate::Generator {
        future: Some(Box::pin(async {
            crate::work::checkpoint().await;
            crate::work::checkpoint().await;
            crate::work::filled_vec(0_u64, usize::MAX).await;
            panic!("failed allocation must not produce a diagram");
        })),
    };
    assert!(matches!(job.step(1), Ok(None)));
    assert!(matches!(job.step(4096), Err(Error::MemoryLimit)));
    assert!(matches!(job.step(1), Err(Error::GenerationClosed)));
    let json = include_str!("../fixtures/viewer-plan.json");
    let mut retry = crate::Generator::new(json.into(), crate::Options::default()).unwrap();
    while retry.step(4096).unwrap().is_none() {}
}
