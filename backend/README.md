# Backend API - Rust + Actix Web

Document upload and analysis system with AI-powered text extraction and summarization.

## 🚀 Features

- **Document Upload & Analysis**: Support for PDF, DOCX, PPTX, and TXT files
- **AI-Powered Analysis**: Automatic extraction of summaries, keywords, entities, and topics using Groq AI
- **Text Extraction**:
  - PDF: Smart page sampling for scanned document detection
  - DOCX: Full text extraction from Word documents
  - PPTX: Slide content extraction from PowerPoint presentations
  - TXT: Plain text file support
- **Todo Management**: Simple CRUD API for todo items
- **SQLite Database**: Lightweight database with Diesel ORM
- **CORS Enabled**: Cross-origin requests supported

## 📁 Project Structure

```
backend/
├── src/
│   ├── main.rs                    # Application entry point
│   ├── schema.rs                  # Diesel database schema
│   ├── models/                    # Data models
│   │   ├── mod.rs
│   │   ├── document.rs           # Document model
│   │   └── todo.rs               # Todo model
│   ├── handlers/                  # HTTP request handlers
│   │   ├── mod.rs
│   │   ├── document.rs           # Document endpoints
│   │   └── todo.rs               # Todo endpoints
│   ├── services/                  # Business logic
│   │   ├── mod.rs
│   │   ├── text_extraction.rs    # File text extraction
│   │   └── ai_analysis.rs        # Groq AI integration
│   └── db/                        # Database configuration
│       └── mod.rs                # Connection pool setup
├── migrations/                    # Diesel migrations
├── uploads/                       # Uploaded files storage
├── Cargo.toml                    # Rust dependencies
├── Dockerfile                    # Docker configuration
└── .env                          # Environment variables
```

## 🛠️ Tech Stack

- **Framework**: Actix Web 4.x
- **Database**: SQLite with Diesel ORM 2.x
- **AI Service**: Groq API (llama-3.1-8b-instant)
- **Document Processing**:
  - `lopdf` - PDF text extraction
  - `docx-rs` - DOCX processing
  - `zip` - PPTX extraction
- **Async Runtime**: Tokio

## ⚙️ Environment Variables

Create a `.env` file in the backend directory:

```env
DATABASE_URL=sqlite://db.sqlite
GROQ_API_KEY=your_groq_api_key_here
```

## 🚦 Getting Started

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- SQLite 3

### Installation

1. **Clone the repository**

   ```bash
   cd backend
   ```

2. **Install dependencies**

   ```bash
   cargo build
   ```

3. **Set up environment variables**

   ```bash
   cp .env.example .env
   # Edit .env and add your GROQ_API_KEY
   ```

4. **Run database migrations**

   ```bash
   diesel migration run
   ```

5. **Start the server**
   ```bash
   cargo run
   ```

The server will start on `http://0.0.0.0:8080`

## 📡 API Endpoints

### Documents

#### Upload Document

```http
POST /documents/upload
Content-Type: multipart/form-data

# Response: Document object with AI analysis
```

#### List Documents

```http
GET /documents/list

# Response: Array of documents
```

#### Get Document Details

```http
GET /documents/{filename}

# Response: Document object
```

#### Delete Document

```http
DELETE /documents/{filename}

# Response: Success message
```

### Todos

#### Create Todo

```http
POST /todos
Content-Type: application/json

{
  "title": "Task name",
  "completed": false
}
```

#### Get All Todos

```http
GET /todos
```

#### Get Todo by ID

```http
GET /todos/{id}
```

#### Update Todo

```http
PUT /todos/{id}
Content-Type: application/json

{
  "title": "Updated task",
  "completed": true
}
```

#### Delete Todo

```http
DELETE /todos/{id}
```

## 🗄️ Database Schema

### Documents Table

```sql
CREATE TABLE documents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  filename TEXT NOT NULL UNIQUE,
  path TEXT NOT NULL,
  summary TEXT,
  keywords TEXT,
  entities TEXT,
  topics TEXT,
  uploaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## 🧪 Development

### Build

```bash
cargo build
```

### Run in development mode

```bash
cargo run
```

### Run with release optimization

```bash
cargo run --release
```

### Format code

```bash
cargo fmt
```

### Run linter

```bash
cargo clippy
```

## 🐳 Docker

Build and run with Docker:

```bash
docker build -t backend-api .
docker run -p 8080:8080 --env-file .env backend-api
```

## 📝 Document Processing Flow

1. **File Upload**: Multipart form data received
2. **Save to Disk**: File stored in `uploads/` directory
3. **Text Extraction**:
   - Runs in isolated thread (`tokio::spawn_blocking`)
   - Handles panics from PDF extraction libraries
   - Smart sampling for PDF documents
4. **AI Analysis**:
   - Text sent to Groq API (max 12,000 chars)
   - Extracts: Summary, Keywords, Entities, Topics
5. **Database Storage**: Document metadata and analysis saved
6. **Response**: Complete document object returned

## 🔒 Security Notes

- **API Keys**: Never commit `.env` file to version control
- **File Uploads**: Files stored in `uploads/` directory (gitignored)
- **CORS**: Currently allows all origins (configure for production)

## 🐛 Known Issues & Limitations

- **Scanned PDFs**: Cannot extract text from image-based PDFs without OCR
- **Large Files**: No file size limits currently enforced
- **Concurrent Uploads**: Multiple simultaneous uploads to same filename may conflict

## 📄 License

This project is part of a demo application.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request
