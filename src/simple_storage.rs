use std::{
    fs,
    sync::{Arc, Mutex},
};

use rusqlite::{params, Connection, Result};

pub fn create_db(db_file: String) -> Result<Connection> {
    // This will open or create the file if it doesn't exist
    let conn = Connection::open(db_file)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tags (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL UNIQUE,
             val TEXT NOT NULL
         )",
        (),
    )?;

    Ok(conn)
}

pub fn delete_db(db_file: &str) -> Result<()> {
    if fs::metadata(db_file).is_ok() {
        // If the file exists, delete it
        fs::remove_file(db_file).unwrap();
        println!("Database file deleted.");
    } else {
        println!("Database file does not exist.");
    }
    Ok(())
}
#[derive(Clone)]
pub struct Tags {
    items: Vec<Tag>,
    connection: Arc<Mutex<Connection>>,
}
pub fn new(conn: Connection) -> Tags {
    let protect = Arc::new(Mutex::new(conn));
    let current_tags = get_tags(protect.clone()).unwrap_or(vec![]);
    Tags {
        items: current_tags,
        connection: protect,
    }
}

fn get_tags(protected: Arc<Mutex<Connection>>) -> Result<Vec<Tag>> {
    let connection = protected.lock().unwrap();
    let mut stmt = connection.prepare("SELECT name, val FROM tags")?;

    let mut rows = stmt.query(())?;
    let mut tags: Vec<Tag> = Vec::new();

    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let val: String = row.get(1)?;

        let tag = Tag { name, val };
        tags.push(tag);
    }
    return Ok(tags);
}

#[derive(Default, Debug, Clone)]
pub struct Tag {
    pub name: String, //channel/payment_request
    pub val: String,  //pubkey / payment_request string
}

impl Tags {
    pub fn save(&mut self, tag: Tag) -> Result<()> {
        let connection = self.connection.lock().unwrap();
        let sql_select = "SELECT name FROM tags WHERE name = ?1";
        let mut select_stmt = connection.prepare(&sql_select)?;

        let tag_exists = select_stmt
            .query_row(params![tag.name], |row| row.get::<_, String>(0))
            .is_ok();

        if tag_exists {
            let sql_update = "UPDATE tags SET val = ?1 WHERE name = ?2";
            let mut update_stmt = connection.prepare(&sql_update)?;
            update_stmt.execute(params![tag.val, tag.name])?;
        } else {
            let sql_insert = "INSERT INTO tags (name, val) VALUES (?1, ?2)";
            let mut insert_stmt = connection.prepare(&sql_insert)?;
            insert_stmt.execute(params![tag.name, tag.val])?;
        }
        if let Some(existing_tag) = self
            .items
            .iter_mut()
            .find(|cached_tag| cached_tag.name == tag.name)
        {
            existing_tag.val = tag.val;
        } else {
            self.items.push(tag);
        }
        Ok(())
    }

    pub fn get_all(&self) -> Vec<Tag> {
        self.items.clone()
    }

    pub fn get_by_name(&self, name: String) -> Tag {
        self.items
            .iter()
            .find(|tag| tag.name == name)
            .unwrap_or(&Tag::default())
            .clone()
    }
}
