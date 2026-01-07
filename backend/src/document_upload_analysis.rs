use crate::schema::documents;
use actix_multipart::Multipart;
use actix_web::{web, Error, HttpResponse, Responder};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use docx_rs::{DocumentChild, ParagraphChild, RunChild};
use futures_util::stream::StreamExt;
use reqwest;
use serde::{Deserialize, Serialize};
use std::fs;
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
            .route(web::delete().to(delete_document)),
    );
}

fn create_pool() -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
    );
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool")
}

fn extract_pdf_text(filepath: &str) -> Result<String, String> {
    use lopdf::Document;

    let doc = Document::load(filepath).map_err(|e| format!("Failed to load PDF: {}", e))?;

    let pages = doc.get_pages();
    let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
    page_numbers.sort();

    let page_count = page_numbers.len();
    eprintln!("Total pages in PDF: {}", page_count);

    // Пробуем извлечь текст из разных частей книги
    let mut samples = Vec::new();

    // Диапазон 1: страницы 10-25 (обычно введение/первая глава)
    let range1_start = 10.min(page_count);
    let range1_end = 25.min(page_count);
    samples.push((range1_start, range1_end, "pages 10-25"));

    // Диапазон 2: страницы из середины книги
    if page_count > 50 {
        let mid = page_count / 2;
        samples.push((mid - 10, mid + 10, "middle pages"));
    }

    // Диапазон 3: последняя треть книги
    if page_count > 100 {
        let third = (page_count * 2) / 3;
        samples.push((third, third + 20, "last third pages"));
    }

    let mut best_text = String::new();
    let mut best_length = 0;
    let mut best_range = "";

    // Пробуем каждый диапазон и выбираем лучший
    for (start_idx, end_idx, label) in samples {
        let mut sample_text = String::new();

        for &page_num in page_numbers
            .iter()
            .skip(start_idx)
            .take(end_idx - start_idx)
        {
            if let Ok(page_text) = doc.extract_text(&[page_num]) {
                sample_text.push_str(&page_text);
                sample_text.push('\n');
            }
        }

        let length = sample_text.len();
        eprintln!("Extracted from {}: {} chars", label, length);

        if length > best_length {
            best_length = length;
            best_text = sample_text;
            best_range = label;
        }
    }

    eprintln!("Best text from: {} ({} chars)", best_range, best_length);

    if best_text.trim().is_empty() {
        Err(
            "⚠️ This PDF appears to be scanned images without searchable text.\n\
            The document cannot be analyzed without OCR (Optical Character Recognition).\n\
            \n\
            Suggestions:\n\
            • Upload a PDF with a text layer (created from Word/digital source)\n\
            • Convert the PDF to text using an OCR tool first\n\
            • Upload the original DOCX/PPTX file if available"
                .to_string(),
        )
    } else {
        Ok(best_text)
    }
}

fn extract_pptx_text(filepath: &str) -> Result<String, String> {
    use std::io::Read;
    use zip::ZipArchive;

    let file = std::fs::File::open(filepath).map_err(|e| format!("Failed to open PPTX: {}", e))?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read PPTX archive: {}", e))?;

    let mut text = String::new();

    // PPTX содержит слайды в ppt/slides/slide*.xml
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read PPTX entry: {}", e))?;

        let name = file.name().to_string();

        // Ищем файлы слайдов
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| format!("Failed to read slide XML: {}", e))?;

            // Извлекаем текст из XML (простой подход - ищем теги <a:t>)
            for part in contents.split("<a:t>") {
                if let Some(end_pos) = part.find("</a:t>") {
                    text.push_str(&part[..end_pos]);
                    text.push(' ');
                }
            }
        }
    }

    if text.trim().is_empty() {
        Err("No text found in PPTX".to_string())
    } else {
        Ok(text)
    }
}

fn extract_text_from_file(filepath: &str) -> Result<String, String> {
    if filepath.ends_with(".pdf") {
        extract_pdf_text(filepath)
    } else if filepath.ends_with(".pptx") {
        extract_pptx_text(filepath)
    } else if filepath.ends_with(".docx") {
        match std::fs::read(filepath) {
            Ok(bytes) => match docx_rs::read_docx(&bytes) {
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
                }
                Err(e) => Err(format!("DOCX extract error: {}", e)),
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
    let api_key = std::env::var("GROQ_API_KEY")
        .map_err(|_| "GROQ_API_KEY environment variable not set".to_string())?;
    let client = reqwest::Client::new();
    let prompt = format!(
        "Extract the following from the document:\n\
        Summary (3-5 sentences):\n\
        Keywords (comma separated):\n\
        Entities (comma separated):\n\
        Topics (comma separated):\n\
        Document:\n{}",
        &text.chars().take(12000).collect::<String>()
    );
    let body = serde_json::json!({
        "model": "llama-3.1-8b-instant",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant that extracts structured information from documents. Always answer in the following format: Summary: ... Keywords: ... Entities: ... Topics: ..."},
            {"role": "user", "content": prompt}
        ]
    });
    let resp = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Groq API error: {}", e))?;
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Groq JSON error: {}", e))?;

    // Логируем ответ для диагностики
    eprintln!("Groq response: {:?}", json);

    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    eprintln!("Groq content: {}", content);

    // Простейший парсер (можно улучшить)
    let summary = content
        .split("Keywords:")
        .next()
        .unwrap_or("")
        .replace("Summary:", "")
        .trim()
        .to_string();
    let keywords = content
        .split("Keywords:")
        .nth(1)
        .and_then(|s| s.split("Entities:").next())
        .unwrap_or("")
        .trim()
        .to_string();
    let entities = content
        .split("Entities:")
        .nth(1)
        .and_then(|s| s.split("Topics:").next())
        .unwrap_or("")
        .trim()
        .to_string();
    let topics = content
        .split("Topics:")
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();
    Ok((summary, keywords, entities, topics))
}

async fn upload_and_analyze(
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

        // Логируем извлечённый текст для диагностики
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
                // Если извлечение не удалось, не анализируем
                (text.clone(), String::new(), String::new(), String::new())
            } else if text.trim().is_empty() {
                // Если текст пустой
                eprintln!("Empty text extracted from {}", filename);
                (
                    "No text content found in document".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            } else {
                // Анализируем успешно извлечённый текст
                eprintln!("Sending to Groq for analysis: {}", filename);
                analyze_document_groq(&text).await.unwrap_or_else(|e| {
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

async fn get_file_info(
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

async fn list_files(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let conn = &mut pool.get().unwrap();
    let docs = documents::table.load::<Document>(conn).unwrap_or_default();
    Ok(HttpResponse::Ok().json(docs))
}

pub async fn delete_document(pool: web::Data<DbPool>, path: web::Path<String>) -> impl Responder {
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
            }
            Err(_) => HttpResponse::InternalServerError().body("Failed to delete file"),
        }
    } else {
        HttpResponse::NotFound().body("File not found")
    }
}
