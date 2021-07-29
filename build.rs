use std::process::Command;

fn main() {
    Command::new("git")
        .args(&["submodule", "init"])
        .output()
        .expect("failed to execute submodule init");
    Command::new("git")
        .args(&["submodule", "update"])
        .output()
        .expect("failed to execute submodule update");
    tonic_build::configure()
    .build_client(true)
    .build_server(true)
    .format(true)
    .compile(&[
        "opentelemetry-proto/opentelemetry/proto/collector/logs/v1/logs_service.proto",
        "opentelemetry-proto/opentelemetry/proto/collector/metrics/v1/metrics_service.proto",
        "opentelemetry-proto/opentelemetry/proto/metrics/experimental/metrics_config_service.proto",
        "opentelemetry-proto/opentelemetry/proto/collector/trace/v1/trace_service.proto",
    ], &[
        "opentelemetry-proto"
    ]).unwrap();
}
