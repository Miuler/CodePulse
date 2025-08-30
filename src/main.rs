use envfile::EnvFile;
use log::{info, debug, trace};
use opentelemetry::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing::instrument;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::util::SubscriberInitExt;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy()
            // .add_directive("opentelemetry=debug".parse().unwrap())
        // .add_directive("opentelemetry_sdk=debug".parse().unwrap())
            // .add_directive("opentelemetry_otlp=debug".parse().unwrap())
        )
        .init();
    info!("starting");

    let env =
        EnvFile::new(Path::new("/home/miuler/proyectos/statistic/.env")).unwrap_or_else(|_| {
            debug!("Archivo .env no encontrado, se utilizará una configuración vacía.");
            EnvFile {
                path: PathBuf::from(".env"),              // Se agrega el campo `path`
                store: std::collections::BTreeMap::new(), // `data` se cambia a `store`
            }
        });

    let variables_count = Arc::new(AtomicUsize::new(0)); // Inicializa el contador global
    let methods_count = Arc::new(AtomicUsize::new(0)); // Inicializa el contador global

    let java_src_path_str = env.get("JAVA_SRC");

    if let Some(path_str) = java_src_path_str {
        let path = Path::new(path_str);
        recursive_statistic(path, Arc::clone(&methods_count), Arc::clone(&variables_count)).await?;
    } else {
        debug!(
            "Variable de entorno 'JAVA_SRC' no encontrada. Usando código de constante para un solo análisis."
        );
        statistic(CODE, Arc::clone(&methods_count), TypeQuery::Method).await?;
        statistic(CODE, Arc::clone(&variables_count), TypeQuery::Method).await?;
    }

    info!("Total de metodos encontradas: {}", methods_count.load(Ordering::SeqCst)); // Muestra el resultado final
    info!("Total de variables encontradas: {}", variables_count.load(Ordering::SeqCst)); // Muestra el resultado final

    Ok(())
}

#[instrument]
// Función auxiliar para el procesamiento recursivo de archivos y directorios
async fn recursive_statistic(
    path: &Path,
    variables_count: Arc<AtomicUsize>,
    methods_count: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        debug!("Procesando archivo: {:?}", path);
        let code_content = fs::read_to_string(path)?;
        statistic(&code_content, methods_count, TypeQuery::Method).await?;
        statistic(&code_content, variables_count, TypeQuery::Variable).await?;
    } else if path.is_dir() {
        debug!("Explorando directorio: {:?}", path);
        for entry_result in fs::read_dir(path)? {
            let entry = entry_result?;
            let entry_path = entry.path();
            Box::pin(recursive_statistic(
                &entry_path,
                Arc::clone(&methods_count),
                Arc::clone(&variables_count),
            ))
            .await?; // Llamada recursiva
        }
    } else {
        debug!("La ruta {:?} no es un archivo ni un directorio válido. Se ignora.", path);
    }
    Ok(())
}

async fn statistic(
    code: &str,
    variable_count: Arc<AtomicUsize>,
    type_query: TypeQuery
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let tree = parser.parse(code, None).ok_or("No se pudo parsear")?;
    trace!("tree: {:?}", tree);
    let root_node = tree.root_node();
    trace!("root node: {:?}", root_node);

    let query = match type_query {
        TypeQuery::Variable => r#"
                [
                  (local_variable_declaration
                    declarator: (variable_declarator
                      name: (identifier) @variable.name))
                  (field_declaration
                    declarator: (variable_declarator
                      name: (identifier) @variable.name))
                ]
                "#,
        TypeQuery::Method => "[(method_declaration name: (identifier) @method.name)]"
    };

    let query = Query::new(&tree_sitter_java::LANGUAGE.into(), query)?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root_node, code.as_bytes());

    debug!("Por iterar");
    while let Some(a_match) = matches.next() {
        trace!("  a_match: {:?}", a_match);
        for capture in a_match.captures {
            trace!("    capture: {:?}", capture);
            // if query.capture_names()[capture.index as usize] == "variable.name" {
            if query.capture_names()[capture.index as usize] == "method.name" {
                let node = capture.node;
                let variable_name = &code[node.byte_range()];
                debug!("      Found variable: {}", variable_name);
                variable_count.fetch_add(1, Ordering::SeqCst); // Incrementa el contador
            }
        }
    }

    Ok(())
}

enum TypeQuery {
    Variable,
    Method,
}

const CODE: &str = r#"
        class MyClass {
            private int number = 42;

            public void aMethod() {
                String message = "Hello, Rust!";
                boolean isValid = true;
            }
        }
    "#;
