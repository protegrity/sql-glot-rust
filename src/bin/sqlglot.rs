use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use sqlglot_rust::parser::parse_statements;
use sqlglot_rust::{Dialect, generate, generate_pretty, optimizer, validate_dialect_support};

/// A SQL parser, optimizer, and transpiler CLI.
///
/// Transpile SQL between dialects, parse SQL into JSON AST,
/// or pretty-print SQL — all from the command line.
#[derive(Parser)]
#[command(name = "sqlglot", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transpile SQL from one dialect to another.
    Transpile {
        /// Source dialect (default: ansi).
        #[arg(long, default_value = "ansi")]
        read: String,

        /// Target dialect (default: ansi).
        #[arg(long, default_value = "ansi")]
        write: String,

        /// Pretty-print the output SQL.
        #[arg(long)]
        pretty: bool,

        /// Read SQL from a file instead of stdin.
        #[arg(long)]
        input: Option<PathBuf>,

        /// Write output to a file instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Run the optimizer before generating output.
        #[arg(long)]
        optimize: bool,
    },

    /// Parse SQL and output the AST as JSON.
    Parse {
        /// Source dialect (default: ansi).
        #[arg(long, default_value = "ansi")]
        read: String,

        /// Read SQL from a file instead of stdin.
        #[arg(long)]
        input: Option<PathBuf>,

        /// Write output to a file instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,

        /// Pretty-print the JSON output.
        #[arg(long)]
        pretty: bool,
    },

    /// Pretty-print (format) SQL.
    Format {
        /// Dialect to use for formatting (default: ansi).
        #[arg(long, default_value = "ansi")]
        read: String,

        /// Read SQL from a file instead of stdin.
        #[arg(long)]
        input: Option<PathBuf>,

        /// Write output to a file instead of stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn read_sql(input: &Option<PathBuf>) -> io::Result<String> {
    match input {
        Some(path) => std::fs::read_to_string(path),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

fn write_output(output: &Option<PathBuf>, content: &str) -> io::Result<()> {
    match output {
        Some(path) => std::fs::write(path, content),
        None => {
            io::stdout().write_all(content.as_bytes())?;
            if !content.ends_with('\n') {
                io::stdout().write_all(b"\n")?;
            }
            Ok(())
        }
    }
}

fn resolve_dialect(name: &str) -> Dialect {
    Dialect::from_str(name).unwrap_or_else(|| {
        eprintln!("error: unknown dialect '{name}'");
        eprintln!(
            "available dialects: {}",
            Dialect::all()
                .iter()
                .map(|d| format!("{d}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        process::exit(2);
    })
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Transpile {
            read,
            write,
            pretty,
            input,
            output,
            optimize,
        } => run_transpile(read, write, *pretty, input, output, *optimize),

        Commands::Parse {
            read,
            input,
            output,
            pretty,
        } => run_parse(read, input, output, *pretty),

        Commands::Format {
            read,
            input,
            output,
        } => run_format(read, input, output),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run_transpile(
    read: &str,
    write: &str,
    pretty: bool,
    input: &Option<PathBuf>,
    output: &Option<PathBuf>,
    optimize: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let sql = read_sql(input)?;
    let read_dialect = resolve_dialect(read);
    let write_dialect = resolve_dialect(write);

    let stmts = parse_statements(sql.trim(), read_dialect)?;
    let mut results = Vec::with_capacity(stmts.len());

    for stmt in stmts {
        let stmt = if optimize {
            optimizer::optimize(stmt)?
        } else {
            stmt
        };
        validate_dialect_support(&stmt, write_dialect)?;
        let generated = if pretty {
            generate_pretty(&stmt, write_dialect)
        } else {
            generate(&stmt, write_dialect)
        };
        results.push(generated);
    }

    write_output(output, &results.join(";\n"))?;
    Ok(())
}

fn run_parse(
    read: &str,
    input: &Option<PathBuf>,
    output: &Option<PathBuf>,
    pretty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let sql = read_sql(input)?;
    let dialect = resolve_dialect(read);

    let stmts = parse_statements(sql.trim(), dialect)?;
    let json = if pretty {
        serde_json::to_string_pretty(&stmts)?
    } else {
        serde_json::to_string(&stmts)?
    };

    write_output(output, &json)?;
    Ok(())
}

fn run_format(
    read: &str,
    input: &Option<PathBuf>,
    output: &Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sql = read_sql(input)?;
    let dialect = resolve_dialect(read);

    let stmts = parse_statements(sql.trim(), dialect)?;
    let mut results = Vec::with_capacity(stmts.len());

    for stmt in &stmts {
        results.push(generate_pretty(stmt, dialect));
    }

    write_output(output, &results.join(";\n"))?;
    Ok(())
}
