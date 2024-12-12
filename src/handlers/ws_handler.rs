use futures_util::StreamExt;
use futures_util::FutureExt;
use warp::ws::{Message, WebSocket};
use crate::handlers::user_handlers::insert_message_into_db;
use crate::{MessageBody, Users};

pub(crate) async fn handle_connection(ws: WebSocket, users: Users) {
    let (user_ws_tx, mut user_ws_rx) = ws.split();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(errorTxt) = result {
            eprintln!("Error sending message. {}", errorTxt);
        }
    }));

    users.lock().unwrap().push(tx);

    while let Some(result) = user_ws_rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    if let Ok(messageBody) = serde_json::from_str::<MessageBody>(text) {
                        insert_message_into_db(&messageBody).unwrap_or_else(|error| {
                            eprintln!("Error inserting message into DB. {}", error);
                        });
                        broadcast_message(msg, &users).await;
                    }
                }
            }
            Err(e) => {
                break;
            }
        }
    }
}

async fn broadcast_message(message: Message, users: &Users) {
    if let Ok(text) = message.to_str() {
        let mut disconnected_clients = vec![];
        let mut users_locked = users.lock().unwrap();

        for (index, user) in users_locked.iter().enumerate() {
            if let Err(_) = user.send(Ok(Message::text(text))) {
                disconnected_clients.push(index);
            }
        }

        for &index in disconnected_clients.iter().rev() {
            users_locked.remove(index);
        }
    }
}