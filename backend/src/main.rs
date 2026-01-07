use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer};
use std::sync::{Arc, Mutex};

mod db;
mod handlers;
mod models;
mod schema;
mod services;

use handlers::todo::{configure_routes as configure_todo_routes, TodoDb};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    // Todo database (in-memory)
    let todo_db: TodoDb = Arc::new(Mutex::new(Vec::new()));

    // Document database pool
    let doc_pool = db::create_pool();

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .app_data(web::Data::new(todo_db.clone()))
            .app_data(web::Data::new(doc_pool.clone()))
            .configure(configure_todo_routes)
            .configure(handlers::document::configure_routes)
            .service(Files::new("/uploads", "uploads").show_files_listing())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
