"use client";
import React, { useEffect, useState, useRef } from "react";
import {
  AppBar,
  Toolbar,
  Typography,
  Box,
  Container,
  Button,
  Card,
  CardContent,
  CardActions,
  CardHeader,
  Avatar,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Snackbar,
  Alert,
  Grid,
  CircularProgress,
  Paper,
  Stack,
  Fade,
  Tooltip,
  Chip,
  Divider,
  LinearProgress,
} from "@mui/material";
import UploadFileIcon from "@mui/icons-material/UploadFile";
import DescriptionIcon from "@mui/icons-material/Description";
import InfoIcon from "@mui/icons-material/Info";
import CloseIcon from "@mui/icons-material/Close";
import DownloadIcon from "@mui/icons-material/Download";
import DeleteIcon from "@mui/icons-material/Delete";

const API_BASE = "http://localhost:8080";

// Тип документа, как на бэкенде
interface Document {
  id?: number;
  filename: string;
  path: string;
  summary?: string;
  keywords?: string;
  entities?: string;
  topics?: string;
  uploaded_at?: string;
}

function formatDate(dateStr?: string) {
  if (!dateStr) return "-";
  const d = new Date(dateStr);
  return d.toLocaleString();
}

export default function DocumentDashboard() {
  const [documents, setDocuments] = useState<Document[]>([]);
  const [loading, setLoading] = useState(true);
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [snackbar, setSnackbar] = useState<{ open: boolean; message: string; severity: "success" | "error" }>({ open: false, message: "", severity: "success" });
  const [selectedDoc, setSelectedDoc] = useState<Document | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const fetchDocuments = async () => {
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE}/documents/list`);
      const data = await res.json();
      setDocuments(data);
    } catch {
      setSnackbar({ open: true, message: "Не удалось загрузить список документов", severity: "error" });
    }
    setLoading(false);
  };

  useEffect(() => {
    fetchDocuments();
  }, []);

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement> | React.DragEvent<HTMLDivElement>) => {
    let files: FileList | null = null;
    if ("dataTransfer" in e) {
      files = e.dataTransfer.files;
    } else {
      files = e.target.files;
    }
    if (!files || files.length === 0) return;
    setUploading(true);
    setProgress(0);
    const formData = new FormData();
    formData.append("file", files[0]);
    try {
      const xhr = new XMLHttpRequest();
      xhr.open("POST", `${API_BASE}/documents/upload`);
      xhr.upload.onprogress = (event) => {
        if (event.lengthComputable) {
          setProgress(Math.round((event.loaded / event.total) * 100));
        }
      };
      xhr.onload = () => {
        setUploading(false);
        setProgress(0);
        if (xhr.status === 200) {
          setSnackbar({ open: true, message: "Документ успешно загружен!", severity: "success" });
          fetchDocuments();
        } else {
          setSnackbar({ open: true, message: "Ошибка загрузки документа", severity: "error" });
        }
      };
      xhr.onerror = () => {
        setUploading(false);
        setSnackbar({ open: true, message: "Ошибка загрузки документа", severity: "error" });
      };
      xhr.send(formData);
    } catch {
      setUploading(false);
      setSnackbar({ open: true, message: "Ошибка загрузки документа", severity: "error" });
    }
  };

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    handleFileChange(e);
  };

  const handleDocClick = async (doc: Document) => {
    setSelectedDoc(null);
    setDetailsOpen(true);
    try {
      const res = await fetch(`${API_BASE}/documents/${encodeURIComponent(doc.filename)}`);
      if (!res.ok) throw new Error();
      const data = await res.json();
      setSelectedDoc(data);
    } catch {
      setSnackbar({ open: true, message: "Не удалось получить детали документа", severity: "error" });
      setDetailsOpen(false);
    }
  };

  const handleDelete = async (filename: string) => {
    try {
      const res = await fetch(`${API_BASE}/documents/${encodeURIComponent(filename)}`, { method: "DELETE" });
      if (!res.ok) throw new Error();
      setSnackbar({ open: true, message: "Документ удалён!", severity: "success" });
      fetchDocuments();
    } catch {
      setSnackbar({ open: true, message: "Ошибка удаления документа", severity: "error" });
    }
  };

  return (
    <Box sx={{ minHeight: "100vh", bgcolor: "#f7fafd" }}>
      <AppBar position="static" color="default" elevation={1} sx={{ bgcolor: "#fff" }}>
        <Toolbar>
          <DescriptionIcon sx={{ mr: 1, color: "primary.main" }} />
          <Typography variant="h5" fontWeight={700} color="primary" sx={{ flexGrow: 1 }}>
            InsightDocs
          </Typography>
          <Button color="primary" variant="outlined" onClick={() => fileInputRef.current?.click()} startIcon={<UploadFileIcon />}>
            Загрузить документ
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            style={{ display: "none" }}
            onChange={handleFileChange}
            accept=".pdf,.docx,.txt"
          />
        </Toolbar>
      </AppBar>
      <Container maxWidth="lg" sx={{ mt: 6 }}>
        <Paper
          elevation={0}
          sx={{
            p: 4,
            mb: 4,
            border: "2px dashed #90caf9",
            borderRadius: 4,
            textAlign: "center",
            bgcolor: uploading ? "#e3f2fd" : "#f7fafd",
            transition: "background 0.2s",
            cursor: uploading ? "progress" : "pointer",
            outline: "none",
            ':hover': { borderColor: "primary.main", bgcolor: "#e3f2fd" },
          }}
          onDrop={handleDrop}
          onDragOver={e => e.preventDefault()}
        >
          <UploadFileIcon sx={{ fontSize: 48, color: "primary.main", mb: 1 }} />
          <Typography variant="h6" gutterBottom>
            Перетащите файл сюда или нажмите "Загрузить документ"
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Поддерживаются PDF, DOCX, TXT
          </Typography>
          {uploading && <LinearProgress variant="determinate" value={progress} sx={{ mt: 2 }} />}
        </Paper>
        <Fade in timeout={700}>
          <Box>
            <Typography variant="h5" fontWeight={600} mb={3}>
              Ваши документы
            </Typography>
            {loading ? (
              <Box display="flex" justifyContent="center" py={4}>
                <CircularProgress />
              </Box>
            ) : (
              <Grid container spacing={3}>
                {documents.length === 0 && (
                  <Box width="100%">
                    <Typography align="center" color="text.secondary" py={2}>
                      Нет загруженных документов
                    </Typography>
                  </Box>
                )}
                {documents.map(doc => (
                  <Box key={doc.filename} sx={{ width: '100%', mb: 3, '@media (min-width:600px)': { width: '50%' }, '@media (min-width:900px)': { width: '33.33%' }, display: 'inline-block', verticalAlign: 'top' }}>
                    <Card elevation={2} sx={{ borderRadius: 3, cursor: "pointer", transition: "box-shadow 0.2s", ':hover': { boxShadow: 6 } }} onClick={() => handleDocClick(doc)}>
                      <CardHeader
                        avatar={<Avatar sx={{ bgcolor: "primary.light" }}><DescriptionIcon /></Avatar>}
                        title={doc.filename}
                        subheader={formatDate(doc.uploaded_at)}
                        sx={{ pb: 0 }}
                      />
                      <CardContent>
                        <Stack direction="row" spacing={1} flexWrap="wrap">
                          {doc.summary && <Chip label="Summary" color="info" size="small" />}
                          {doc.keywords && <Chip label="Keywords" color="success" size="small" />}
                          {doc.entities && <Chip label="Entities" color="warning" size="small" />}
                          {doc.topics && <Chip label="Topics" color="secondary" size="small" />}
                        </Stack>
                        <Divider sx={{ my: 1 }} />
                        <Typography variant="body2" color="text.secondary" noWrap>
                          {doc.summary || "Документ без анализа"}
                        </Typography>
                      </CardContent>
                      <CardActions>
                        <Tooltip title="Подробнее">
                          <IconButton color="primary" onClick={e => { e.stopPropagation(); handleDocClick(doc); }}>
                            <InfoIcon />
                          </IconButton>
                        </Tooltip>
                        <Tooltip title="Скачать">
                          <IconButton color="primary" component="a" href={`/uploads/${encodeURIComponent(doc.filename)}`} download onClick={e => e.stopPropagation()}>
                            <DownloadIcon />
                          </IconButton>
                        </Tooltip>
                        <Tooltip title="Удалить">
                          <IconButton color="error" onClick={e => { e.stopPropagation(); handleDelete(doc.filename); }}>
                            <DeleteIcon />
                          </IconButton>
                        </Tooltip>
                      </CardActions>
                    </Card>
                  </Box>
                ))}
              </Grid>
            )}
          </Box>
        </Fade>
      </Container>
      <Dialog open={detailsOpen} onClose={() => setDetailsOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>
          Детали документа
          <IconButton aria-label="close" onClick={() => setDetailsOpen(false)} sx={{ position: "absolute", right: 8, top: 8 }}>
            <CloseIcon />
          </IconButton>
        </DialogTitle>
        <DialogContent dividers>
          {selectedDoc ? (
            <Box>
              <Typography variant="h6" gutterBottom>{selectedDoc.filename}</Typography>
              <Typography variant="body2" color="text.secondary" gutterBottom>
                Загружен: {formatDate(selectedDoc.uploaded_at)}
              </Typography>
              <Divider sx={{ my: 2 }} />
              <Typography variant="subtitle1" fontWeight={600}>Summary</Typography>
              <Typography variant="body2" gutterBottom>{selectedDoc.summary || "-"}</Typography>
              <Typography variant="subtitle1" fontWeight={600}>Keywords</Typography>
              <Typography variant="body2" gutterBottom>{selectedDoc.keywords || "-"}</Typography>
              <Typography variant="subtitle1" fontWeight={600}>Entities</Typography>
              <Typography variant="body2" gutterBottom>{selectedDoc.entities || "-"}</Typography>
              <Typography variant="subtitle1" fontWeight={600}>Topics</Typography>
              <Typography variant="body2" gutterBottom>{selectedDoc.topics || "-"}</Typography>
            </Box>
          ) : (
            <Box display="flex" justifyContent="center" alignItems="center" minHeight={200}>
              <CircularProgress />
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDetailsOpen(false)} color="primary">Закрыть</Button>
        </DialogActions>
      </Dialog>
      <Snackbar
        open={snackbar.open}
        autoHideDuration={3000}
        onClose={() => setSnackbar({ ...snackbar, open: false })}
      >
        <Alert severity={snackbar.severity} sx={{ width: '100%' }}>
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Box>
  );
}
