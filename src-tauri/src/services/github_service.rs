use anyhow::{Result, anyhow};
use octocrab::{Octocrab, models::Repository};
use git2::Repository as GitRepository;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use crate::services::AuthService;
use crate::database::models::{TechnologyStack, CreateStudent, CreateProject};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub clone_url: String,
    pub default_branch: String,
    pub technology_stack: Vec<TechnologyStack>,
    pub readme_content: Option<String>,
    pub has_dockerfile: bool,
    pub has_tests: bool,
    pub language: Option<String>,
    pub size: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub files: Vec<FileInfo>,
    pub directories: Vec<String>,
    pub package_files: Vec<PackageFile>,
    pub config_files: Vec<String>,
    pub documentation_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFile {
    pub path: String,
    pub file_type: PackageFileType,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackageFileType {
    PackageJson,
    RequirementsTxt,
    PomXml,
    CargoToml,
    GoMod,
    ComposerJson,
    Gemfile,
    Unknown,
}

pub struct GitHubService {
    client: Option<Octocrab>,
    auth_service: AuthService,
}

impl GitHubService {
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            client: None,
            auth_service,
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        let credentials = self.auth_service.get_stored_credentials()?;
        
        if let Some(token) = credentials.github_token {
            let octocrab = Octocrab::builder()
                .personal_token(token)
                .build()?;
            self.client = Some(octocrab);
        }

        Ok(())
    }

    pub async fn get_repository_info(&self, repo_url: &str) -> Result<RepositoryInfo> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("GitHub client not initialized"))?;

        let (owner, repo_name) = self.parse_github_url(repo_url)?;
        
        let repo = client
            .repos(&owner, &repo_name)
            .get()
            .await?;

        let technology_stack = self.detect_technology_stack(&owner, &repo_name).await?;
        let readme_content = self.get_readme_content(&owner, &repo_name).await.ok();

        Ok(RepositoryInfo {
            name: repo.name,
            description: repo.description,
            url: repo.html_url.map(|u| u.to_string()).unwrap_or_default(),
            clone_url: repo.clone_url.unwrap_or_default(),
            default_branch: repo.default_branch.unwrap_or_else(|| "main".to_string()),
            technology_stack,
            readme_content,
            has_dockerfile: self.check_file_exists(&owner, &repo_name, "Dockerfile").await.unwrap_or(false),
            has_tests: self.detect_test_files(&owner, &repo_name).await.unwrap_or(false),
            language: repo.language,
            size: repo.size.unwrap_or(0),
            created_at: repo.created_at.map(|d| d.to_string()).unwrap_or_default(),
            updated_at: repo.updated_at.map(|d| d.to_string()).unwrap_or_default(),
        })
    }

    pub async fn clone_repository(&self, repo_url: &str, target_dir: &Path) -> Result<PathBuf> {
        let credentials = self.auth_service.get_stored_credentials()?;
        let token = credentials.github_token
            .ok_or_else(|| anyhow!("No GitHub token available"))?;

        // Create target directory if it doesn't exist
        fs::create_dir_all(target_dir)?;

        // Prepare authenticated clone URL
        let auth_url = if repo_url.starts_with("https://github.com/") {
            repo_url.replace("https://github.com/", &format!("https://{}@github.com/", token))
        } else {
            repo_url.to_string()
        };

        let repo_name = self.extract_repo_name(repo_url)?;
        let clone_path = target_dir.join(&repo_name);

        // Remove existing directory if it exists
        if clone_path.exists() {
            fs::remove_dir_all(&clone_path)?;
        }

        // Clone the repository
        GitRepository::clone(&auth_url, &clone_path)
            .map_err(|e| anyhow!("Failed to clone repository: {}", e))?;

        Ok(clone_path)
    }

    pub async fn analyze_project_structure(&self, project_path: &Path) -> Result<ProjectStructure> {
        let mut files = Vec::new();
        let mut directories = Vec::new();
        let mut package_files = Vec::new();
        let mut config_files = Vec::new();
        let mut documentation_files = Vec::new();

        self.scan_directory(
            project_path, 
            project_path, 
            &mut files, 
            &mut directories,
            &mut package_files,
            &mut config_files,
            &mut documentation_files,
            0
        )?;

        Ok(ProjectStructure {
            files,
            directories,
            package_files,
            config_files,
            documentation_files,
        })
    }

    fn scan_directory(
        &self,
        current_path: &Path,
        base_path: &Path,
        files: &mut Vec<FileInfo>,
        directories: &mut Vec<String>,
        package_files: &mut Vec<PackageFile>,
        config_files: &mut Vec<String>,
        documentation_files: &mut Vec<String>,
        depth: usize,
    ) -> Result<()> {
        if depth > 5 { // Limit recursion depth
            return Ok(());
        }

        for entry in fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and common ignore patterns
            if file_name.starts_with('.') || 
               file_name == "node_modules" || 
               file_name == "target" ||
               file_name == "__pycache__" ||
               file_name == "vendor" {
                continue;
            }

            let relative_path = path.strip_prefix(base_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            if path.is_dir() {
                directories.push(relative_path.clone());
                self.scan_directory(&path, base_path, files, directories, package_files, config_files, documentation_files, depth + 1)?;
            } else {
                let metadata = fs::metadata(&path)?;
                let extension = path.extension().map(|ext| ext.to_string_lossy().to_string());
                
                let file_info = FileInfo {
                    path: relative_path.clone(),
                    name: file_name.clone(),
                    extension: extension.clone(),
                    size: metadata.len(),
                    is_binary: self.is_binary_file(&path)?,
                };

                files.push(file_info);

                // Categorize special files
                match file_name.as_str() {
                    "package.json" => {
                        package_files.push(PackageFile {
                            path: relative_path.clone(),
                            file_type: PackageFileType::PackageJson,
                            dependencies: self.extract_npm_dependencies(&path).ok(),
                        });
                    }
                    "requirements.txt" => {
                        package_files.push(PackageFile {
                            path: relative_path.clone(),
                            file_type: PackageFileType::RequirementsTxt,
                            dependencies: self.extract_pip_dependencies(&path).ok(),
                        });
                    }
                    "pom.xml" => {
                        package_files.push(PackageFile {
                            path: relative_path.clone(),
                            file_type: PackageFileType::PomXml,
                            dependencies: None, // Could implement XML parsing
                        });
                    }
                    "Cargo.toml" => {
                        package_files.push(PackageFile {
                            path: relative_path.clone(),
                            file_type: PackageFileType::CargoToml,
                            dependencies: None, // Could implement TOML parsing
                        });
                    }
                    _ => {}
                }

                // Configuration files
                if file_name.ends_with(".config") || 
                   file_name.ends_with(".yml") ||
                   file_name.ends_with(".yaml") ||
                   file_name.ends_with(".json") ||
                   file_name == "Dockerfile" ||
                   file_name == "docker-compose.yml" {
                    config_files.push(relative_path.clone());
                }

                // Documentation files
                if file_name.to_lowercase().starts_with("readme") ||
                   file_name.ends_with(".md") ||
                   file_name.ends_with(".txt") ||
                   file_name.ends_with(".rst") {
                    documentation_files.push(relative_path);
                }
            }
        }

        Ok(())
    }

    async fn detect_technology_stack(&self, owner: &str, repo: &str) -> Result<Vec<TechnologyStack>> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("GitHub client not initialized"))?;

        let mut stacks = Vec::new();

        // Check for common package files
        if self.check_file_exists(owner, repo, "package.json").await.unwrap_or(false) {
            stacks.push(TechnologyStack::NodeJS);
            
            // Check for specific frameworks
            if let Ok(package_content) = self.get_file_content(owner, repo, "package.json").await {
                if package_content.contains("\"react\"") {
                    stacks.push(TechnologyStack::React);
                }
                if package_content.contains("\"vue\"") {
                    stacks.push(TechnologyStack::Vue);
                }
                if package_content.contains("\"@angular/core\"") {
                    stacks.push(TechnologyStack::Angular);
                }
            }
        }

        if self.check_file_exists(owner, repo, "requirements.txt").await.unwrap_or(false) ||
           self.check_file_exists(owner, repo, "setup.py").await.unwrap_or(false) {
            stacks.push(TechnologyStack::Python);
            
            // Check for Python frameworks
            if let Ok(req_content) = self.get_file_content(owner, repo, "requirements.txt").await {
                if req_content.contains("Django") {
                    stacks.push(TechnologyStack::Django);
                }
                if req_content.contains("Flask") {
                    stacks.push(TechnologyStack::Flask);
                }
            }
        }

        if self.check_file_exists(owner, repo, "pom.xml").await.unwrap_or(false) ||
           self.check_file_exists(owner, repo, "build.gradle").await.unwrap_or(false) {
            stacks.push(TechnologyStack::Java);
            
            // Check for Spring Boot
            if let Ok(pom_content) = self.get_file_content(owner, repo, "pom.xml").await {
                if pom_content.contains("spring-boot") {
                    stacks.push(TechnologyStack::SpringBoot);
                }
            }
        }

        if self.check_file_exists(owner, repo, "Cargo.toml").await.unwrap_or(false) {
            stacks.push(TechnologyStack::Rust);
        }

        if self.check_file_exists(owner, repo, "go.mod").await.unwrap_or(false) {
            stacks.push(TechnologyStack::Go);
        }

        if self.check_file_exists(owner, repo, "composer.json").await.unwrap_or(false) {
            stacks.push(TechnologyStack::PHP);
        }

        if self.check_file_exists(owner, repo, "Gemfile").await.unwrap_or(false) {
            stacks.push(TechnologyStack::Ruby);
        }

        if stacks.is_empty() {
            stacks.push(TechnologyStack::Generic);
        }

        Ok(stacks)
    }

    async fn check_file_exists(&self, owner: &str, repo: &str, file_path: &str) -> Result<bool> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("GitHub client not initialized"))?;

        match client.repos(owner, repo).get_content().path(file_path).send().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_file_content(&self, owner: &str, repo: &str, file_path: &str) -> Result<String> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("GitHub client not initialized"))?;

        let content = client.repos(owner, repo).get_content().path(file_path).send().await?;
        
        if let Some(file) = content.items.first() {
            if let Some(content_str) = &file.content {
                let decoded = base64::decode(content_str.replace('\n', ""))?;
                return Ok(String::from_utf8(decoded)?);
            }
        }

        Err(anyhow!("File content not found"))
    }

    async fn get_readme_content(&self, owner: &str, repo: &str) -> Result<String> {
        for readme_name in &["README.md", "README.txt", "README.rst", "README"] {
            if let Ok(content) = self.get_file_content(owner, repo, readme_name).await {
                return Ok(content);
            }
        }
        Err(anyhow!("No README file found"))
    }

    async fn detect_test_files(&self, owner: &str, repo: &str) -> Result<bool> {
        let test_patterns = &["test", "tests", "__tests__", "spec", "specs"];
        
        for pattern in test_patterns {
            if self.check_file_exists(owner, repo, pattern).await.unwrap_or(false) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn parse_github_url(&self, url: &str) -> Result<(String, String)> {
        let url = url.trim_end_matches('/').trim_end_matches(".git");
        
        if let Some(captures) = regex::Regex::new(r"github\.com/([^/]+)/([^/]+)")
            .unwrap()
            .captures(url) {
            let owner = captures.get(1).unwrap().as_str().to_string();
            let repo = captures.get(2).unwrap().as_str().to_string();
            Ok((owner, repo))
        } else {
            Err(anyhow!("Invalid GitHub URL format"))
        }
    }

    fn extract_repo_name(&self, url: &str) -> Result<String> {
        let url = url.trim_end_matches('/').trim_end_matches(".git");
        
        if let Some(name) = url.split('/').last() {
            Ok(name.to_string())
        } else {
            Err(anyhow!("Could not extract repository name"))
        }
    }

    fn is_binary_file(&self, path: &Path) -> Result<bool> {
        let buffer = fs::read(path)?;
        let sample_size = std::cmp::min(buffer.len(), 1024);
        
        for byte in &buffer[..sample_size] {
            if *byte == 0 {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    fn extract_npm_dependencies(&self, package_json_path: &Path) -> Result<Vec<String>> {
        let content = fs::read_to_string(package_json_path)?;
        let package: serde_json::Value = serde_json::from_str(&content)?;
        
        let mut dependencies = Vec::new();
        
        if let Some(deps) = package["dependencies"].as_object() {
            dependencies.extend(deps.keys().cloned());
        }
        
        if let Some(dev_deps) = package["devDependencies"].as_object() {
            dependencies.extend(dev_deps.keys().cloned());
        }
        
        Ok(dependencies)
    }

    fn extract_pip_dependencies(&self, requirements_path: &Path) -> Result<Vec<String>> {
        let content = fs::read_to_string(requirements_path)?;
        let dependencies: Vec<String> = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(|line| {
                // Extract package name before version specifiers
                line.split_whitespace()
                    .next()
                    .unwrap_or(line)
                    .split(&['=', '>', '<', '!', '~'][..])
                    .next()
                    .unwrap_or(line)
                    .to_string()
            })
            .collect();
        
        Ok(dependencies)
    }

    pub fn validate_github_url(&self, url: &str) -> bool {
        regex::Regex::new(r"^https://github\.com/[^/]+/[^/]+/?(?:\.git)?$")
            .unwrap()
            .is_match(url)
    }
} 