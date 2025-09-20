use log::{debug, info, trace};
use std::fs::{self, File};
use std::path::{self, Path};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use comfy_table::presets::UTF8_FULL;
use comfy_table::Table;
use tokio::sync::Mutex;
use tracing::instrument;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use std::io::Write;

#[instrument(level = "info")]
pub async fn statistic_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let methods_count = Arc::new(AtomicUsize::new(0));
    let variables_count = Arc::new(AtomicUsize::new(0));
    let mut output_file = Arc::new(Mutex::new(fs::File::create("statistics.txt")?));


    let result = recursive_statistic(
        path,
        Arc::clone(&methods_count),
        Arc::clone(&variables_count),
        Arc::clone(&output_file),
    ).await;

    // info!("Total de metodos encontradas: {}", methods_count.load(Ordering::SeqCst));
    // info!("Total de variables encontradas: {}", variables_count.load(Ordering::SeqCst));

    let mut table = Table::new();
    table
        .set_header(vec!["Metrica", "Cantidad"])
        .add_row(vec!["Metodos", &methods_count.load(Ordering::SeqCst).to_string()])
        .add_row(vec!["Variables", &variables_count.load(Ordering::SeqCst).to_string()]);
    table.load_preset(UTF8_FULL);

    result
}

#[instrument(level = "info")]
async fn recursive_statistic(
    path: &Path,
    methods_count: Arc<AtomicUsize>,
    variables_count: Arc<AtomicUsize>,
    output_file: Arc<Mutex<File>>,
) -> Result<(), Box<dyn std::error::Error>> {

    if path.is_file() {
        debug!("Procesando archivo: {:?}", path);
        let filename = path.file_name().map(|f| f.to_str().unwrap_or("")).unwrap_or("");
        let code_content = fs::read_to_string(path)?;
        statistic(&code_content, TypeQuery::Method(Arc::clone(&methods_count)), filename, Arc::clone(&output_file)).await?;
        statistic(&code_content, TypeQuery::Variable(Arc::clone(&variables_count)), filename, Arc::clone(&output_file)).await?;
    } else if path.is_dir() {
        debug!("Explorando directorio: {:?}", path);
        for entry_result in fs::read_dir(path)? {
            let entry = entry_result?;
            let entry_path = entry.path();
            Box::pin(recursive_statistic(
                &entry_path,
                Arc::clone(&methods_count),
                Arc::clone(&variables_count),
                Arc::clone(&output_file),
            )).await?; // Llamada recursiva
        }
    } else {
        debug!("La ruta {:?} no es un archivo ni un directorio válido. Se ignora.", path);
    }

    Ok(())
}

pub async fn statistic(
    code: &str,
    type_query: TypeQuery,
    filename: &str,
    output_file: Arc<Mutex<File>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let names = vec!["method.name", "variable.name"];
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

    let tree = parser.parse(code, None).ok_or("No se pudo parsear")?;
    trace!("tree: {:?}", tree);
    let root_node = tree.root_node();
    trace!("root node: {:?}", root_node);

    let (query, count) = match &type_query {
        TypeQuery::Variable(count) => (r#"
                [
                  (local_variable_declaration
                    declarator: (variable_declarator
                      name: (identifier) @variable.name))
                  (field_declaration
                    declarator: (variable_declarator
                      name: (identifier) @variable.name))
                ]
                "#, count),
        TypeQuery::Method(count) => ("[(method_declaration name: (identifier) @method.name)]", count)
    };

    let query = Query::new(&tree_sitter_java::LANGUAGE.into(), query)?;

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root_node, code.as_bytes());

    debug!("Por iterar");
    while let Some(a_match) = matches.next() {
        trace!("  a_match: {:?}", a_match);
        for capture in a_match.captures {
            trace!("    capture: {:?}", capture);
            let _names = query.capture_names()[capture.index as usize];
            trace!("     name {}", _names);
            if names.contains(&_names) {
                let node = capture.node;
                let variable_name = &code[node.byte_range()];
                let found = format!("{}, {}, {}, {}", type_query.name(), type_query.count(), filename, variable_name);
                writeln!(output_file.lock().await, "{}", &found)?;
                info!("      {}", found);
                count.fetch_add(1, Ordering::SeqCst); // Incrementa el contador
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub enum TypeQuery {
    Variable(Arc<AtomicUsize>),
    Method(Arc<AtomicUsize>),
}

impl TypeQuery {
    // Método para obtener el nombre del tipo como un string
    pub fn name(&self) -> &'static str {
        match self {
            TypeQuery::Variable(_) => "Variable",
            TypeQuery::Method(_) => "Method",
        }
    }

    // Método para obtener el Arc<AtomicUsize> asociado
    pub fn count(&self) -> usize {
        match self {
            TypeQuery::Variable(arc) => arc.load(Ordering::SeqCst),
            TypeQuery::Method(arc) => arc.load(Ordering::SeqCst),
        }
    }
}

pub const CODE: &str = r#"
        class MyClass {
            private int number = 42;

            public void aMethod() {
                String message = "Hello, Rust!";
                boolean isValid = true;
            }
        }
    "#;