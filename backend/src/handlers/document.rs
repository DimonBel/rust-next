use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse, Responder};
use diesel::prelude::*;
use futures_util::stream::StreamExt;
use std::fs;
use std::path::Path;

use crate::db::DbPool;
use crate::models::{Document, NewDocument};
use crate::schema::documents;
use crate::services::{analyze_document_with_ai, extract_text_from_file};

pub async fn upload_and_analyze(
    pool: web::Data<DbPool>,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    fs::create_dir_all("uploads").ok();

    while let Some(item) = payload.next().await {
        let mut field = item?;
        let filename = field
            .content_disposition()
            .get_filename()
            .unwrap_or("file")
            .to_string();
        let filepath = format!("uploads/{}", filename);
        let mut data = Vec::new();

        while let Some(chunk) = field.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
        }

        fs::write(&filepath, &data).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Извлекаем текст в отдельном потоке с изоляцией паники
        let filepath_clone = filepath.clone();
        let filename_clone = filename.clone();
        let text = match tokio::task::spawn_blocking(move || {
            extract_text_from_file(&filepath_clone)
        })
        .await
        {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                eprintln!("Text extraction failed for {}: {}", filename_clone, e);
                format!("Error extracting text: {}", e)
            }
            Err(e) => {
                eprintln!("Task panicked for {}: {}", filename_clone, e);
                "PDF extraction failed - file may use unsupported features. Please try a different file.".to_string()
            }
        };

        eprintln!(
            "Extracted text length for {}: {} chars",
            filename,
            text.len()
        );
        eprintln!(
            "First 200 chars: {}",
            text.chars().take(200).collect::<String>()
        );

        // Анализируем документ, только если текст был успешно извлечён
        let (summary, keywords, entities, topics) =
            if text.starts_with("Error") || text.starts_with("PDF extraction failed") {
                (text.clone(), String::new(), String::new(), String::new())
            } else if text.trim().is_empty() {
                eprintln!("Empty text extracted from {}", filename);
                (
                    "No text content found in document".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            } else {
                eprintln!("Sending to AI for analysis: {}", filename);
                analyze_document_with_ai(&text).await.unwrap_or_else(|e| {
                    eprintln!("Analysis failed for {}: {}", filename, e);
                    (
                        format!("Analysis error: {}", e),
                        String::new(),
                        String::new(),
                        String::new(),
                    )
                })
            };

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

        let doc: Document = documents::table
            .filter(documents::filename.eq(&filename))
            .first(conn)
            .unwrap();

        return Ok(HttpResponse::Ok().json(doc));
    }
    Ok(HttpResponse::BadRequest().body("No file uploaded"))
}

pub async fn get_file_info(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let filename = path.into_inner();
    let conn = &mut pool.get().unwrap();
    match documents::table
        .filter(documents::filename.eq(&filename))
        .first::<Document>(conn)
    {
        Ok(doc) => Ok(HttpResponse::Ok().json(doc)),
        Err(_) => Ok(HttpResponse::NotFound().body("File not found")),
    }
}

pub async fn list_files(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let conn = &mut pool.get().unwrap();
    let docs = documents::table.load::<Document>(conn).unwrap_or_default();
    Ok(HttpResponse::Ok().json(docs))
}

pub async fn delete_document(pool: web::Data<DbPool>, path: web::Path<String>) -> impl Responder {
    let filename = path.into_inner();
    let file_path = format!("uploads/{}", filename);

    if Path::new(&file_path).exists() {
        fs::remove_file(&file_path).ok();
    }

    let conn = &mut pool.get().unwrap();
    diesel::delete(documents::table.filter(documents::filename.eq(&filename)))
        .execute(conn)
        .ok();

    HttpResponse::Ok().body("Document deleted")
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/documents")
            .route("/upload", web::post().to(upload_and_analyze))
            .route("/list", web::get().to(list_files))
            .route("/{filename}", web::get().to(get_file_info))
            .route("/{filename}", web::delete().to(delete_document)),
    );
}
