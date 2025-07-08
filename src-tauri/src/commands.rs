use crate::database::{Database, schema};
use crate::services::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

// Application state structure
pub struct AppState {
    pub db: Arc<Database>,
    pub auth_service: Arc<AuthService>,
    pub github_service: Arc<Mutex<GitHubService>>,
    pub sheets_service: Arc<SheetsService>,
    pub docker_service: Arc<Mutex<DockerService>>,
    pub analysis_service: Arc<AnalysisService>,
}

// Authentication Commands
#[tauri::command]
pub async fn get_auth_status(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    state.auth_service
        .get_auth_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn generate_google_auth_url(state: State<'_, AppState>) -> Result<GoogleAuthUrl, String> {
    state.auth_service
        .generate_google_auth_url()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn exchange_google_code(
    code: String,
    csrf_token: String,
    pkce_verifier: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    state.auth_service
        .exchange_google_code(code, csrf_token, pkce_verifier)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_github_token(
    token: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    state.auth_service
        .validate_github_token(&token)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    state.auth_service
        .logout()
        .map_err(|e| e.to_string())
}

// Google Sheets Commands
#[tauri::command]
pub async fn get_sheet_data(
    spreadsheet_id: String,
    range: String,
    state: State<'_, AppState>
) -> Result<SheetData, String> {
    state.sheets_service
        .get_sheet_data(&spreadsheet_id, &range)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn parse_and_validate_sheet_data(
    sheet_data: SheetData,
    mapping: SheetMapping,
    state: State<'_, AppState>
) -> Result<(Vec<StudentData>, Vec<String>), String> {
    let students = state.sheets_service
        .parse_student_data(&sheet_data, &mapping)
        .map_err(|e| e.to_string())?;
    
    let errors = state.sheets_service
        .validate_student_data(&students)
        .map_err(|e| e.to_string())?;
    
    Ok((students, errors))
}

#[tauri::command]
pub async fn import_students_from_sheet(
    students_data: Vec<StudentData>,
    state: State<'_, AppState>
) -> Result<ImportResult, String> {
    let mut students_imported = 0;
    let mut projects_imported = 0;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Convert to CreateStudent structs
    let create_students = state.sheets_service.convert_to_create_students(&students_data);
    
    // Import students
    let mut student_ids = std::collections::HashMap::new();
    for create_student in create_students {
        match schema::create_student(&state.db.pool, create_student.clone()).await {
            Ok(id) => {
                student_ids.insert(create_student.name.clone(), id);
                students_imported += 1;
            }
            Err(e) => {
                errors.push(format!("Failed to import student {}: {}", create_student.name, e));
            }
        }
    }

    // Import projects
    let create_projects = state.sheets_service.convert_to_create_projects(&students_data, &student_ids);
    for create_project in create_projects {
        match schema::create_project(&state.db.pool, create_project.clone()).await {
            Ok(_) => {
                projects_imported += 1;
            }
            Err(e) => {
                errors.push(format!("Failed to import project {}: {}", create_project.name, e));
            }
        }
    }

    Ok(ImportResult {
        students_imported,
        projects_imported,
        errors,
        warnings,
    })
}

#[tauri::command]
pub async fn extract_spreadsheet_id(url: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.sheets_service.extract_spreadsheet_id(&url))
}

// Project Management Commands
#[tauri::command]
pub async fn get_all_projects(state: State<'_, AppState>) -> Result<Vec<crate::database::models::ProjectWithStudent>, String> {
    schema::get_projects_with_students(&state.db.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_project_by_id(id: i64, state: State<'_, AppState>) -> Result<Option<crate::database::models::Project>, String> {
    schema::get_project_by_id(&state.db.pool, id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_project_status(
    id: i64,
    status: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    schema::update_project_status(&state.db.pool, id, &status)
        .await
        .map_err(|e| e.to_string())
}

// GitHub Integration Commands
#[tauri::command]
pub async fn get_repository_info(
    repo_url: String,
    state: State<'_, AppState>
) -> Result<RepositoryInfo, String> {
    let github_service = state.github_service.lock().await;
    github_service
        .get_repository_info(&repo_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clone_repository(
    repo_url: String,
    target_dir: String,
    state: State<'_, AppState>
) -> Result<String, String> {
    let github_service = state.github_service.lock().await;
    let target_path = std::path::Path::new(&target_dir);
    let cloned_path = github_service
        .clone_repository(&repo_url, target_path)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(cloned_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn analyze_project_structure(
    project_path: String,
    state: State<'_, AppState>
) -> Result<ProjectStructure, String> {
    let github_service = state.github_service.lock().await;
    let path = std::path::Path::new(&project_path);
    github_service
        .analyze_project_structure(path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_github_url(url: String, state: State<'_, AppState>) -> Result<bool, String> {
    let github_service = state.github_service.lock().await;
    Ok(github_service.validate_github_url(&url))
}

// Analysis Commands
#[tauri::command]
pub async fn analyze_project(
    project_id: i64,
    state: State<'_, AppState>
) -> Result<crate::services::analysis_service::AnalysisResult, String> {
    // Get project details
    let project = schema::get_project_by_id(&state.db.pool, project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    // Update project status to analyzing
    schema::update_project_status(&state.db.pool, project_id, "analyzing")
        .await
        .map_err(|e| e.to_string())?;

    // Clone repository for analysis
    let temp_dir = std::env::temp_dir().join(format!("r3viewer_analysis_{}", project_id));
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let github_service = state.github_service.lock().await;
    let project_path = github_service
        .clone_repository(&project.github_url, &temp_dir)
        .await
        .map_err(|e| e.to_string())?;

    // Detect technology stack
    let repo_info = github_service
        .get_repository_info(&project.github_url)
        .await
        .map_err(|e| e.to_string())?;

    drop(github_service); // Release the lock

    // Perform analysis
    let analysis_result = state.analysis_service
        .analyze_project(&project_path, &repo_info.technology_stack)
        .await
        .map_err(|e| e.to_string())?;

    // Save analysis results
    let create_analysis = state.analysis_service
        .convert_to_create_analysis_result(project_id, &analysis_result);

    schema::create_analysis_result(&state.db.pool, create_analysis)
        .await
        .map_err(|e| e.to_string())?;

    // Update project status to completed
    schema::update_project_status(&state.db.pool, project_id, "completed")
        .await
        .map_err(|e| e.to_string())?;

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(analysis_result)
}

#[tauri::command]
pub async fn get_analysis_by_project_id(
    project_id: i64,
    state: State<'_, AppState>
) -> Result<Option<crate::database::models::AnalysisResult>, String> {
    schema::get_analysis_by_project_id(&state.db.pool, project_id)
        .await
        .map_err(|e| e.to_string())
}

// Playground Commands
#[tauri::command]
pub async fn start_playground(
    project_id: i64,
    state: State<'_, AppState>
) -> Result<PlaygroundInfo, String> {
    // Get project details
    let project = schema::get_project_by_id(&state.db.pool, project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Project not found".to_string())?;

    // Clone repository for playground
    let temp_dir = std::env::temp_dir().join(format!("r3viewer_playground_{}", project_id));
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let github_service = state.github_service.lock().await;
    let project_path = github_service
        .clone_repository(&project.github_url, &temp_dir)
        .await
        .map_err(|e| e.to_string())?;

    // Get repository info for tech stack
    let repo_info = github_service
        .get_repository_info(&project.github_url)
        .await
        .map_err(|e| e.to_string())?;

    drop(github_service); // Release the lock

    // Start playground container
    let docker_service = state.docker_service.lock().await;
    let playground_info = docker_service
        .start_playground(&project_path, &repo_info.technology_stack)
        .await
        .map_err(|e| e.to_string())?;

    // Save playground session
    let create_session = crate::database::models::CreatePlaygroundSession {
        project_id,
        container_id: Some(playground_info.container_id.clone()),
        port: Some(playground_info.port as i32),
        status: "running".to_string(),
    };

    schema::create_playground_session(&state.db.pool, create_session)
        .await
        .map_err(|e| e.to_string())?;

    Ok(playground_info)
}

#[tauri::command]
pub async fn stop_playground(
    project_id: i64,
    state: State<'_, AppState>
) -> Result<(), String> {
    // Get playground session
    let session = schema::get_playground_session_by_project_id(&state.db.pool, project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No playground session found".to_string())?;

    if let Some(container_id) = &session.container_id {
        let docker_service = state.docker_service.lock().await;
        docker_service
            .stop_playground(container_id)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Update session status
    schema::update_playground_session_status(&state.db.pool, session.id, "stopped")
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_playground_status(
    project_id: i64,
    state: State<'_, AppState>
) -> Result<Option<PlaygroundStatus>, String> {
    let session = schema::get_playground_session_by_project_id(&state.db.pool, project_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(session) = session {
        if let Some(container_id) = &session.container_id {
            let docker_service = state.docker_service.lock().await;
            let status = docker_service
                .get_playground_status(container_id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Some(status))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_playground_resource_usage(
    project_id: i64,
    state: State<'_, AppState>
) -> Result<Option<ResourceUsage>, String> {
    let session = schema::get_playground_session_by_project_id(&state.db.pool, project_id)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(session) = session {
        if let Some(container_id) = &session.container_id {
            let docker_service = state.docker_service.lock().await;
            let usage = docker_service
                .get_resource_usage(container_id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Some(usage))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn list_active_playgrounds(state: State<'_, AppState>) -> Result<Vec<bollard::models::ContainerSummary>, String> {
    let docker_service = state.docker_service.lock().await;
    docker_service
        .list_active_playgrounds()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cleanup_old_containers(
    max_age_hours: u64,
    state: State<'_, AppState>
) -> Result<usize, String> {
    let docker_service = state.docker_service.lock().await;
    docker_service
        .cleanup_old_containers(max_age_hours)
        .await
        .map_err(|e| e.to_string())
}

// Utility Commands
#[tauri::command]
pub async fn get_app_data_dir(app_handle: AppHandle) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    Ok(app_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn check_docker_status(state: State<'_, AppState>) -> Result<bool, String> {
    let docker_service = state.docker_service.lock().await;
    // Try to list containers to check if Docker is running
    match docker_service.list_active_playgrounds().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// Export/Import Commands
#[tauri::command]
pub async fn export_results_to_sheet(
    spreadsheet_id: String,
    range: String,
    results: Vec<ExportRow>,
    state: State<'_, AppState>
) -> Result<(), String> {
    state.sheets_service
        .export_results_to_sheet(&spreadsheet_id, &range, &results)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_project_results(
    project_ids: Vec<i64>,
    state: State<'_, AppState>
) -> Result<Vec<ExportRow>, String> {
    let mut results = Vec::new();

    for project_id in project_ids {
        let project = schema::get_project_by_id(&state.db.pool, project_id)
            .await
            .map_err(|e| e.to_string())?;
        
        let student = if let Some(proj) = &project {
            schema::get_student_by_id(&state.db.pool, proj.student_id)
                .await
                .map_err(|e| e.to_string())?
        } else {
            None
        };

        let analysis = schema::get_analysis_by_project_id(&state.db.pool, project_id)
            .await
            .map_err(|e| e.to_string())?;

        if let (Some(project), Some(student)) = (project, student) {
            results.push(ExportRow {
                student_name: student.name,
                project_name: project.name,
                total_score: analysis.as_ref().and_then(|a| a.total_score),
                code_quality_score: analysis.as_ref().and_then(|a| a.code_quality_score),
                structure_score: analysis.as_ref().and_then(|a| a.structure_score),
                documentation_score: analysis.as_ref().and_then(|a| a.documentation_score),
                functionality_score: analysis.as_ref().and_then(|a| a.functionality_score),
                feedback: analysis.as_ref().and_then(|a| a.feedback.clone()),
            });
        }
    }

    Ok(results)
} 