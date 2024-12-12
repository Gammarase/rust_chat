use actix_web::{web, HttpResponse, Error};
use serde::Serialize;
use crate::datasource::Datasource;

#[derive(Serialize)]
struct Message {
    chatname: String,
    content: String,
    sended_at: String,
}

pub async fn get_messages(
    chatname: web::Path<String>,
) -> Result<HttpResponse, Error> {
    match Datasource::new() {
        Ok(datasource) => {
            match datasource.get_messages_by_chatname(&chatname) {
                Ok(messages) => {
                    let response: Vec<Message> = messages
                        .into_iter()
                        .map(|( chatname, content, sended_at)| Message {
                            chatname,
                            content,
                            sended_at,
                        })
                        .collect();
                    Ok(HttpResponse::Ok().json(response))
                }
                Err(_) => Ok(HttpResponse::InternalServerError().body("Failed to fetch messages")),
            }
        }
        Err(_) => Ok(HttpResponse::InternalServerError().body("Failed to connect to the database")),
    }
}
