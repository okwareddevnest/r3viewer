use sqlx::{SqlitePool, Row};
use anyhow::Result;
use crate::database::models::*;
use chrono::{DateTime, Utc};

pub async fn create_tables(pool: &SqlitePool) -> Result<()> {
    // Create students table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS students (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE,
            github_username TEXT,
            cohort TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create projects table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            student_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            github_url TEXT NOT NULL,
            technology_stack TEXT, -- JSON array as string
            status TEXT DEFAULT 'pending',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (student_id) REFERENCES students(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create analysis_results table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS analysis_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            code_quality_score INTEGER,
            structure_score INTEGER,
            documentation_score INTEGER,
            functionality_score INTEGER,
            total_score INTEGER,
            feedback TEXT,
            analysis_data TEXT, -- JSON as string
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id) REFERENCES projects(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create playground_sessions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS playground_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            container_id TEXT UNIQUE,
            port INTEGER,
            status TEXT DEFAULT 'starting',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (project_id) REFERENCES projects(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indices for better performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_projects_student_id ON projects(student_id)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_analysis_results_project_id ON analysis_results(project_id)")
        .execute(pool)
        .await?;
    
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_playground_sessions_project_id ON playground_sessions(project_id)")
        .execute(pool)
        .await?;

    Ok(())
}

// Student CRUD operations
pub async fn create_student(pool: &SqlitePool, student: CreateStudent) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO students (name, email, github_username, cohort) VALUES (?, ?, ?, ?)"
    )
    .bind(&student.name)
    .bind(&student.email)
    .bind(&student.github_username)
    .bind(&student.cohort)
    .execute(pool)
    .await?;
    
    Ok(result.last_insert_rowid())
}

pub async fn get_student_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Student>> {
    let student = sqlx::query_as::<_, Student>(
        "SELECT * FROM students WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    Ok(student)
}

pub async fn get_all_students(pool: &SqlitePool) -> Result<Vec<Student>> {
    let students = sqlx::query_as::<_, Student>(
        "SELECT * FROM students ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(students)
}

// Project CRUD operations
pub async fn create_project(pool: &SqlitePool, project: CreateProject) -> Result<i64> {
    let tech_stack_json = match project.technology_stack {
        Some(stack) => Some(serde_json::to_string(&stack)?),
        None => None,
    };
    
    let result = sqlx::query(
        "INSERT INTO projects (student_id, name, description, github_url, technology_stack) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(project.student_id)
    .bind(&project.name)
    .bind(&project.description)
    .bind(&project.github_url)
    .bind(&tech_stack_json)
    .execute(pool)
    .await?;
    
    Ok(result.last_insert_rowid())
}

pub async fn get_project_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Project>> {
    let project = sqlx::query_as::<_, Project>(
        "SELECT * FROM projects WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    Ok(project)
}

pub async fn get_all_projects(pool: &SqlitePool) -> Result<Vec<Project>> {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT * FROM projects ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(projects)
}

pub async fn get_projects_with_students(pool: &SqlitePool) -> Result<Vec<ProjectWithStudent>> {
    let rows = sqlx::query(
        r#"
        SELECT 
            p.id, p.student_id, p.name, p.description, p.github_url, 
            p.technology_stack, p.status, p.created_at,
            s.name as student_name, s.email as student_email, 
            s.github_username as student_github_username
        FROM projects p
        JOIN students s ON p.student_id = s.id
        ORDER BY p.created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    
    let mut projects = Vec::new();
    for row in rows {
        let tech_stack_str: Option<String> = row.get("technology_stack");
        let technology_stack: Option<Vec<String>> = match tech_stack_str {
            Some(json_str) => serde_json::from_str(&json_str).ok(),
            None => None,
        };
        
        projects.push(ProjectWithStudent {
            id: row.get("id"),
            student_id: row.get("student_id"),
            name: row.get("name"),
            description: row.get("description"),
            github_url: row.get("github_url"),
            technology_stack,
            status: row.get("status"),
            created_at: row.get("created_at"),
            student_name: row.get("student_name"),
            student_email: row.get("student_email"),
            student_github_username: row.get("student_github_username"),
        });
    }
    
    Ok(projects)
}

pub async fn update_project_status(pool: &SqlitePool, id: i64, status: &str) -> Result<()> {
    sqlx::query("UPDATE projects SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(())
}

// Analysis results CRUD operations
pub async fn create_analysis_result(pool: &SqlitePool, analysis: CreateAnalysisResult) -> Result<i64> {
    let analysis_data_json = match analysis.analysis_data {
        Some(data) => Some(serde_json::to_string(&data)?),
        None => None,
    };
    
    let result = sqlx::query(
        r#"
        INSERT INTO analysis_results (
            project_id, code_quality_score, structure_score, 
            documentation_score, functionality_score, total_score, 
            feedback, analysis_data
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(analysis.project_id)
    .bind(analysis.code_quality_score)
    .bind(analysis.structure_score)
    .bind(analysis.documentation_score)
    .bind(analysis.functionality_score)
    .bind(analysis.total_score)
    .bind(&analysis.feedback)
    .bind(&analysis_data_json)
    .execute(pool)
    .await?;
    
    Ok(result.last_insert_rowid())
}

pub async fn get_analysis_by_project_id(pool: &SqlitePool, project_id: i64) -> Result<Option<AnalysisResult>> {
    let analysis = sqlx::query_as::<_, AnalysisResult>(
        "SELECT * FROM analysis_results WHERE project_id = ? ORDER BY created_at DESC LIMIT 1"
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(analysis)
}

// Playground session CRUD operations
pub async fn create_playground_session(pool: &SqlitePool, session: CreatePlaygroundSession) -> Result<i64> {
    let result = sqlx::query(
        "INSERT INTO playground_sessions (project_id, container_id, port, status) VALUES (?, ?, ?, ?)"
    )
    .bind(session.project_id)
    .bind(&session.container_id)
    .bind(session.port)
    .bind(&session.status)
    .execute(pool)
    .await?;
    
    Ok(result.last_insert_rowid())
}

pub async fn get_playground_session_by_project_id(pool: &SqlitePool, project_id: i64) -> Result<Option<PlaygroundSession>> {
    let session = sqlx::query_as::<_, PlaygroundSession>(
        "SELECT * FROM playground_sessions WHERE project_id = ? ORDER BY created_at DESC LIMIT 1"
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(session)
}

pub async fn update_playground_session_status(pool: &SqlitePool, id: i64, status: &str) -> Result<()> {
    sqlx::query("UPDATE playground_sessions SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(())
} 