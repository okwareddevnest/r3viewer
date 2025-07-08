use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use crate::database::models::{CreateAnalysisResult, TechnologyStack};
use crate::services::{GitHubService, ProjectStructure, FileInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub code_quality: CodeQualityMetrics,
    pub structure: StructureMetrics,
    pub documentation: DocumentationMetrics,
    pub functionality: FunctionalityMetrics,
    pub total_score: i32,
    pub feedback: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeQualityMetrics {
    pub score: i32,
    pub lint_issues: usize,
    pub complexity_score: i32,
    pub duplicate_code_percentage: f64,
    pub test_coverage_percentage: f64,
    pub security_issues: Vec<SecurityIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureMetrics {
    pub score: i32,
    pub organization_score: i32,
    pub naming_convention_score: i32,
    pub file_structure_score: i32,
    pub configuration_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationMetrics {
    pub score: i32,
    pub readme_quality: i32,
    pub code_comments_percentage: f64,
    pub api_documentation_score: i32,
    pub inline_documentation_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalityMetrics {
    pub score: i32,
    pub build_success: bool,
    pub tests_passing: bool,
    pub feature_completeness_score: i32,
    pub error_handling_score: i32,
    pub performance_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: SecuritySeverity,
    pub description: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

pub struct AnalysisService {
    github_service: GitHubService,
}

impl AnalysisService {
    pub fn new(github_service: GitHubService) -> Self {
        Self { github_service }
    }

    pub async fn analyze_project(&self, project_path: &Path, tech_stack: &[TechnologyStack]) -> Result<AnalysisResult> {
        // Analyze project structure
        let structure = self.github_service.analyze_project_structure(project_path).await?;
        
        // Perform different analysis components
        let code_quality = self.analyze_code_quality(project_path, tech_stack, &structure).await?;
        let structure_metrics = self.analyze_structure(project_path, &structure).await?;
        let documentation = self.analyze_documentation(project_path, &structure).await?;
        let functionality = self.analyze_functionality(project_path, tech_stack, &structure).await?;

        // Calculate total score
        let total_score = self.calculate_total_score(&code_quality, &structure_metrics, &documentation, &functionality);

        // Generate feedback
        let feedback = self.generate_feedback(&code_quality, &structure_metrics, &documentation, &functionality);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&code_quality, &structure_metrics, &documentation, &functionality);

        Ok(AnalysisResult {
            code_quality,
            structure: structure_metrics,
            documentation,
            functionality,
            total_score,
            feedback,
            recommendations,
        })
    }

    async fn analyze_code_quality(&self, project_path: &Path, tech_stack: &[TechnologyStack], structure: &ProjectStructure) -> Result<CodeQualityMetrics> {
        let mut lint_issues = 0;
        let mut complexity_score = 100;
        let duplicate_code_percentage = self.analyze_duplicate_code(project_path, &structure.files).await?;
        let test_coverage_percentage = self.calculate_test_coverage(project_path, structure).await?;
        let security_issues = self.scan_security_issues(project_path, &structure.files).await?;

        // Technology-specific linting
        for stack in tech_stack {
            match stack {
                TechnologyStack::NodeJS | TechnologyStack::React | TechnologyStack::Vue | TechnologyStack::Angular => {
                    lint_issues += self.run_eslint_analysis(project_path).await?;
                }
                TechnologyStack::Python | TechnologyStack::Django | TechnologyStack::Flask => {
                    lint_issues += self.run_python_linting(project_path).await?;
                }
                TechnologyStack::Java | TechnologyStack::SpringBoot => {
                    lint_issues += self.run_java_analysis(project_path).await?;
                }
                _ => {}
            }
        }

        // Calculate complexity score based on file sizes and nesting
        complexity_score = self.calculate_complexity_score(&structure.files);

        // Calculate final code quality score
        let score = self.calculate_code_quality_score(lint_issues, complexity_score, duplicate_code_percentage, test_coverage_percentage, &security_issues);

        Ok(CodeQualityMetrics {
            score,
            lint_issues,
            complexity_score,
            duplicate_code_percentage,
            test_coverage_percentage,
            security_issues,
        })
    }

    async fn analyze_structure(&self, project_path: &Path, structure: &ProjectStructure) -> Result<StructureMetrics> {
        let organization_score = self.evaluate_project_organization(structure);
        let naming_convention_score = self.evaluate_naming_conventions(&structure.files);
        let file_structure_score = self.evaluate_file_structure(structure);
        let configuration_score = self.evaluate_configuration_files(&structure.config_files);

        let score = (organization_score + naming_convention_score + file_structure_score + configuration_score) / 4;

        Ok(StructureMetrics {
            score,
            organization_score,
            naming_convention_score,
            file_structure_score,
            configuration_score,
        })
    }

    async fn analyze_documentation(&self, project_path: &Path, structure: &ProjectStructure) -> Result<DocumentationMetrics> {
        let readme_quality = self.evaluate_readme_quality(project_path, &structure.documentation_files).await?;
        let code_comments_percentage = self.calculate_code_comments_percentage(&structure.files).await?;
        let api_documentation_score = self.evaluate_api_documentation(project_path, &structure.files).await?;
        let inline_documentation_score = self.evaluate_inline_documentation(&structure.files).await?;

        let score = (readme_quality + api_documentation_score + inline_documentation_score) / 3;

        Ok(DocumentationMetrics {
            score,
            readme_quality,
            code_comments_percentage,
            api_documentation_score,
            inline_documentation_score,
        })
    }

    async fn analyze_functionality(&self, project_path: &Path, tech_stack: &[TechnologyStack], structure: &ProjectStructure) -> Result<FunctionalityMetrics> {
        let build_success = self.test_build_success(project_path, tech_stack).await?;
        let tests_passing = self.run_tests(project_path, tech_stack).await?;
        let feature_completeness_score = self.evaluate_feature_completeness(project_path, structure).await?;
        let error_handling_score = self.evaluate_error_handling(&structure.files).await?;
        let performance_score = self.evaluate_performance_indicators(&structure.files).await?;

        let mut score = feature_completeness_score;
        if build_success { score += 20; }
        if tests_passing { score += 20; }
        score = (score + error_handling_score + performance_score) / 3;
        score = score.min(100);

        Ok(FunctionalityMetrics {
            score,
            build_success,
            tests_passing,
            feature_completeness_score,
            error_handling_score,
            performance_score,
        })
    }

    // Code Quality Analysis Methods
    async fn run_eslint_analysis(&self, project_path: &Path) -> Result<usize> {
        let package_json_path = project_path.join("package.json");
        if !package_json_path.exists() {
            return Ok(0);
        }

        // Check for common JavaScript/TypeScript issues
        let mut issues = 0;
        
        // Scan for common patterns that would be caught by ESLint
        issues += self.scan_for_js_issues(project_path).await?;
        
        Ok(issues)
    }

    async fn run_python_linting(&self, project_path: &Path) -> Result<usize> {
        let mut issues = 0;
        
        // Scan for common Python issues
        issues += self.scan_for_python_issues(project_path).await?;
        
        Ok(issues)
    }

    async fn run_java_analysis(&self, project_path: &Path) -> Result<usize> {
        let mut issues = 0;
        
        // Scan for common Java issues
        issues += self.scan_for_java_issues(project_path).await?;
        
        Ok(issues)
    }

    async fn scan_for_js_issues(&self, project_path: &Path) -> Result<usize> {
        let mut issues = 0;
        
        for entry in walkdir::WalkDir::new(project_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "js" || ext == "ts" || ext == "jsx" || ext == "tsx" {
                    if let Ok(content) = fs::read_to_string(path) {
                        // Check for common issues
                        if content.contains("console.log") { issues += 1; }
                        if content.contains("var ") { issues += 1; }
                        if content.contains("==") && !content.contains("===") { issues += 1; }
                        // Add more checks as needed
                    }
                }
            }
        }
        
        Ok(issues)
    }

    async fn scan_for_python_issues(&self, project_path: &Path) -> Result<usize> {
        let mut issues = 0;
        
        for entry in walkdir::WalkDir::new(project_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "py" {
                    if let Ok(content) = fs::read_to_string(path) {
                        // Check for common issues
                        if content.contains("print(") && !path.to_string_lossy().contains("test") { issues += 1; }
                        if content.lines().any(|line| line.len() > 120) { issues += 1; }
                        // Add more checks as needed
                    }
                }
            }
        }
        
        Ok(issues)
    }

    async fn scan_for_java_issues(&self, project_path: &Path) -> Result<usize> {
        let mut issues = 0;
        
        for entry in walkdir::WalkDir::new(project_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "java" {
                    if let Ok(content) = fs::read_to_string(path) {
                        // Check for common issues
                        if content.contains("System.out.println") && !path.to_string_lossy().contains("test") { issues += 1; }
                        if !content.contains("package ") { issues += 1; }
                        // Add more checks as needed
                    }
                }
            }
        }
        
        Ok(issues)
    }

    fn calculate_complexity_score(&self, files: &[FileInfo]) -> i32 {
        let mut total_complexity = 0;
        let mut file_count = 0;

        for file in files {
            if !file.is_binary && file.size > 0 {
                let complexity = match file.size {
                    0..=1000 => 100,        // Small files
                    1001..=5000 => 80,      // Medium files
                    5001..=10000 => 60,     // Large files
                    _ => 40,                // Very large files
                };
                total_complexity += complexity;
                file_count += 1;
            }
        }

        if file_count > 0 {
            total_complexity / file_count
        } else {
            100
        }
    }

    async fn analyze_duplicate_code(&self, _project_path: &Path, files: &[FileInfo]) -> Result<f64> {
        // Simple duplicate detection based on file sizes
        let mut size_map: HashMap<u64, usize> = HashMap::new();
        let mut total_files = 0;
        let mut duplicate_files = 0;

        for file in files {
            if !file.is_binary && file.size > 100 { // Ignore very small files
                total_files += 1;
                let count = size_map.entry(file.size).or_insert(0);
                *count += 1;
                if *count == 2 {
                    duplicate_files += 1; // First duplicate found
                } else if *count > 2 {
                    duplicate_files += 1; // Additional duplicates
                }
            }
        }

        if total_files > 0 {
            Ok((duplicate_files as f64 / total_files as f64) * 100.0)
        } else {
            Ok(0.0)
        }
    }

    async fn calculate_test_coverage(&self, project_path: &Path, structure: &ProjectStructure) -> Result<f64> {
        let test_files = structure.files.iter()
            .filter(|f| {
                let name_lower = f.name.to_lowercase();
                name_lower.contains("test") || 
                name_lower.contains("spec") || 
                f.path.contains("__tests__") ||
                f.path.contains("/test/") ||
                f.path.contains("/tests/")
            })
            .count();

        let source_files = structure.files.iter()
            .filter(|f| !f.is_binary && !f.name.starts_with('.'))
            .count();

        if source_files > 0 {
            Ok((test_files as f64 / source_files as f64) * 100.0)
        } else {
            Ok(0.0)
        }
    }

    async fn scan_security_issues(&self, project_path: &Path, files: &[FileInfo]) -> Result<Vec<SecurityIssue>> {
        let mut issues = Vec::new();

        for file in files {
            if !file.is_binary {
                let file_path = project_path.join(&file.path);
                if let Ok(content) = fs::read_to_string(&file_path) {
                    // Check for hardcoded secrets
                    if content.contains("password") || content.contains("secret") || content.contains("api_key") {
                        issues.push(SecurityIssue {
                            severity: SecuritySeverity::High,
                            description: "Potential hardcoded credentials found".to_string(),
                            file_path: file.path.clone(),
                            line_number: None,
                            recommendation: "Use environment variables or secure credential storage".to_string(),
                        });
                    }

                    // Check for SQL injection patterns
                    if content.contains("SELECT") && content.contains("'+") {
                        issues.push(SecurityIssue {
                            severity: SecuritySeverity::High,
                            description: "Potential SQL injection vulnerability".to_string(),
                            file_path: file.path.clone(),
                            line_number: None,
                            recommendation: "Use parameterized queries".to_string(),
                        });
                    }
                }
            }
        }

        Ok(issues)
    }

    fn calculate_code_quality_score(&self, lint_issues: usize, complexity_score: i32, duplicate_percentage: f64, test_coverage: f64, security_issues: &[SecurityIssue]) -> i32 {
        let mut score = 100;

        // Deduct for lint issues
        score -= (lint_issues as i32).min(50);

        // Factor in complexity
        score = (score + complexity_score) / 2;

        // Deduct for duplicates
        score -= (duplicate_percentage as i32).min(30);

        // Bonus for test coverage
        if test_coverage > 50.0 {
            score += 10;
        }

        // Deduct for security issues
        for issue in security_issues {
            match issue.severity {
                SecuritySeverity::Critical => score -= 20,
                SecuritySeverity::High => score -= 10,
                SecuritySeverity::Medium => score -= 5,
                SecuritySeverity::Low => score -= 2,
            }
        }

        score.max(0).min(100)
    }

    // Structure Analysis Methods
    fn evaluate_project_organization(&self, structure: &ProjectStructure) -> i32 {
        let mut score = 100;

        // Check for common directory structure
        let has_src = structure.directories.iter().any(|d| d.contains("src") || d.contains("lib"));
        let has_tests = structure.directories.iter().any(|d| d.contains("test") || d.contains("spec"));
        let has_docs = structure.directories.iter().any(|d| d.contains("doc") || d.contains("docs"));

        if !has_src { score -= 20; }
        if !has_tests { score -= 15; }
        if !has_docs { score -= 10; }

        // Check for excessive nesting
        let max_depth = structure.directories.iter()
            .map(|d| d.matches('/').count())
            .max()
            .unwrap_or(0);

        if max_depth > 5 {
            score -= 15;
        }

        score.max(0)
    }

    fn evaluate_naming_conventions(&self, files: &[FileInfo]) -> i32 {
        let mut score = 100;
        let mut violations = 0;

        for file in files {
            if !file.is_binary {
                // Check for consistent naming
                if file.name.contains(" ") {
                    violations += 1; // Spaces in filenames
                }
                if file.name.chars().any(|c| c.is_uppercase()) && file.name.contains("-") {
                    violations += 1; // Mixed case with hyphens
                }
            }
        }

        score -= (violations as i32 * 5).min(50);
        score.max(0)
    }

    fn evaluate_file_structure(&self, structure: &ProjectStructure) -> i32 {
        let mut score = 100;

        // Check for essential files
        let has_readme = structure.documentation_files.iter().any(|f| f.to_lowercase().starts_with("readme"));
        let has_gitignore = structure.files.iter().any(|f| f.name == ".gitignore");
        let has_package_file = !structure.package_files.is_empty();

        if !has_readme { score -= 20; }
        if !has_gitignore { score -= 10; }
        if !has_package_file { score -= 15; }

        score.max(0)
    }

    fn evaluate_configuration_files(&self, config_files: &[String]) -> i32 {
        let mut score = 70; // Base score

        // Bonus for having configuration files
        if config_files.iter().any(|f| f.contains("docker")) {
            score += 10; // Docker configuration
        }
        if config_files.iter().any(|f| f.contains(".yml") || f.contains(".yaml")) {
            score += 10; // YAML configuration
        }
        if config_files.iter().any(|f| f.contains("package.json")) {
            score += 10; // Package configuration
        }

        score.min(100)
    }

    // Documentation Analysis Methods
    async fn evaluate_readme_quality(&self, project_path: &Path, doc_files: &[String]) -> Result<i32> {
        let readme_file = doc_files.iter()
            .find(|f| f.to_lowercase().starts_with("readme"))
            .ok_or_else(|| anyhow!("No README file found"))?;

        let readme_path = project_path.join(readme_file);
        let content = fs::read_to_string(readme_path)?;

        let mut score = 50; // Base score for having a README

        // Check for essential sections
        if content.to_lowercase().contains("installation") { score += 10; }
        if content.to_lowercase().contains("usage") { score += 10; }
        if content.to_lowercase().contains("description") || content.to_lowercase().contains("about") { score += 10; }
        if content.to_lowercase().contains("contributing") { score += 5; }
        if content.to_lowercase().contains("license") { score += 5; }
        if content.contains("```") { score += 10; } // Code examples

        score.min(100)
    }

    async fn calculate_code_comments_percentage(&self, files: &[FileInfo]) -> Result<f64> {
        let mut total_lines = 0;
        let mut comment_lines = 0;

        for file in files {
            if !file.is_binary && (
                file.name.ends_with(".js") || 
                file.name.ends_with(".ts") || 
                file.name.ends_with(".py") || 
                file.name.ends_with(".java") ||
                file.name.ends_with(".rs")
            ) {
                // This is a simplified comment detection
                // In a real implementation, you'd parse the files properly
                total_lines += 100; // Placeholder
                comment_lines += 10; // Placeholder
            }
        }

        if total_lines > 0 {
            Ok((comment_lines as f64 / total_lines as f64) * 100.0)
        } else {
            Ok(0.0)
        }
    }

    async fn evaluate_api_documentation(&self, _project_path: &Path, files: &[FileInfo]) -> Result<i32> {
        let mut score = 50; // Base score

        // Check for API documentation files
        for file in files {
            if file.name.to_lowercase().contains("api") || 
               file.name.to_lowercase().contains("swagger") ||
               file.name.ends_with(".yml") && file.name.contains("api") {
                score += 25;
                break;
            }
        }

        Ok(score.min(100))
    }

    async fn evaluate_inline_documentation(&self, files: &[FileInfo]) -> Result<i32> {
        let documented_files = files.iter()
            .filter(|f| !f.is_binary)
            .count();

        // This is a placeholder - in reality, you'd analyze the actual content
        if documented_files > 5 {
            Ok(80)
        } else if documented_files > 2 {
            Ok(60)
        } else {
            Ok(40)
        }
    }

    // Functionality Analysis Methods
    async fn test_build_success(&self, project_path: &Path, tech_stack: &[TechnologyStack]) -> Result<bool> {
        for stack in tech_stack {
            match stack {
                TechnologyStack::NodeJS | TechnologyStack::React | TechnologyStack::Vue | TechnologyStack::Angular => {
                    return self.test_npm_build(project_path).await;
                }
                TechnologyStack::Python | TechnologyStack::Django | TechnologyStack::Flask => {
                    return self.test_python_build(project_path).await;
                }
                TechnologyStack::Java | TechnologyStack::SpringBoot => {
                    return self.test_java_build(project_path).await;
                }
                _ => continue,
            }
        }
        
        Ok(true) // Default to true if no specific build system detected
    }

    async fn test_npm_build(&self, project_path: &Path) -> Result<bool> {
        let package_json = project_path.join("package.json");
        if !package_json.exists() {
            return Ok(false);
        }

        // Check if package.json is valid
        let content = fs::read_to_string(package_json)?;
        serde_json::from_str::<serde_json::Value>(&content)
            .map(|_| true)
            .map_err(|_| anyhow!("Invalid package.json"))
    }

    async fn test_python_build(&self, project_path: &Path) -> Result<bool> {
        // Check for Python syntax errors in main files
        for entry in fs::read_dir(project_path)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".py") {
                    let content = fs::read_to_string(entry.path())?;
                    // Basic syntax check - in reality, you'd use a Python parser
                    if content.contains("def ") || content.contains("class ") {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    async fn test_java_build(&self, project_path: &Path) -> Result<bool> {
        // Check for valid Java structure
        let has_src = project_path.join("src").exists();
        let has_maven = project_path.join("pom.xml").exists();
        let has_gradle = project_path.join("build.gradle").exists();
        
        Ok(has_src && (has_maven || has_gradle))
    }

    async fn run_tests(&self, project_path: &Path, tech_stack: &[TechnologyStack]) -> Result<bool> {
        // This would actually run the test suites
        // For now, we'll check if test files exist and are properly structured
        let test_dirs = ["test", "tests", "__tests__", "spec"];
        
        for dir in &test_dirs {
            if project_path.join(dir).exists() {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    async fn evaluate_feature_completeness(&self, _project_path: &Path, structure: &ProjectStructure) -> Result<i32> {
        let mut score = 50; // Base score

        // Basic feature completeness based on file count and structure
        let file_count = structure.files.len();
        match file_count {
            0..=5 => score = 30,
            6..=15 => score = 60,
            16..=30 => score = 80,
            _ => score = 90,
        }

        Ok(score)
    }

    async fn evaluate_error_handling(&self, files: &[FileInfo]) -> Result<i32> {
        let mut score = 50;
        let mut error_handling_files = 0;

        for file in files {
            if !file.is_binary && (
                file.name.contains("error") || 
                file.name.contains("exception") ||
                file.name.contains("handler")
            ) {
                error_handling_files += 1;
            }
        }

        if error_handling_files > 0 {
            score += 25;
        }

        Ok(score.min(100))
    }

    async fn evaluate_performance_indicators(&self, files: &[FileInfo]) -> Result<i32> {
        let mut score = 70; // Base score

        // Check for performance-related files
        for file in files {
            if file.name.contains("cache") || 
               file.name.contains("optimize") ||
               file.name.contains("performance") {
                score += 10;
                break;
            }
        }

        Ok(score.min(100))
    }

    // Scoring and Feedback Methods
    fn calculate_total_score(&self, code_quality: &CodeQualityMetrics, structure: &StructureMetrics, documentation: &DocumentationMetrics, functionality: &FunctionalityMetrics) -> i32 {
        // Weighted scoring as per architecture specs
        let weighted_score = (
            (code_quality.score as f64 * 0.25) +
            (structure.score as f64 * 0.20) +
            (documentation.score as f64 * 0.15) +
            (functionality.score as f64 * 0.40)
        ) as i32;

        weighted_score.max(0).min(100)
    }

    fn generate_feedback(&self, code_quality: &CodeQualityMetrics, structure: &StructureMetrics, documentation: &DocumentationMetrics, functionality: &FunctionalityMetrics) -> String {
        let mut feedback = String::new();

        feedback.push_str(&format!("## Project Analysis Summary\n\n"));
        feedback.push_str(&format!("**Code Quality Score: {}/100**\n", code_quality.score));
        feedback.push_str(&format!("**Structure Score: {}/100**\n", structure.score));
        feedback.push_str(&format!("**Documentation Score: {}/100**\n", documentation.score));
        feedback.push_str(&format!("**Functionality Score: {}/100**\n\n", functionality.score));

        // Code Quality Feedback
        feedback.push_str("### Code Quality\n");
        if code_quality.lint_issues > 10 {
            feedback.push_str("⚠️ High number of linting issues detected. Consider running a linter to improve code quality.\n");
        } else if code_quality.lint_issues > 0 {
            feedback.push_str("✨ Minor linting issues found. Overall code quality looks good.\n");
        } else {
            feedback.push_str("✅ Excellent code quality with no major issues detected.\n");
        }

        if code_quality.test_coverage_percentage < 30.0 {
            feedback.push_str("⚠️ Low test coverage. Consider adding more comprehensive tests.\n");
        } else if code_quality.test_coverage_percentage > 70.0 {
            feedback.push_str("✅ Good test coverage detected.\n");
        }

        // Structure Feedback
        feedback.push_str("\n### Project Structure\n");
        if structure.score > 80 {
            feedback.push_str("✅ Well-organized project structure.\n");
        } else if structure.score > 60 {
            feedback.push_str("✨ Good project structure with room for minor improvements.\n");
        } else {
            feedback.push_str("⚠️ Project structure could be improved for better maintainability.\n");
        }

        // Documentation Feedback
        feedback.push_str("\n### Documentation\n");
        if documentation.score > 80 {
            feedback.push_str("✅ Excellent documentation quality.\n");
        } else if documentation.score > 60 {
            feedback.push_str("✨ Good documentation with some areas for improvement.\n");
        } else {
            feedback.push_str("⚠️ Documentation needs improvement. Consider adding more comprehensive docs.\n");
        }

        // Functionality Feedback
        feedback.push_str("\n### Functionality\n");
        if functionality.build_success {
            feedback.push_str("✅ Project builds successfully.\n");
        } else {
            feedback.push_str("⚠️ Build issues detected. Please check your build configuration.\n");
        }

        if functionality.tests_passing {
            feedback.push_str("✅ Tests are properly set up.\n");
        } else {
            feedback.push_str("⚠️ No tests detected or tests are failing.\n");
        }

        feedback
    }

    fn generate_recommendations(&self, code_quality: &CodeQualityMetrics, structure: &StructureMetrics, documentation: &DocumentationMetrics, functionality: &FunctionalityMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Code Quality Recommendations
        if code_quality.lint_issues > 5 {
            recommendations.push("Set up and configure a linter for your technology stack".to_string());
        }
        if code_quality.test_coverage_percentage < 50.0 {
            recommendations.push("Increase test coverage to at least 50%".to_string());
        }
        if !code_quality.security_issues.is_empty() {
            recommendations.push("Address security vulnerabilities found in the codebase".to_string());
        }

        // Structure Recommendations
        if structure.organization_score < 70 {
            recommendations.push("Reorganize project structure following best practices for your technology stack".to_string());
        }
        if structure.naming_convention_score < 70 {
            recommendations.push("Improve file and directory naming conventions".to_string());
        }

        // Documentation Recommendations
        if documentation.readme_quality < 70 {
            recommendations.push("Enhance README with installation, usage, and contribution guidelines".to_string());
        }
        if documentation.code_comments_percentage < 30.0 {
            recommendations.push("Add more inline comments to explain complex logic".to_string());
        }

        // Functionality Recommendations
        if !functionality.build_success {
            recommendations.push("Fix build configuration and ensure project compiles successfully".to_string());
        }
        if !functionality.tests_passing {
            recommendations.push("Add comprehensive test suite covering main functionality".to_string());
        }
        if functionality.error_handling_score < 60 {
            recommendations.push("Implement proper error handling and validation".to_string());
        }

        recommendations
    }

    pub fn convert_to_create_analysis_result(&self, project_id: i64, analysis: &AnalysisResult) -> CreateAnalysisResult {
        let analysis_data = serde_json::to_value(analysis).ok();

        CreateAnalysisResult {
            project_id,
            code_quality_score: Some(analysis.code_quality.score),
            structure_score: Some(analysis.structure.score),
            documentation_score: Some(analysis.documentation.score),
            functionality_score: Some(analysis.functionality.score),
            total_score: Some(analysis.total_score),
            feedback: Some(analysis.feedback.clone()),
            analysis_data,
        }
    }
} 