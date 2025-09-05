mod statistic;

use log::{info, debug};
use opentelemetry::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use dotenv::dotenv;
use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::util::SubscriberInitExt;
use crate::statistic::{ statistic, statistic_file, TypeQuery, CODE};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    log_and_tracing()?;
    info!("STARTING");

    let java_src_path_str = env::var("JAVA_SRC");

    if let Ok(path_str) = java_src_path_str {
        let path = Path::new(&path_str);
        statistic_file(path).await?;
    } else {
        debug!(
            "Variable de entorno 'JAVA_SRC' no encontrada. Usando código de constante para un solo análisis."
        );
        let variables_count = Arc::new(AtomicUsize::new(0));
        let methods_count = Arc::new(AtomicUsize::new(0));
        statistic(CODE, TypeQuery::Method(Arc::clone(&methods_count))).await?;
        statistic(CODE, TypeQuery::Variable(Arc::clone(&variables_count))).await?;
        info!("Total de metodos encontradas: {}", methods_count.load(Ordering::SeqCst));
        info!("Total de variables encontradas: {}", variables_count.load(Ordering::SeqCst));
    }

    Ok(())
}


fn log_and_tracing() -> Result<(), Box<dyn std::error::Error>> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .build()?;
    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_simple_exporter(span_exporter)
        .build();
    let tracer = tracer_provider.tracer(env!("CARGO_PKG_NAME"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer()
            .pretty()
        )
        // .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(EnvFilter::builder()
                  .with_default_directive(LevelFilter::INFO.into())
                  .from_env_lossy()
              // .add_directive("opentelemetry=debug".parse().unwrap())
              // .add_directive("opentelemetry_sdk=debug".parse().unwrap())
              // .add_directive("opentelemetry_otlp=debug".parse().unwrap())
        )
        .init();

    Ok(())
}