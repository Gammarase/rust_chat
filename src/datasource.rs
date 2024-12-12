use rusqlite::{Connection, Result, params};

pub struct Datasource {
    conn: Connection,
}

impl Datasource {
    pub fn new() -> Result<Self> {
        let conn = Connection::open("src/database/main.db")?;
        
        Ok(Datasource { conn })
    }  

    pub fn store_message(&self, chatname: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (chatname, content) VALUES (?1, ?2);",
            params![chatname, content],
        )?;
        Ok(())
    }

    pub fn get_messages_by_chatname(&self, chatname: &str) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT chatname, content, sended_at FROM messages WHERE chatname = ?1 ORDER BY sended_at ASC;",
        )?;

        let messages = stmt
            .query_map(params![chatname], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }
}
