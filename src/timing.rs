use std::cell::RefCell;
use std::io::Write;
use std::time::{Duration, Instant};

thread_local! {
    static REPORT: RefCell<Option<Report>> = const { RefCell::new(None) };
}

struct Report {
    started: Instant,
    rows: Vec<Row>,
    stack: Vec<usize>,
}

struct Row {
    stage: &'static str,
    parent: Option<usize>,
    elapsed: Duration,
    children: Duration,
    thread_cpu: Option<Duration>,
    process_peak_rss_kib: Option<u64>,
    counters: Vec<(&'static str, usize)>,
}

pub(crate) struct Session;

impl Session {
    pub(crate) fn start(enabled: bool) -> Self {
        if enabled {
            REPORT.with(|report| {
                *report.borrow_mut() = Some(Report {
                    started: Instant::now(),
                    rows: Vec::new(),
                    stack: Vec::new(),
                });
            });
        }
        Self
    }

    pub(crate) fn finish(&self) -> Result<(), String> {
        let report = REPORT.with(|report| report.borrow_mut().take());
        let Some(report) = report else { return Ok(()) };
        let total = report.started.elapsed();
        let write = || -> std::io::Result<()> {
            let mut stderr = std::io::stderr().lock();
            writeln!(
                stderr,
                "timings: total_ms={:.3} (wall clock; inclusive rows overlap; exclusive excludes children)",
                total.as_secs_f64() * 1000.0
            )?;
            for (index, row) in report.rows.iter().enumerate() {
                write!(
                    stderr,
                    "timing: id={} parent={} stage={} inclusive_ms={:.3} exclusive_ms={:.3}",
                    index + 1,
                    row.parent.map_or(0, |p| p + 1),
                    row.stage,
                    row.elapsed.as_secs_f64() * 1000.0,
                    row.elapsed.saturating_sub(row.children).as_secs_f64() * 1000.0
                )?;
                if let Some(cpu) = row.thread_cpu {
                    write!(stderr, " thread_cpu_ms={:.3}", cpu.as_secs_f64() * 1000.0)?;
                }
                if let Some(rss) = row.process_peak_rss_kib {
                    write!(stderr, " process_peak_rss_kib_at_end={rss}")?;
                }
                for (key, value) in &row.counters {
                    write!(stderr, " {key}={value}")?;
                }
                writeln!(stderr)?;
            }
            Ok(())
        };
        write().map_err(|error| format!("cannot write timings: {error}"))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        REPORT.with(|report| {
            report.borrow_mut().take();
        });
    }
}

fn thread_cpu() -> Option<Duration> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
        Some(Duration::from_nanos(
            stat.split_whitespace().next()?.parse().ok()?,
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn process_peak_rss_kib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        status
            .lines()
            .find_map(|line| line.strip_prefix("VmHWM:"))?
            .trim()
            .strip_suffix(" kB")?
            .trim()
            .parse()
            .ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

pub(crate) struct Span(Option<(Instant, Option<Duration>)>);

pub(crate) fn span(stage: &'static str, counters: &[(&'static str, usize)]) -> Span {
    REPORT.with(|report| {
        let mut report = report.borrow_mut();
        let Some(report) = report.as_mut() else {
            return Span(None);
        };
        let index = report.rows.len();
        report.rows.push(Row {
            stage,
            parent: report.stack.last().copied(),
            elapsed: Duration::ZERO,
            thread_cpu: None,
            process_peak_rss_kib: None,
            children: Duration::ZERO,
            counters: counters.to_vec(),
        });
        report.stack.push(index);
        Span(Some((Instant::now(), thread_cpu())))
    })
}

impl Drop for Span {
    fn drop(&mut self) {
        let Some((started, cpu)) = self.0 else { return };
        let elapsed = started.elapsed();
        REPORT.with(|report| {
            if let Some(report) = report.borrow_mut().as_mut() {
                if let Some(index) = report.stack.pop() {
                    report.rows[index].elapsed = elapsed;
                    report.rows[index].thread_cpu =
                        cpu.and_then(|start| thread_cpu()?.checked_sub(start));
                    report.rows[index].process_peak_rss_kib = process_peak_rss_kib();
                    if let Some(parent) = report.stack.last().copied() {
                        report.rows[parent].children += elapsed;
                    }
                }
            }
        });
    }
}
