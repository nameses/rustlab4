use warp::Filter;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use serde_derive::{Deserialize, Serialize};
use futures::{StreamExt, SinkExt, FutureExt};
use warp::ws::{Message};

use crate::handlers::ws_handler;
use crate::handlers::user_handlers;

pub mod handlers;

#[derive(Serialize, Deserialize)]
struct User
{
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct MessageBody
{
    sender: String,
    receiver: String,
    message: String,
}

#[derive(Deserialize)]
struct HistoryQueryParams
{
    user_from: String,
    user_to: String,
}

type Users = Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

#[tokio::main]
async fn main() {
    let conn = Connection::open("chat.db").unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS users (username TEXT PRIMARY KEY, password TEXT NOT NULL)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS messages (sender TEXT, receiver TEXT, message TEXT, timestamp DATETIME DEFAULT CURRENT_TIMESTAMP)", []).unwrap();

    let login = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .map(move |user: User| user_handlers::handle_login(user));

    let register = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .map(move |user: User| user_handlers::handle_register(user));

    let get_users = warp::path("users")
        .and(warp::get())
        .map(move || user_handlers::handle_get_users());

    let get_history = warp::path("history")
        .and(warp::get())
        .and(warp::query::<HistoryQueryParams>())
        .map(|p: HistoryQueryParams| {
            user_handlers::handle_history(p)
        });

    let users = Users::default();

    let chat = warp::path("chat")
        .and(warp::ws())
        .and(with_users(users.clone()))
        .map(|ws: warp::ws::Ws, users| {
            ws.on_upgrade(move |socket| ws_handler::handle_connection(socket, users))
        });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS", "PUT", "DELETE"])
        .allow_headers(vec!["Content-Type"]);

    let routes = login.or(register).or(get_users).or(get_history).or(chat).with(cors);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

fn with_users(
    users: Users,
) -> impl Filter<Extract = (Users,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || users.clone())
}
