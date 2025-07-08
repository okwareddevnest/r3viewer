// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod services;
mod commands;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;

use database::Database;
use services::*;
use commands::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Initialize async runtime for setup
            tauri::async_runtime::spawn(async move {
                match initialize_app_state(&app_handle).await {
                    Ok(app_state) => {
                        app_handle.manage(app_state);
                        println!("✅ r3viewer initialized successfully");
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to initialize r3viewer: {}", e);
                        std::process::exit(1);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Authentication Commands
            commands::get_auth_status,
            commands::generate_google_auth_url,
            commands::exchange_google_code,
            commands::validate_github_token,
            commands::logout,
            
            // Google Sheets Commands
            commands::get_sheet_data,
            commands::parse_and_validate_sheet_data,
            commands::import_students_from_sheet,
            commands::extract_spreadsheet_id,
            commands::export_results_to_sheet,
            commands::export_project_results,
            
            // Project Management Commands
            commands::get_all_projects,
            commands::get_project_by_id,
            commands::update_project_status,
            
            // GitHub Integration Commands
            commands::get_repository_info,
            commands::clone_repository,
            commands::analyze_project_structure,
            commands::validate_github_url,
            
            // Analysis Commands
            commands::analyze_project,
            commands::get_analysis_by_project_id,
            
            // Playground Commands
            commands::start_playground,
            commands::stop_playground,
            commands::get_playground_status,
            commands::get_playground_resource_usage,
            commands::list_active_playgrounds,
            commands::cleanup_old_containers,
            
            // Utility Commands
            commands::get_app_data_dir,
            commands::check_docker_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn initialize_app_state(app_handle: &tauri::AppHandle) -> anyhow::Result<AppState> {
    println!("🔄 Initializing r3viewer...");

    // Initialize database
    println!("🗄️  Setting up database...");
    let db = Arc::new(Database::new(app_handle).await?);
    
    // Initialize auth service
    println!("🔐 Setting up authentication...");
    let auth_service = Arc::new(AuthService::new());
    
    // Initialize GitHub service
    println!("🐙 Setting up GitHub integration...");
    let mut github_service = GitHubService::new((*auth_service).clone());
    if let Err(e) = github_service.initialize().await {
        eprintln!("⚠️  GitHub service initialization failed: {}. GitHub features may be limited.", e);
    }
    let github_service = Arc::new(Mutex::new(github_service));
    
    // Initialize Google Sheets service
    println!("📊 Setting up Google Sheets integration...");
    let sheets_service = Arc::new(SheetsService::new((*auth_service).clone()));
    
    // Initialize Docker service
    println!("🐳 Setting up Docker playground...");
    let docker_service = match DockerService::new().await {
        Ok(service) => {
            println!("✅ Docker service initialized successfully");
            Arc::new(Mutex::new(service))
        }
        Err(e) => {
            eprintln!("⚠️  Docker service initialization failed: {}. Playground features will be disabled.", e);
            // For now, we'll create a placeholder that panics - this should be improved
            // to return a proper dummy service
            Arc::new(Mutex::new(create_dummy_docker_service().unwrap()))
        }
    };
    
    // Initialize analysis service
    println!("🔍 Setting up analysis engine...");
    let github_service_clone = {
        let github_guard = github_service.lock().await;
        (*github_guard).clone()
    };
    let analysis_service = Arc::new(AnalysisService::new(github_service_clone));
    
    println!("✅ All services initialized successfully");

    Ok(AppState {
        db,
        auth_service,
        github_service,
        sheets_service,
        docker_service,
        analysis_service,
    })
}

// Create a dummy docker service for when Docker is not available
fn create_dummy_docker_service() -> anyhow::Result<DockerService> {
    // In a real implementation, this would return a mock/dummy service
    // For now, we'll return an error to indicate Docker is unavailable
    Err(anyhow::anyhow!("Docker service is not available"))
}
