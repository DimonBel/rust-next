use actix_web::{web, App, HttpServer, HttpResponse, Responder, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::{Mutex, Arc};
use actix_cors::Cors;
use actix_files::Files;
mod document_upload_analysis;
mod schema;

#[derive(Serialize, Deserialize, Clone)]
struct Todo {
    id: Uuid,
    title: String,
    completed: bool,
}

#[derive(Deserialize)]
struct CreateTodo {
    title: String,
    completed: Option<bool>,
}

#[derive(Deserialize)]
struct UpdateTodo {
    title: Option<String>,
    completed: Option<bool>,
}

type Db = Arc<Mutex<Vec<Todo>>>;

async fn create_todo(db: web::Data<Db>, item: web::Json<CreateTodo>) -> impl Responder {
    let mut todos = db.lock().unwrap();
    let todo = Todo {
        id: Uuid::new_v4(),
        title: item.title.clone(),
        completed: item.completed.unwrap_or(false),
    };
    todos.push(todo.clone());
    HttpResponse::Created().json(todo)
}

async fn get_todos(db: web::Data<Db>) -> impl Responder {
    let todos = db.lock().unwrap();
    HttpResponse::Ok().json(&*todos)
}

async fn get_todo(db: web::Data<Db>, path: web::Path<Uuid>) -> Result<impl Responder> {
    let todos = db.lock().unwrap();
    if let Some(todo) = todos.iter().find(|t| t.id == *path) {
        Ok(HttpResponse::Ok().json(todo))
    } else {
        Ok(HttpResponse::NotFound().body("Todo not found"))
    }
}

async fn update_todo(db: web::Data<Db>, path: web::Path<Uuid>, item: web::Json<UpdateTodo>) -> Result<impl Responder> {
    let mut todos = db.lock().unwrap();
    if let Some(todo) = todos.iter_mut().find(|t| t.id == *path) {
        if let Some(title) = &item.title {
            todo.title = title.clone();
        }
        if let Some(completed) = item.completed {
            todo.completed = completed;
        }
        Ok(HttpResponse::Ok().json(todo.clone()))
    } else {
        Ok(HttpResponse::NotFound().body("Todo not found"))
    }
}

async fn delete_todo(db: web::Data<Db>, path: web::Path<Uuid>) -> Result<impl Responder> {
    let mut todos = db.lock().unwrap();
    if let Some(pos) = todos.iter().position(|t| t.id == *path) {
        let removed = todos.remove(pos);
        Ok(HttpResponse::Ok().json(removed))
    } else {
        Ok(HttpResponse::NotFound().body("Todo not found"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let db: Db = Arc::new(Mutex::new(Vec::new()));
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
            )
            .app_data(web::Data::new(db.clone()))
            .route("/todos", web::post().to(create_todo))
            .route("/todos", web::get().to(get_todos))
            .route("/todos/{id}", web::get().to(get_todo))
            .route("/todos/{id}", web::put().to(update_todo))
            .route("/todos/{id}", web::delete().to(delete_todo))
            .configure(document_upload_analysis::init_routes)
            .service(Files::new("/uploads", "uploads").show_files_listing())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
} 