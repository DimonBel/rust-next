use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Error, Responder};
use futures_util::stream::StreamExt;
use std::fs;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use serde::{Serialize, Deserialize};
use crate::schema::documents;
use chrono::NaiveDateTime;
use reqwest;
use docx_rs::{DocumentChild, ParagraphChild, RunChild};
use std::path::Path;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[derive(Queryable, Serialize, Deserialize, Clone)]
pub struct Document {
    pub id: Option<i32>,
    pub filename: String,
    pub path: String,
    pub summary: Option<String>,
    pub keywords: Option<String>,
    pub entities: Option<String>,
    pub topics: Option<String>,
    pub uploaded_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = documents)]
pub struct NewDocument<'a> {
    pub filename: &'a str,
    pub path: &'a str,
    pub summary: Option<&'a str>,
    pub keywords: Option<&'a str>,
    pub entities: Option<&'a str>,
    pub topics: Option<&'a str>,
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.app_data(web::Data::new(create_pool()));
    cfg.service(web::resource("/documents/upload").route(web::post().to(upload_and_analyze)));
    cfg.service(web::resource("/documents/list").route(web::get().to(list_files)));
    cfg.service(
        web::resource("/documents/{filename}")
            .route(web::get().to(get_file_info))
            .route(web::delete().to(delete_document))
    );
}

fn create_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"));
    r2d2::Pool::builder().build(manager).expect("Failed to create pool")
}

fn extract_text_from_file(filepath: &str) -> Result<String, String> {
    if filepath.ends_with(".pdf") {
        match pdf_extract::extract_text(filepath) {
            Ok(text) => Ok(text),
            Err(e) => Err(format!("PDF extract error: {}", e)),
        }
    } else if filepath.ends_with(".docx") {
        match std::fs::read(filepath) {
            Ok(bytes) => {
                match docx_rs::read_docx(&bytes) {
                    Ok(docx) => {
                        let mut text = String::new();
                        for child in docx.document.children.iter() {
                            if let DocumentChild::Paragraph(p) = child {
                                for run in &p.children {
                                    if let ParagraphChild::Run(r) = run {
                                        for run_child in &r.children {
                                            if let RunChild::Text(t) = run_child {
                                                text.push_str(&t.text);
                                                text.push(' ');
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(text)
                    },
                    Err(e) => Err(format!("DOCX extract error: {}", e)),
                }
            },
            Err(e) => Err(format!("DOCX read error: {}", e)),
        }
    } else if filepath.ends_with(".txt") {
        match std::fs::read_to_string(filepath) {
            Ok(text) => Ok(text),
            Err(e) => Err(format!("TXT read error: {}", e)),
        }
    } else {
        Err("Unsupported file type".to_string())
    }
}

async fn analyze_document_groq(text: &str) -> Result<(String, String, String, String), String> {
    let api_key = "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";
    let client = reqwest::Client::new();
    let prompt = format!(
        "Extract the following from the document:\n\
        Summary (3-5 sentences):\n\
        Keywords (comma separated):\n\
        Entities (comma separated):\n\
        Topics (comma separated):\n\
        Document:\n{}",
        &text.chars().take(4000).collect::<String>()
    );
    let body = serde_json::json!({
        "model": "llama3-8b-8192",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant that extracts structured information from documents. Always answer in the following format: Summary: ... Keywords: ... Entities: ... Topics: ..."},
            {"role": "user", "content": prompt}
        ]
    });
    let resp = client.post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send().await.map_err(|e| format!("Groq API error: {}", e))?;
    let json: serde_json::Value = resp.json().await.map_err(|e| format!("Groq JSON error: {}", e))?;
    let content = json["choices"][0]["message"]["content"].as_str().unwrap_or("");
    // Простейший парсер (можно улучшить)
    let summary = content.split("Keywords:").next().unwrap_or("").replace("Summary:", "").trim().to_string();
    let keywords = content.split("Keywords:").nth(1).and_then(|s| s.split("Entities:").next()).unwrap_or("").trim().to_string();
    let entities = content.split("Entities:").nth(1).and_then(|s| s.split("Topics:").next()).unwrap_or("").trim().to_string();
    let topics = content.split("Topics:").nth(1).unwrap_or("").trim().to_string();
    Ok((summary, keywords, entities, topics))
}

async fn upload_and_analyze(
    pool: web::Data<DbPool>,
    mut payload: Multipart
) -> Result<HttpResponse, Error> {
    fs::create_dir_all("uploads").ok();
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let filename = field.content_disposition().get_filename().unwrap_or("file").to_string();
        let filepath = format!("uploads/{}", filename);
        let mut data = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
        }
        fs::write(&filepath, &data).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        let text = extract_text_from_file(&filepath).unwrap_or_else(|e| e);
        let (summary, keywords, entities, topics) = analyze_document_groq(&text).await.unwrap_or_else(|e| (e.clone(), String::new(), String::new(), String::new()));
        let new_doc = NewDocument {
            filename: &filename,
            path: &filepath,
            summary: Some(&summary),
            keywords: Some(&keywords),
            entities: Some(&entities),
            topics: Some(&topics),
        };
        let conn = &mut pool.get().unwrap();
        diesel::insert_into(documents::table)
            .values(&new_doc)
            .on_conflict(documents::filename)
            .do_update()
            .set((
                documents::path.eq(&filepath),
                documents::summary.eq(Some(&summary)),
                documents::keywords.eq(Some(&keywords)),
                documents::entities.eq(Some(&entities)),
                documents::topics.eq(Some(&topics)),
            ))
            .execute(conn)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        let doc: Document = documents::table.filter(documents::filename.eq(&filename)).first(conn).unwrap();
        return Ok(HttpResponse::Ok().json(doc));
    }
    Ok(HttpResponse::BadRequest().body("No file uploaded"))
}

async fn get_file_info(
    pool: web::Data<DbPool>,
    path: web::Path<String>
) -> Result<HttpResponse, Error> {
    let filename = path.into_inner();
    let conn = &mut pool.get().unwrap();
    match documents::table.filter(documents::filename.eq(&filename)).first::<Document>(conn) {
        Ok(doc) => Ok(HttpResponse::Ok().json(doc)),
        Err(_) => Ok(HttpResponse::NotFound().body("File not found")),
    }
}

async fn list_files(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let conn = &mut pool.get().unwrap();
    let docs = documents::table.load::<Document>(conn).unwrap_or_default();
    Ok(HttpResponse::Ok().json(docs))
}

pub async fn delete_document(
    pool: web::Data<DbPool>,
    path: web::Path<String>
) -> impl Responder {
    let filename = path.into_inner();
    let file_path = format!("uploads/{}", filename);
    if Path::new(&file_path).exists() {
        match fs::remove_file(&file_path) {
            Ok(_) => {
                // Also remove from DB
                let conn = &mut pool.get().unwrap();
                diesel::delete(documents::table.filter(documents::filename.eq(&filename)))
                    .execute(conn)
                    .ok();
                HttpResponse::Ok().body("File deleted")
            },
            Err(_) => HttpResponse::InternalServerError().body("Failed to delete file"),
        }
    } else {
        HttpResponse::NotFound().body("File not found")
    }
} 