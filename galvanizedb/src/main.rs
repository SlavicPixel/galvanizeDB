use std::io::Write;
use sqlx::{Result, Row, Column, TypeInfo};
use sqlx::sqlite::SqlitePool;
use rustyline::Editor;
use rustyline::config::Config;
use rustyline::error::ReadlineError;
use rustyline::history::MemHistory;

fn get_user_input(prompt: &str) -> String {
    print!("{}", prompt);
    
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .unwrap();

    input.trim().to_string()
}

fn get_database_name() -> String {
    let mut database_name = get_user_input("Please enter name of a new or existing database: ");
    
    if !database_name.ends_with(".db") {
        database_name.push_str(".db");
    }

    database_name

}

async fn create_or_connect_database(db_name: &str) -> Result<SqlitePool, sqlx::Error> {
    let database_url: String = format!("sqlite:{}?mode=rwc", db_name);
    let pool = SqlitePool::connect(&database_url).await?;
    Ok(pool)
}

async fn execute_sql(pool: &SqlitePool, sql: &str) -> anyhow::Result<()> {
    if sql.trim().to_lowercase().starts_with("select") {
        let rows = sqlx::query(sql).fetch_all(pool).await?;

        if rows.is_empty() {
            println!("No results found.");
            return Ok(());
        }

        let columns = rows[0].columns();
        let mut column_widths: Vec<usize> = columns.iter().map(|col| col.name().len()).collect();

        for row in &rows {
            for (i, col) in columns.iter().enumerate() {
                let length = match col.type_info().name() {
                    "TEXT" => row.try_get::<String, _>(col.name()).map(|v| v.len()).unwrap_or(0),
                    "INTEGER" => row.try_get::<i64, _>(col.name()).map(|v| v.to_string().len()).unwrap_or(0),
                    // TODO: add support for other types, ex. date
                    _ => "Unsupported type".len(),
                };
                column_widths[i] = std::cmp::max(column_widths[i], length);
            }
        }

        // Function to create a horizontal line
        let create_line = |widths: &[usize]| {
            widths
                .iter()
                .map(|w| "-".repeat(*w + 2))
                .collect::<Vec<_>>()
                .join("+")
        };

        // Print top border
        println!("+{}+", create_line(&column_widths));

        // Print header row
        for (i, col) in columns.iter().enumerate() {
            print!("| {:width$} ", col.name(), width = column_widths[i]);
        }
        println!("|");

        // Print line after header
        println!("+{}+", create_line(&column_widths));

        // Print table rows
        for row in &rows {
            for (i, col) in columns.iter().enumerate() {
                let value = match col.type_info().name() {
                    "TEXT" => row.try_get::<String, _>(col.name()).unwrap_or_default(),
                    "INTEGER" => row.try_get::<i64, _>(col.name()).map(|v| v.to_string()).unwrap_or_default(),
                    // Add other type conversions as necessary
                    _ => "Unsupported type".to_string(),
                };
                print!("| {:width$} ", value, width = column_widths[i]);
            }
            println!("|");
        }

        // Print bottom border
        println!("+{}+", create_line(&column_widths));
    } else {
        sqlx::query(sql).execute(pool).await?;
        println!("Query executed successfully.");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let config = Config::default();
    let mut rl = Editor::<(), MemHistory>::with_history(config, MemHistory::new())
        .expect("Failed to create editor");

    print!("\x1B[2J\x1B[1;1H"); 
    
    let database_name = get_database_name();
    let sql_pool = create_or_connect_database(&database_name).await?;

    loop {
        let prompt = format!("GalvanizeDB [{database_name}]> ");

        match rl.readline(&prompt) {
            Ok(line) => {
                if let Err(e) = rl.add_history_entry(line.as_str()) {
                    eprintln!("Failed to add history entry: {}", e);
                }
                if line.to_lowercase() == "exit" {
                    break;
                }
                match execute_sql(&sql_pool, &line).await {
                    Ok(_) => println!("Query executed successfully."),
                    Err(e) => println!("Error executing query: {}", e),
                }
            },
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break;
            },
            Err(err) => {
                println!("Error reading line: {:?}", err);
            }
        }
    }

    Ok(())
}
