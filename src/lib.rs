// Copyright 2020-2021, The Tremor Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The code in the `gen` folder was seed by [`tonic-build`].
//! The code in the `src` folder extends the generated code with utility
//! code to allow for convenient usage and definition of tonic-based gRPC
//! servers. Specifically, this library is designed for use by the Tremor Project
//! but has no dependencies on tremor and can be used standalone.
//!
//! This library does not provide an API or SDK designed for use as a tracing
//! facility. The official [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust) project
//! is a complete OpenTelemetry SDK designed for that purpose. It uses the same
//! underlying protocol buffer definitions and will be a better target for projects that
//! require OpenTelemetry based observability instrumentation and iteroperability with
//! the wider observability ecosystem through third party crates.
//!
//! This library is designed for system integration and interoperability and is not
//! recommended for use as a tracing SDK or for instrumentation as that is well covered
//! already by the OpenTelemetry Rust crate. For instrumentation, use the official crate.
//!
//! For those projects that need basic interworking, interoperability or integration with
//! OpenTelemetry based systems at a wire level, this project may be useful.
//!
//! ## Example
//! The complete code can be found [here]()https://github.com/tremor-rs/tremor-otelapis).
//!
//! Cargo.toml:
//! ```toml
//! [dependencies]
//! tremor-otelapis = { version = "0.1", features = ["otel-all"] }
//! tonic = { version = "0.4", features = ["tls"] }
//! prost = "0.7"
//! prost-types = "0.7"
//! tokio = { version = "1.1", features = ["rt-multi-thread", "time", "fs", "macros"] }
//! ```
//!
//! Example OpenTelemetry Log client
//!
//! ```ignore
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let channel = Endpoint::from_static("http://0.0.0.0:4316")
//!         .connect()
//!         .await?;
//!
//!     let mut client = LogsServiceClient::new(channel);
//!
//!     let resource_logs = ResourceLogs {
//!         ...
//!     };
//!
//!     client
//!         .export(ExportLogsServiceRequest {
//!             resource_logs: vec![resource_logs],
//!         })
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! Example OpenTelemetry Log Server
//!
//! ```ignore
//! fn on_logs(
//!     request: tonic::Request<ExportLogsServiceRequest>,
//! ) -> Result<tonic::Response<ExportLogsServiceResponse>, tonic::Status> {
//!     println!("Got a request from {:?}", request.remote_addr());
//!     let reply = ExportLogsServiceResponse::default();
//!     Ok(tonic::Response::new(reply))
//! }

//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let addr = "0.0.0.0:4317".parse()?;
//!     let svc = otelapis::logs::make_service(Box::new(on_logs));
//!     Server::builder().add_service(svc).serve(addr).await?;
//!
//!     Ok(())
//! }
//!
//! ```
//!
//! Example async-channel based OpenTelemetry for ease of integration with
//! async runtimes such as [tremor](https://www.tremor.rs):
//!
//! ```ignore
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let addr = "0.0.0.0:4317".parse()?;
//!     let (tx, rx) = bounded(128);
//!     let otel_collector_service = otelapis::all::make(addr, tx).await?;
//!
//!     // ...
//!
//!     loop {
//!         match rx.try_recv() {
//!             Ok(OpenTelemetryEvents::Metrics(metrics)) => {
//!                 // Do something with metrics request
//!             }
//!             Ok(OpenTelemetryEvents::Logs(log)) => {
//!                 // Do something with log request
//!             }
//!             Ok(OpenTelemetryEvents::Trace(trace)) => {
//!                 // Do something with trace request
//!             }
//!             _ => error!("Unsupported"),
//!         };
//!    }
//!
//!    // ...
//! }
//!
//! [`otelapis`]: https://github.com/open-telemetry/opentelemetry-specification
//! [`tonic-build`]: https://github.com/hyperium/tonic/tree/master/tonic-build

#![forbid(warnings)]
#![deny(
    clippy::all,
    clippy::unwrap_used,
    clippy::unnecessary_unwrap,
    // clippy::pedantic
)]

#[allow(unused_macros)]
macro_rules! include_proto {
    ($package: tt) => {
        include!(concat!("../gen", concat!("/", $package, ".rs")));
    };
}

include!("otelapis.rs");


#[cfg(feature = "otel-trace")]
/// This module defines a skeleton implementation of the open telemetry
/// collector tracing service
///
pub mod trace {
    use crate::opentelemetry::proto::collector::trace::v1 as base;
    use crate::opentelemetry::proto::collector::trace::v1::trace_service_server as skel;

    impl<S: skel::TraceService> tonic::transport::NamedService for skel::TraceServiceServer<S> {
        // NOTE This name *MUST* match the proto fqsn
        const NAME: &'static str = "opentelemetry.proto.collector.trace.v1.TraceService";
    }

    /// Alias tonic TraceRequest
    pub type OtelTraceRequest = tonic::Request<base::ExportTraceServiceRequest>;

    /// Alias tonic TraceResponse
    pub type OtelTraceResponse = tonic::Response<base::ExportTraceServiceResponse>;

    /// Alias the generated server skeletons
    pub use skel::TraceServiceServer;

    /// Alias trace callback fn
    pub type OnTraceFn =
        dyn Fn(OtelTraceRequest) -> Result<OtelTraceResponse, tonic::Status> + Send + Sync + 'static;

    /// GRPC trace service skeleton
    pub struct OtelTraceService {
        on_trace: Box<OnTraceFn>,
    }

    impl OtelTraceService {
        /// Creates a trace service with the specified trace event handler function
        pub fn with_handler(handler: Box<OnTraceFn>) -> Self {
            OtelTraceService { on_trace: handler }
        }
    }

    /// Creates a tonic service handler for open telemetry trace events
    pub fn make_service(handler: Box<OnTraceFn>) -> skel::TraceServiceServer<OtelTraceService> {
        skel::TraceServiceServer::new(OtelTraceService::with_handler(handler))
    }

    #[tonic::async_trait]
    impl skel::TraceService for OtelTraceService {
        async fn export(
            &self,
            request: tonic::Request<base::ExportTraceServiceRequest>,
        ) -> Result<tonic::Response<base::ExportTraceServiceResponse>, tonic::Status> {
            (self.on_trace)(request)
        }
    }
}

/// This module defines a skeleton implementation of the open telemetry
/// collector logging service
///
#[cfg(feature = "otel-logs")]
pub mod logs {
    use crate::opentelemetry::proto::collector::logs::v1 as base;
    use crate::opentelemetry::proto::collector::logs::v1::logs_service_server as skel;
    use async_channel::{Receiver, Sender};

    impl<S: skel::LogsService> tonic::transport::NamedService for skel::LogsServiceServer<S> {
        // NOTE This name *MUST* match the proto fqsn
        const NAME: &'static str = "opentelemetry.proto.collector.logs.v1.LogsService";
    }

    /// Alias tonic request
    pub type OtelLogsRequest = tonic::Request<base::ExportLogsServiceRequest>;

    /// Alias tonic reponse
    pub type OtelLogsResponse = tonic::Response<base::ExportLogsServiceResponse>;

    /// Alias service skeleton
    pub use skel::LogsService;

    /// Alias logs server
    pub use skel::LogsServiceServer;

    /// Alias logs callback fn
    pub type OnLogsFn =
        dyn Fn(OtelLogsRequest) -> Result<OtelLogsResponse, tonic::Status> + Send + Sync + 'static;

    /// GRPC logs service skeleton
    pub struct OtelLogsService {
        on_logs: Box<OnLogsFn>,
    }

    impl OtelLogsService {
        /// Creates a logs service with the specified logs event handler function
        pub fn with_handler(handler: Box<OnLogsFn>) -> Self {
            OtelLogsService { on_logs: handler }
        }
    }

    #[tonic::async_trait]
    impl skel::LogsService for OtelLogsService {
        async fn export(
            &self,
            request: tonic::Request<base::ExportLogsServiceRequest>,
        ) -> Result<tonic::Response<base::ExportLogsServiceResponse>, tonic::Status> {
            (self.on_logs)(request)
        }
    }

    /// Creates a tonic service handler for open telemetry log events
    pub fn make_service(handler: Box<OnLogsFn>) -> skel::LogsServiceServer<OtelLogsService> {
        skel::LogsServiceServer::new(OtelLogsService::with_handler(handler))
    }

    /// Asynchronous channel sender
    pub type OtelLogsSender = Sender<base::ExportLogsServiceRequest>;

    /// Asynchronous channel receiver
    pub type OtelLogsReceiver = Receiver<base::ExportLogsServiceRequest>;

    /// Logs forwarding agent
    pub struct OtelLogsServiceForwarder {
        channel: Sender<base::ExportLogsServiceRequest>,
    }

    // Creates a metrics service with the specified asynchronous sender channel
    impl OtelLogsServiceForwarder {
        /// Creates a log forwarding agent with an asynchronous channel sender
        pub fn with_sender(channel: Sender<base::ExportLogsServiceRequest>) -> Self {
            OtelLogsServiceForwarder {
                channel,
            }
        }
    }

    #[tonic::async_trait]
    impl skel::LogsService for OtelLogsServiceForwarder {
        async fn export(
            &self,
            request: tonic::Request<base::ExportLogsServiceRequest>,
        ) -> Result<tonic::Response<base::ExportLogsServiceResponse>, tonic::Status> {
            match self.channel.send(request.into_inner()).await {
                Ok(_) => Ok(tonic::Response::new(base::ExportLogsServiceResponse {})),
                Err(e) => Err(tonic::Status::internal(&format!(
                    "Logs gRPC forwarder channel sender failed to dispatch {}",
                    e
                ))),
            }
        }
    }

    /// Creates a tonic service handler for open telemetry logs events
    pub fn make_forwarder(sender: OtelLogsSender) -> skel::LogsServiceServer<OtelLogsServiceForwarder> {
        skel::LogsServiceServer::new(OtelLogsServiceForwarder::with_sender(sender))
    }
}

/// This module defines a skeleton implementation of the open telemetry
/// collector metrics service
///
#[cfg(feature = "otel-metrics")]
pub mod metrics {
    use crate::opentelemetry::proto::collector::metrics::v1 as base;
    use crate::opentelemetry::proto::collector::metrics::v1::metrics_service_server as skel;
    use async_channel::{Receiver, Sender};

    pub use skel::MetricsService;
    pub use skel::MetricsServiceServer;

    impl<S: skel::MetricsService> tonic::transport::NamedService for skel::MetricsServiceServer<S> {
        // NOTE This name *MUST* match the proto fqsn
        const NAME: &'static str = "opentelemetry.proto.collector.metrics.v1.MetricsService";
    }

    /// Alias tonic request
    pub type OtelMetricsRequest = tonic::Request<base::ExportMetricsServiceRequest>;

    /// Alias tonic response
    pub type OtelMetricsResponse = tonic::Response<base::ExportMetricsServiceResponse>;

    /// Alias metrics callback fn
    pub type OnMetricsFn =
        dyn Fn(OtelMetricsRequest) -> Result<OtelMetricsResponse, tonic::Status> + Send + Sync + 'static;

    /// GRPC metrics service skeleton
    pub struct OtelMetricsService {
        on_metrics: Box<OnMetricsFn>,
    }

    impl OtelMetricsService {
        /// Creates a metrics service with the specified metrics event handler function
        pub fn with_handler(handler: Box<OnMetricsFn>) -> Self {
            OtelMetricsService {
                on_metrics: handler,
            }
        }
    }

    #[tonic::async_trait]
    impl skel::MetricsService for OtelMetricsService {
        async fn export(
            &self,
            request: tonic::Request<base::ExportMetricsServiceRequest>,
        ) -> Result<tonic::Response<base::ExportMetricsServiceResponse>, tonic::Status> {
            (self.on_metrics)(request)
        }
    }

    /// Creates a tonic service handler for open telemetry metrics events
    pub fn make_service(
        handler: Box<OnMetricsFn>,
    ) -> skel::MetricsServiceServer<OtelMetricsService> {
        skel::MetricsServiceServer::new(OtelMetricsService::with_handler(handler))
    }

    /// Asynchronous channel sender
    pub type OtelMetricsSender = Sender<base::ExportMetricsServiceRequest>;

    /// Asynchronous channel receiver
    pub type OtelMetricsReceiver = Receiver<base::ExportMetricsServiceRequest>;

    /// Creates a metrics service with the specified asynchronous sender channel
    pub struct OtelMetricsServiceForwarder {
        channel: Sender<base::ExportMetricsServiceRequest>,
    }

    impl OtelMetricsServiceForwarder {
        /// Creates a metrics service forwarding agent with an asynchronous channel sender
        pub fn with_sender(channel: Sender<base::ExportMetricsServiceRequest>) -> Self {
            OtelMetricsServiceForwarder {
                channel,
            }
        }
    }

    #[tonic::async_trait]
    impl skel::MetricsService for OtelMetricsServiceForwarder {
        async fn export(
            &self,
            request: tonic::Request<base::ExportMetricsServiceRequest>,
        ) -> Result<tonic::Response<base::ExportMetricsServiceResponse>, tonic::Status> {
            match self.channel.send(request.into_inner()).await {
                Ok(_) => Ok(tonic::Response::new(base::ExportMetricsServiceResponse {})),
                Err(e) => Err(tonic::Status::internal(&format!(
                    "Metrics gRPC forwarder channel sender failed to dispatch {}",
                    e
                ))),
            }
        }
    }

    /// Creates a tonic service forwarder for open telemetry metrics events
    pub fn make_forwarder(
        sender: OtelMetricsSender,
    ) -> skel::MetricsServiceServer<OtelMetricsServiceForwarder> {
        skel::MetricsServiceServer::new(OtelMetricsServiceForwarder::with_sender(sender))
    }
}

/// A unified set of services that provide log, metrics and trace events
#[cfg(feature = "otel-all")]
pub mod all {
    use crate::opentelemetry::proto::collector::logs::v1 as logs_base;
    use crate::opentelemetry::proto::collector::metrics::v1 as metrics_base;
    use crate::opentelemetry::proto::collector::trace::v1 as trace_base;
    use async_channel::{Receiver, Sender};
    use std::net::SocketAddr;
    use tonic::transport::Server;

    /// Enumeration of protocol buffer messages that are sendable/receivable
    pub enum OpenTelemetryEvents {
        /// A logs export request
        Logs(logs_base::ExportLogsServiceRequest),
        /// A metrics export request
        Metrics(metrics_base::ExportMetricsServiceRequest),
        /// A trace export request
        Trace(trace_base::ExportTraceServiceRequest),
    }

    /// Alias receiver
    pub type OpenTelemetrySender = Sender<OpenTelemetryEvents>;

    /// Alias sender
    pub type OpenTelemetryReceiver = Receiver<OpenTelemetryEvents>;

    /// Creates a logs service with the specified asynchronous sender channel
    pub struct LogsServiceForwarder {
        channel: Sender<OpenTelemetryEvents>,
    }

    impl LogsServiceForwarder {
        /// Creates a logs service forwarding agent
        pub fn with_sender(channel: Sender<OpenTelemetryEvents>) -> Self {
            LogsServiceForwarder {
                channel,
            }
        }
    }

    #[tonic::async_trait]
    impl super::logs::LogsService for LogsServiceForwarder {
        async fn export(
            &self,
            request: tonic::Request<logs_base::ExportLogsServiceRequest>,
        ) -> Result<tonic::Response<logs_base::ExportLogsServiceResponse>, tonic::Status> {
            match self
                .channel
                .send(OpenTelemetryEvents::Logs(request.into_inner()))
                .await
            {
                Ok(_) => Ok(tonic::Response::new(
                    logs_base::ExportLogsServiceResponse {},
                )),
                Err(e) => Err(tonic::Status::internal(&format!(
                    "Logs gRPC forwarder channel sender failed to dispatch {}",
                    e
                ))),
            }
        }
    }

    /// Creates a metrics service with the specified asynchronous sender channel
    pub struct MetricsServiceForwarder {
        channel: Sender<OpenTelemetryEvents>,
    }

    impl MetricsServiceForwarder {
        /// Creates a metrics service forwarding agent
        pub fn with_sender(channel: Sender<OpenTelemetryEvents>) -> Self {
            MetricsServiceForwarder {
                channel,
            }
        }
    }

    #[tonic::async_trait]
    impl super::metrics::MetricsService for MetricsServiceForwarder {
        async fn export(
            &self,
            request: tonic::Request<metrics_base::ExportMetricsServiceRequest>,
        ) -> Result<tonic::Response<metrics_base::ExportMetricsServiceResponse>, tonic::Status>
        {
            match self
                .channel
                .send(OpenTelemetryEvents::Metrics(request.into_inner()))
                .await
            {
                Ok(_) => Ok(tonic::Response::new(
                    metrics_base::ExportMetricsServiceResponse {},
                )),
                Err(e) => Err(tonic::Status::internal(&format!(
                    "Metrics gRPC forwarder channel sender failed to dispatch {}",
                    e
                ))),
            }
        }
    }

    /// Creates a trace service with the specified asynchronous sender channel
    pub struct TraceServiceForwarder {
        channel: Sender<OpenTelemetryEvents>,
    }

    impl TraceServiceForwarder {
        /// Creates a trace service forwarding agent
        pub fn with_sender(channel: Sender<OpenTelemetryEvents>) -> Self {
            TraceServiceForwarder {
                channel,
            }
        }
    }

    #[tonic::async_trait]
    impl trace_base::trace_service_server::TraceService for TraceServiceForwarder {
        async fn export(
            &self,
            request: tonic::Request<trace_base::ExportTraceServiceRequest>,
        ) -> Result<tonic::Response<trace_base::ExportTraceServiceResponse>, tonic::Status>
        {
            match self
                .channel
                .send(OpenTelemetryEvents::Trace(request.into_inner()))
                .await
            {
                Ok(_) => Ok(tonic::Response::new(
                    trace_base::ExportTraceServiceResponse {},
                )),
                Err(e) => Err(tonic::Status::internal(&format!(
                    "Trace gRPC forwarder channel sender failed to dispatch {}",
                    e
                ))),
            }
        }
    }

    /// Spins up a `gRPC OpenTelemetry Collector` instance
    pub async fn make(
        addr: SocketAddr,
        sender: Sender<OpenTelemetryEvents>,
    ) -> Result<(), tonic::transport::Error> {
        Server::builder()
            .add_service(super::trace::TraceServiceServer::new(
                TraceServiceForwarder::with_sender(sender.clone()),
            ))
            .add_service(super::logs::LogsServiceServer::new(
                LogsServiceForwarder::with_sender(sender.clone()),
            ))
            .add_service(super::metrics::MetricsServiceServer::new(
                MetricsServiceForwarder::with_sender(sender),
            ))
            .serve(addr)
            .await
    }
}
