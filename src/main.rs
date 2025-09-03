mod statistic;

use envfile::EnvFile;
use log::{info, debug};
use opentelemetry::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::util::SubscriberInitExt;
use crate::statistic::{ statistic, statistic_file, TypeQuery, CODE};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log_and_tracing()?;
    info!("STARTING");

    let env =
        EnvFile::new(Path::new(".env")).unwrap_or_else(|_| {
            debug!("Archivo .env no encontrado, se utilizará una configuración vacía.");
            EnvFile {
                path: PathBuf::from(".env"),              // Se agrega el campo `path`
                store: std::collections::BTreeMap::new(), // `data` se cambia a `store`
            }
        });

    let java_src_path_str = env.get("JAVA_SRC");

    if let Some(path_str) = java_src_path_str {
        let path = Path::new(path_str);
        statistic_file(path).await?;
    } else {
        debug!(
            "Variable de entorno 'JAVA_SRC' no encontrada. Usando código de constante para un solo análisis."
        );
        let variables_count = Arc::new(AtomicUsize::new(0)); // Inicializa el contador global
        let methods_count = Arc::new(AtomicUsize::new(0)); // Inicializa el contador global
        statistic(CODE, TypeQuery::Method(Arc::clone(&methods_count))).await?;
        statistic(CODE, TypeQuery::Variable(Arc::clone(&variables_count))).await?;
        info!("Total de metodos encontradas: {}", methods_count.load(Ordering::SeqCst)); // Muestra el resultado final
        info!("Total de variables encontradas: {}", variables_count.load(Ordering::SeqCst)); // Muestra el resultado final
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