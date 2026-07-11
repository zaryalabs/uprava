use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn unavailable_otlp_collector_does_not_break_local_logging() {
    let path = std::env::temp_dir().join(format!(
        "uprava-otlp-unavailable-{}-{}.log",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after unix epoch")
            .as_nanos()
    ));
    std::env::set_var("UPRAVA_OTLP_ENABLED", "true");
    std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:9");
    std::env::set_var("OTEL_EXPORTER_OTLP_TIMEOUT", "10");
    std::env::set_var("RUST_LOG", "info");

    uprava_logging::init_tracing("otlp-test", "UPRAVA_OTLP_TEST_LOG", &path)
        .expect("local logging initializes without a collector");
    tracing::info!(operation = "startup", "OTLP collector is optional");

    let content = (0..100)
        .find_map(|_| {
            let content = std::fs::read_to_string(&path).ok()?;
            content
                .contains("OTLP collector is optional")
                .then_some(content)
                .or_else(|| {
                    std::thread::sleep(Duration::from_millis(10));
                    None
                })
        })
        .expect("local writer receives the event");
    assert!(content.contains("OTLP collector is optional"));
    let _ = std::fs::remove_file(path);
}
