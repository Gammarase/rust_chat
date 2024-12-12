use rusqlite::{Connection, Result};

fn main() -> Result<()> {
    let connection = Connection::open("src/database/main.db")?;

    connection.execute(
        "
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY,
            chatname TEXT NOT NULL,
            content TEXT NOT NULL,
            sended_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        ",
        []
    )?;

    println!("Messages table created!");
    Ok(())
}
