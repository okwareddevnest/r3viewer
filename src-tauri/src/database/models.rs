use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Student {
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub github_username: Option<String>,
    pub cohort: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: i64,
    pub student_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub github_url: String,
    pub technology_stack: Option<String>, // JSON array as string
    pub status: String, // 'pending', 'analyzing', 'completed', 'failed'
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalysisResult {
    pub id: i64,
    pub project_id: i64,
    pub code_quality_score: Option<i32>,
    pub structure_score: Option<i32>,
    pub documentation_score: Option<i32>,
    pub functionality_score: Option<i32>,
    pub total_score: Option<i32>,
    pub feedback: Option<String>,
    pub analysis_data: Option<String>, // JSON as string
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaygroundSession {
    pub id: i64,
    pub project_id: i64,
    pub container_id: Option<String>,
    pub port: Option<i32>,
    pub status: String, // 'starting', 'running', 'stopped', 'error'
    pub created_at: DateTime<Utc>,
}

// Input DTOs for creating new records
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStudent {
    pub name: String,
    pub email: Option<String>,
    pub github_username: Option<String>,
    pub cohort: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProject {
    pub student_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub github_url: String,
    pub technology_stack: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAnalysisResult {
    pub project_id: i64,
    pub code_quality_score: Option<i32>,
    pub structure_score: Option<i32>,
    pub documentation_score: Option<i32>,
    pub functionality_score: Option<i32>,
    pub total_score: Option<i32>,
    pub feedback: Option<String>,
    pub analysis_data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePlaygroundSession {
    pub project_id: i64,
    pub container_id: Option<String>,
    pub port: Option<i32>,
    pub status: String,
}

// Response DTOs with joined data
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectWithStudent {
    pub id: i64,
    pub student_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub github_url: String,
    pub technology_stack: Option<Vec<String>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub student_name: String,
    pub student_email: Option<String>,
    pub student_github_username: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectWithAnalysis {
    pub project: Project,
    pub student: Student,
    pub analysis: Option<AnalysisResult>,
    pub playground: Option<PlaygroundSession>,
}

// Technology stack enum for type safety
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechnologyStack {
    #[serde(rename = "nodejs")]
    NodeJS,
    #[serde(rename = "python")]
    Python,
    #[serde(rename = "java")]
    Java,
    #[serde(rename = "react")]
    React,
    #[serde(rename = "vue")]
    Vue,
    #[serde(rename = "angular")]
    Angular,
    #[serde(rename = "django")]
    Django,
    #[serde(rename = "flask")]
    Flask,
    #[serde(rename = "spring-boot")]
    SpringBoot,
    #[serde(rename = "rust")]
    Rust,
    #[serde(rename = "go")]
    Go,
    #[serde(rename = "php")]
    PHP,
    #[serde(rename = "ruby")]
    Ruby,
    #[serde(rename = "generic")]
    Generic,
}

// Project status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "analyzing")]
    Analyzing,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

// Playground session status enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlaygroundStatus {
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "error")]
    Error,
} 