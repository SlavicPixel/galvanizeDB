use std::path::Path;
use sqlx::{Column, Result, Row, TypeInfo};
use sqlx::sqlite::SqlitePool;
use rustyline::Editor;
use rustyline::config::Config;
use rustyline::error::ReadlineError;
use rustyline::history::MemHistory;

fn extract_db_name(input: &str) -> Option<String> {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.len() >= 2 {
        let command = parts[0].to_uppercase();

        if command == "USE" && parts.len() == 2 {
            let database_name = parts[1].strip_suffix(';').unwrap_or(parts[1]);
            Some(format_db_name(database_name))
        } else if (command == "CREATE" || command == "DROP") && parts.len() >= 3 && parts[1].eq_ignore_ascii_case("database") {
            let database_name = parts[2].strip_suffix(';').unwrap_or(parts[2]);
            Some(format_db_name(database_name))
        } else {
            None
        }
    } else {
        None
    }
}

fn format_db_name(name: &str) -> String {
    let mut formatted_name = name.to_string();

    if !formatted_name.ends_with(".db") {
        formatted_name.push_str(".db");
    }

    formatted_name
}

fn db_file_check(db_file_name: &str) -> bool {
    let path = Path::new(&db_file_name);

    if path.exists(){
        return true;
    }
    return false;
}

fn help() {
    println!(
        "\nGalvanizeDB Basic Manual\n\
        ---------------------------\n\
        Create a database:\n    CREATE DATABASE database_name;\n\n\
        Connect to a database:\n    USE database_name;\n\n\
        List tables in a database:\n    SHOW TABLES;\n\n\
        Close connection to a database:\n    DROP SCHEMA database_name;\n\n\
        When connected to a database, use standard SQLite queries to interact with the database.\n\n\
        Type 'exit' to close GalvanizeDB CLI.\n\n\
        Report issues at: https://github.com/SlavicPixel/galvanizedb\n"
    );
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
                    _ => "Unsupported type".len(),
                };
                column_widths[i] = std::cmp::max(column_widths[i], length);
            }
        }

        // Print horizontal line
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
                    _ => {
                        row.try_get::<f64, _>(col.name()).map(|v| v.to_string())
                            .unwrap_or_else(|_| "Unsupported type".to_string())
                    },
                };
                print!("| {:width$} ", value, width = column_widths[i]);
            }
            println!("|");
        }

        // Print bottom border
        println!("+{}+", create_line(&column_widths));
    } else {
        sqlx::query(sql).execute(pool).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::default();
    let mut rl = Editor::<(), MemHistory>::with_history(config, MemHistory::new())
        .expect("Failed to create editor");

    //print!("\x1B[2J\x1B[1;1H"); // clears the terminal
    
    let mut database_name = "None".to_string();
    let mut sql_pool: Option<SqlitePool> = None;

    println!("Welcome to the GalvanizeDB CLI. Type help or ? to list commands.\n");

    loop {
        let prompt = format!("GalvanizeDB [{}]> ", database_name);

        match rl.readline(&prompt) {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());

                if line.to_lowercase().starts_with("use ") || line.to_lowercase().starts_with("create database "){
                    if let Some(active_database_name) = extract_db_name(&line) {
                        database_name = active_database_name;
                        if !db_file_check(&database_name) && line.to_lowercase().starts_with("use "){
                            println!("{} does not exist. \nAttempting to create {}", database_name, database_name);
                        }
                        match create_or_connect_database(&database_name).await {
                            Ok(pool) => {
                                if line.to_lowercase().starts_with("create database ") {
                                    println!("{} successfully created.", database_name);
                                }
                                println!("Database connection established to '{}'.\n", database_name);
                                sql_pool = Some(pool);
                            },
                            Err(e) => {
                                eprintln!("Error connecting to database '{}': {}\n", database_name, e);
                                sql_pool = None; // Reset the pool if connection fails
                            }
                        }
                    } else {
                        eprintln!("Invalid database name.");
                    }
                }
                else if line.to_lowercase().starts_with("drop schema ") {
                    if let Some(pool) = &sql_pool {
                        println!("Closing database connection...");
                        pool.close().await;
                        println!("Connection closed.\n");
                        database_name = "None".to_string();
                    }
                }
                else if line.to_lowercase() == "show tables;" {
                    if let Some(pool) = &sql_pool {
                        let show_tables_query = "SELECT name FROM sqlite_master WHERE type='table';";
                        match execute_sql(pool, show_tables_query).await {
                            Ok(_) => println!("\nQuery executed successfully.\n"),
                            Err(e) => println!("\nError executing query: {}\n", e),
                        }
                    } else {
                        println!("No database selected.");
                    }
                }
                else if line.to_lowercase().starts_with("drop database ") {
                    if let Some(new_database_name) = extract_db_name(&line) {
                        if let Some(pool) = &sql_pool {
                            println!("Closing database connection...");
                            pool.close().await;
                            println!("Connection closed.");
                        }
                
                        let db_file_path = format!("{}", new_database_name);
                        match std::fs::remove_file(&db_file_path) {
                            Ok(_) => println!("Database '{}' dropped successfully.", new_database_name),
                            Err(e) => eprintln!("Error dropping database '{}': {}", new_database_name, e),
                        }
                
                        database_name = "None".to_string();
                        sql_pool = None;
                    } else {
                        eprintln!("Invalid database name.");
                    }
                }
                else if line.to_lowercase() == "help" || line == "?" {
                    help();
                }
                else if line.to_lowercase() == "exit" {
                    if let Some(pool) = sql_pool {
                        println!("Closing database connection...");
                        pool.close().await;
                        println!("Connection closed.");
                    }
                    break;
                } else {
                    if let Some(pool) = &sql_pool {
                        match execute_sql(pool, &line).await {
                            Ok(_) => println!("\nQuery executed successfully.\n"),
                            Err(e) => println!("\nError executing query: {}\n", e),
                        }
                    } else {
                        println!("No database selected.");
                    }
                }
            },
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                if let Some(pool) = sql_pool {
                    println!("Closing database connection due to interruption...");
                    pool.close().await;
                    println!("Connection closed.");
                }
                break;
            },
            Err(err) => {
                println!("Error reading line: {:?}", err);
            }
        }
    }

    Ok(())
}
