use bcrypt::{hash, verify};
use rusqlite::{params, Connection};
use warp::Reply;
use crate::{HistoryQueryParams, MessageBody, User};

pub(crate) fn handle_login(user: User) -> impl Reply {
    let conn = Connection::open("chat.db").unwrap();
    let mut stmt = conn.prepare("SELECT password FROM users WHERE username = ?").unwrap();
    let mut rows = stmt.query(params![user.username]).unwrap();
    if let Some(row) = rows.next().unwrap() {
        let db_password: String = row.get(0).unwrap();
        if verify(&user.password, &db_password).unwrap() {
            warp::reply::json(&"Login successful")
        } else {
            warp::reply::json(&"Invalid password")
        }
    } else {
        warp::reply::json(&"User not found")
    }
}

pub(crate) fn handle_register(user: User) -> impl Reply {
    let hashed_password = hash(&user.password, 4).unwrap();
    let conn = Connection::open("chat.db").unwrap();
    conn.execute("INSERT INTO users (username, password) VALUES (?1, ?2)", params![user.username, hashed_password]).unwrap();
    warp::reply::json(&"Registration successful")
}

pub(crate) fn handle_get_users() -> impl Reply {
    let conn = Connection::open("chat.db").unwrap();
    let mut stmt = conn.prepare("SELECT username FROM users").unwrap();
    let users_iter = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(0)?)
    }).unwrap();

    let users: Vec<String> = users_iter.filter_map(|result| result.ok()).collect();

    warp::reply::json(&users)
}

pub(crate) fn handle_history(params: HistoryQueryParams) -> impl Reply {
    let conn = Connection::open("chat.db").unwrap();

    let mut stmt = conn.prepare(
        "SELECT sender, receiver, message \
        FROM messages \
        WHERE (sender = ?1 AND receiver = ?2) OR (sender = ?2 AND receiver = ?1) \
        ORDER BY timestamp ASC"
    ).unwrap();

    let message_iter = stmt.query_map(
        params![params.user_from, params.user_to],
        |row| {
            Ok(MessageBody {
                sender: row.get(0)?,
                receiver: row.get(1)?,
                message: row.get(2)?,
            })
        },
    ).unwrap();

    let messages: Vec<MessageBody> = message_iter.filter_map(|result| result.ok()).collect();

    warp::reply::json(&messages)
}

pub(crate) fn insert_message_into_db(payload: &MessageBody) -> rusqlite::Result<()> {
    let conn = Connection::open("chat.db")?;

    conn.execute(
        "INSERT INTO messages (sender, receiver, message) VALUES (?1, ?2, ?3)",
        params![payload.sender, payload.receiver, payload.message],
    )?;

    Ok(())
}
