use docx_rs::{DocumentChild, ParagraphChild, RunChild};
use std::io::Read;
use zip::ZipArchive;

pub fn extract_text_from_file(filepath: &str) -> Result<String, String> {
    if filepath.ends_with(".pdf") {
        extract_pdf_text(filepath)
    } else if filepath.ends_with(".pptx") {
        extract_pptx_text(filepath)
    } else if filepath.ends_with(".docx") {
        extract_docx_text(filepath)
    } else if filepath.ends_with(".txt") {
        extract_txt_text(filepath)
    } else {
        Err("Unsupported file type".to_string())
    }
}

fn extract_pdf_text(filepath: &str) -> Result<String, String> {
    use lopdf::Document;

    let doc = Document::load(filepath).map_err(|e| format!("Failed to load PDF: {}", e))?;

    let pages = doc.get_pages();
    let mut page_numbers: Vec<u32> = pages.keys().copied().collect();
    page_numbers.sort();

    let page_count = page_numbers.len();
    eprintln!("Total pages in PDF: {}", page_count);

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
    let file = std::fs::File::open(filepath).map_err(|e| format!("Failed to open PPTX: {}", e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read PPTX archive: {}", e))?;

    let mut text = String::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read PPTX entry: {}", e))?;

        let name = file.name().to_string();

        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| format!("Failed to read slide XML: {}", e))?;

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

fn extract_docx_text(filepath: &str) -> Result<String, String> {
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
}

fn extract_txt_text(filepath: &str) -> Result<String, String> {
    std::fs::read_to_string(filepath).map_err(|e| format!("TXT read error: {}", e))
}
