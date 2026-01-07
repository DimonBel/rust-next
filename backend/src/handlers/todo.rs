use actix_web::{web, HttpResponse, Responder, Result};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::models::{CreateTodo, Todo, UpdateTodo};

pub type TodoDb = Arc<Mutex<Vec<Todo>>>;

pub async fn create_todo(db: web::Data<TodoDb>, item: web::Json<CreateTodo>) -> impl Responder {
    let mut todos = db.lock().unwrap();
    let todo = Todo {
        id: Uuid::new_v4(),
        title: item.title.clone(),
        completed: item.completed.unwrap_or(false),
    };
    todos.push(todo.clone());
    HttpResponse::Created().json(todo)
}

pub async fn get_todos(db: web::Data<TodoDb>) -> impl Responder {
    let todos = db.lock().unwrap();
    HttpResponse::Ok().json(&*todos)
}

pub async fn get_todo(db: web::Data<TodoDb>, path: web::Path<Uuid>) -> Result<impl Responder> {
    let todos = db.lock().unwrap();
    if let Some(todo) = todos.iter().find(|t| t.id == *path) {
        Ok(HttpResponse::Ok().json(todo))
    } else {
        Ok(HttpResponse::NotFound().body("Todo not found"))
    }
}

pub async fn update_todo(
    db: web::Data<TodoDb>,
    path: web::Path<Uuid>,
    item: web::Json<UpdateTodo>,
) -> Result<impl Responder> {
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

pub async fn delete_todo(db: web::Data<TodoDb>, path: web::Path<Uuid>) -> Result<impl Responder> {
    let mut todos = db.lock().unwrap();
    if let Some(pos) = todos.iter().position(|t| t.id == *path) {
        let removed = todos.remove(pos);
        Ok(HttpResponse::Ok().json(removed))
    } else {
        Ok(HttpResponse::NotFound().body("Todo not found"))
    }
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/todos")
            .route("", web::post().to(create_todo))
            .route("", web::get().to(get_todos))
            .route("/{id}", web::get().to(get_todo))
            .route("/{id}", web::put().to(update_todo))
            .route("/{id}", web::delete().to(delete_todo)),
    );
}
