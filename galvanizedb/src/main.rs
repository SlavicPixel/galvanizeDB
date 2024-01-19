use std::io::Write;
use sqlx::{database, Pool, Result, Row, Column, Sqlite, TypeInfo};
use sqlx::sqlite::{SqlitePool, SqliteRow};

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
        let mut columns_names: Vec<String> = Vec::new();
        let mut individual_rows: Vec<Vec<String>> = Vec::new();

        for row in &rows {
            let columns = row.columns();
            let mut values: Vec<String> = Vec::new();

            for col in columns {
                let col_name = col.name();
                let col_type = col.type_info();

                if columns_names.len() < columns.len() {
                    columns_names.push(col_name.to_string());                   
                }


                let value = if col_type.name() == "TEXT" {
                    row.try_get::<String, _>(col_name).unwrap_or_default()
                } else if col_type.name() == "INTEGER" {
                    row.try_get::<i64, _>(col_name).map(|v| v.to_string()).unwrap_or_default()
                } else {
                    // TODO: add support for other types, ex. date
                    format!("Unsupported type: {}", col_type.name())
                };

                values.push(value);
                
            }
            individual_rows.push(values);
        }

        for name in columns_names {
            print!("{} ", name);
        }

        print!("\n");

        for row in individual_rows {
            for r in row {
                print!("{} ", r);
            }
            print!("\n");
        }

    } else {
        sqlx::query(sql).execute(pool).await?;
        println!("Query executed successfully.");
    }

    Ok(())
}





#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let mut database_name = get_database_name();

    let mut sql_pool = create_or_connect_database(&database_name).await?;

    loop {
        let sql = get_user_input("Enter SQL query (or 'exit' to quit):");
        if sql.to_lowercase() == "exit" {
            break;
        }

        match execute_sql(&sql_pool, &sql).await {
            Ok(_) => println!("Query executed successfully."),
            Err(e) => println!("Error executing query: {}", e),
        }
    }

    Ok(())
}
