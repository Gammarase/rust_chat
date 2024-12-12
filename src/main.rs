use std::io;

use actix_files::NamedFile;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use tokio::{
    task::{spawn, spawn_local},
    try_join,
};

mod handler;
mod server;
mod datasource;
mod api;

pub use self::server::{ChatServer, ChatServerHandle};

async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}


async fn chat_ws(
    req: HttpRequest,
    stream: web::Payload,
    chat_server: web::Data<ChatServerHandle>,
) -> Result<HttpResponse, Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

   
    spawn_local(handler::chat_ws(
        (**chat_server).clone(),
        session,
        msg_stream,
    ));

    Ok(res)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://localhost:8080");

    let (chat_server, server_tx) = ChatServer::new();

    let chat_server = spawn(chat_server.run());

    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server_tx.clone()))
            .service(web::resource("/").to(index))
            .service(web::resource("/ws").route(web::get().to(chat_ws)))
            .route("/messages/{chatname}", web::get().to(api::get_messages))
            .wrap(middleware::Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run();

    try_join!(http_server, async move { chat_server.await.unwrap() })?;

    Ok(())
}
