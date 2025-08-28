use envfile::EnvFile;
use log::{info, trace};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    info!("starting");

    let env =
        EnvFile::new(Path::new("/home/miuler/proyectos/statistic/.env")).unwrap_or_else(|_| {
            info!("Archivo .env no encontrado, se utilizará una configuración vacía.");
            EnvFile {
                path: PathBuf::from(".env"),              // Se agrega el campo `path`
                store: std::collections::BTreeMap::new(), // `data` se cambia a `store`
            }
        });

    let variable_count = Arc::new(AtomicUsize::new(0)); // Inicializa el contador global

    let java_src_path_str = env.get("JAVA_SRC");

    if let Some(path_str) = java_src_path_str {
        let path = Path::new(path_str);
        recursive_statistic(path, Arc::clone(&variable_count)).await?;
    } else {
        info!(
            "Variable de entorno 'JAVA_SRC' no encontrada. Usando código de constante para un solo análisis."
        );
        statistic(CODE, Arc::clone(&variable_count)).await?;
    }

    info!(
        "Total de variables encontradas: {}",
        variable_count.load(Ordering::SeqCst)
    ); // Muestra el resultado final

    Ok(()) // Asegura que main retorne Ok(())
}

// Función auxiliar para el procesamiento recursivo de archivos y directorios
async fn recursive_statistic(
    path: &Path,
    variable_count: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        info!("Procesando archivo: {:?}", path);
        let code_content = fs::read_to_string(path)?;
        statistic(&code_content, variable_count).await?;
    } else if path.is_dir() {
        info!("Explorando directorio: {:?}", path);
        for entry_result in fs::read_dir(path)? {
            let entry = entry_result?;
            let entry_path = entry.path();
            Box::pin(recursive_statistic(
                &entry_path,
                Arc::clone(&variable_count),
            ))
            .await?; // Llamada recursiva
        }
    } else {
        info!(
            "La ruta {:?} no es un archivo ni un directorio válido. Se ignora.",
            path
        );
    }
    Ok(())
}

async fn statistic(
    code: &str,
    variable_count: Arc<AtomicUsize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let tree = parser.parse(code, None).ok_or("No se pudo parsear")?;
    trace!("tree: {:?}", tree);
    let root_node = tree.root_node();
    trace!("root node: {:?}", root_node);

    let query = " (method_declaration) @method name: (identifier) @variable.name ";
    let query = r#"
    [
      (local_variable_declaration
        declarator: (variable_declarator
          name: (identifier) @variable.name))
      (field_declaration
        declarator: (variable_declarator
          name: (identifier) @variable.name))
    ]
    "#;
    let query = "[(method_declaration name: (identifier) @method.name)]";

    let query = Query::new(&tree_sitter_java::LANGUAGE.into(), query)?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root_node, code.as_bytes());

    info!("Por iterar");
    while let Some(a_match) = matches.next() {
        trace!("  a_match: {:?}", a_match);
        for capture in a_match.captures {
            trace!("    capture: {:?}", capture);
            // if query.capture_names()[capture.index as usize] == "variable.name" {
            if query.capture_names()[capture.index as usize] == "method.name" {
                let node = capture.node;
                let variable_name = &code[node.byte_range()];
                info!("      Found variable: {}", variable_name);
                variable_count.fetch_add(1, Ordering::SeqCst); // Incrementa el contador
            }
        }
    }

    Ok(())
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
