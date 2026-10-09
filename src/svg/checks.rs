use crate::model::architecture::CheckResult;
use std::fmt::Write;

pub(super) fn render(checks: &[CheckResult]) -> String {
    // Count static objects only: instance statuses are details, not extra checks.
    let mut counts = [0usize; 6];
    let mut details = String::new();
    for check in checks {
        let index = match check.status.as_deref() {
            Some("pass") => 0,
            Some("fail") => 1,
            Some("error") => 2,
            Some("unknown") => 3,
            Some(_) => 4,
            None => 5,
        };
        counts[index] += 1;
        writeln!(
            details,
            "Object: {}",
            check.status.as_deref().unwrap_or("unavailable")
        )
        .unwrap();
        for instance in &check.instances {
            writeln!(
                details,
                "  {}: {}",
                instance
                    .resource
                    .as_deref()
                    .unwrap_or("unassociated instance"),
                instance.status.as_deref().unwrap_or("unavailable")
            )
            .unwrap();
        }
    }
    format!(
        r##"<g id="plan-checks"><title>{}</title><text x="40" y="161" font-size="12" fill="#475569">Checks (objects): {} pass / {} fail / {} error / {} unknown / {} other / {} unavailable</text></g>
"##,
        super::escape(&details),
        counts[0],
        counts[1],
        counts[2],
        counts[3],
        counts[4],
        counts[5]
    )
}
