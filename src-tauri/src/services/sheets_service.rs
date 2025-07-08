use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::services::AuthService;
use crate::database::models::{CreateStudent, CreateProject, Student, Project};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentData {
    pub name: String,
    pub email: Option<String>,
    pub github_username: Option<String>,
    pub github_url: Option<String>,
    pub project_name: Option<String>,
    pub project_description: Option<String>,
    pub cohort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub students_imported: usize,
    pub projects_imported: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetMapping {
    pub name_column: String,
    pub email_column: Option<String>,
    pub github_username_column: Option<String>,
    pub github_url_column: Option<String>,
    pub project_name_column: Option<String>,
    pub project_description_column: Option<String>,
    pub cohort_column: Option<String>,
}

impl Default for SheetMapping {
    fn default() -> Self {
        Self {
            name_column: "Name".to_string(),
            email_column: Some("Email".to_string()),
            github_username_column: Some("GitHub Username".to_string()),
            github_url_column: Some("GitHub URL".to_string()),
            project_name_column: Some("Project Name".to_string()),
            project_description_column: Some("Project Description".to_string()),
            cohort_column: Some("Cohort".to_string()),
        }
    }
}

pub struct SheetsService {
    auth_service: AuthService,
    client: Option<reqwest::Client>,
}

impl SheetsService {
    pub fn new(auth_service: AuthService) -> Self {
        Self {
            auth_service,
            client: Some(reqwest::Client::new()),
        }
    }

    pub async fn get_sheet_data(&self, spreadsheet_id: &str, range: &str) -> Result<SheetData> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("HTTP client not initialized"))?;

        let credentials = self.auth_service.get_stored_credentials()?;
        let access_token = credentials.google_access_token
            .ok_or_else(|| anyhow!("No Google access token available"))?;

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            spreadsheet_id, range
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            if response.status() == 401 {
                // Try to refresh the token
                let new_token = self.auth_service.refresh_google_token().await?;
                return self.get_sheet_data_with_token(spreadsheet_id, range, &new_token).await;
            } else {
                return Err(anyhow!("Failed to fetch sheet data: {}", response.status()));
            }
        }

        let response_data: serde_json::Value = response.json().await?;
        let values = response_data["values"]
            .as_array()
            .ok_or_else(|| anyhow!("No values found in sheet response"))?;

        if values.is_empty() {
            return Err(anyhow!("Sheet is empty"));
        }

        // First row is headers
        let headers: Vec<String> = values[0]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid header row"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        // Remaining rows are data
        let rows: Vec<Vec<String>> = values[1..]
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .collect();

        Ok(SheetData { headers, rows })
    }

    async fn get_sheet_data_with_token(&self, spreadsheet_id: &str, range: &str, token: &str) -> Result<SheetData> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("HTTP client not initialized"))?;

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            spreadsheet_id, range
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch sheet data: {}", response.status()));
        }

        let response_data: serde_json::Value = response.json().await?;
        let values = response_data["values"]
            .as_array()
            .ok_or_else(|| anyhow!("No values found in sheet response"))?;

        if values.is_empty() {
            return Err(anyhow!("Sheet is empty"));
        }

        let headers: Vec<String> = values[0]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid header row"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();

        let rows: Vec<Vec<String>> = values[1..]
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .collect();

        Ok(SheetData { headers, rows })
    }

    pub fn parse_student_data(&self, sheet_data: &SheetData, mapping: &SheetMapping) -> Result<Vec<StudentData>> {
        let header_indices = self.build_header_indices(&sheet_data.headers, mapping)?;
        let mut students = Vec::new();

        for (row_index, row) in sheet_data.rows.iter().enumerate() {
            // Skip empty rows
            if row.iter().all(|cell| cell.trim().is_empty()) {
                continue;
            }

            let name = self.get_cell_value(row, header_indices.get("name"))
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| anyhow!("Missing name in row {}", row_index + 2))?;

            let email = self.get_cell_value(row, header_indices.get("email"))
                .filter(|s| !s.trim().is_empty());

            let github_username = self.get_cell_value(row, header_indices.get("github_username"))
                .filter(|s| !s.trim().is_empty());

            let github_url = self.get_cell_value(row, header_indices.get("github_url"))
                .filter(|s| !s.trim().is_empty());

            let project_name = self.get_cell_value(row, header_indices.get("project_name"))
                .filter(|s| !s.trim().is_empty());

            let project_description = self.get_cell_value(row, header_indices.get("project_description"))
                .filter(|s| !s.trim().is_empty());

            let cohort = self.get_cell_value(row, header_indices.get("cohort"))
                .filter(|s| !s.trim().is_empty());

            students.push(StudentData {
                name,
                email,
                github_username,
                github_url,
                project_name,
                project_description,
                cohort,
            });
        }

        Ok(students)
    }

    pub fn validate_student_data(&self, students: &[StudentData]) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        for (index, student) in students.iter().enumerate() {
            let row_num = index + 2; // +2 because we start from row 2 (after headers)

            // Validate name
            if student.name.trim().is_empty() {
                errors.push(format!("Row {}: Name is required", row_num));
            }

            // Validate email format if provided
            if let Some(email) = &student.email {
                if !email.trim().is_empty() && !self.is_valid_email(email) {
                    errors.push(format!("Row {}: Invalid email format", row_num));
                }
            }

            // Validate GitHub URL format if provided
            if let Some(github_url) = &student.github_url {
                if !github_url.trim().is_empty() && !self.is_valid_github_url(github_url) {
                    errors.push(format!("Row {}: Invalid GitHub URL format", row_num));
                }
            }

            // Check if either GitHub username or URL is provided
            if student.github_username.is_none() && student.github_url.is_none() {
                errors.push(format!("Row {}: Either GitHub username or GitHub URL is required", row_num));
            }
        }

        Ok(errors)
    }

    pub fn convert_to_create_students(&self, students: &[StudentData]) -> Vec<CreateStudent> {
        students
            .iter()
            .map(|student| CreateStudent {
                name: student.name.clone(),
                email: student.email.clone(),
                github_username: student.github_username.clone(),
                cohort: student.cohort.clone(),
            })
            .collect()
    }

    pub fn convert_to_create_projects(&self, students: &[StudentData], student_ids: &HashMap<String, i64>) -> Vec<CreateProject> {
        let mut projects = Vec::new();

        for student in students {
            if let (Some(project_name), Some(&student_id)) = (&student.project_name, student_ids.get(&student.name)) {
                let github_url = student.github_url.as_ref()
                    .or_else(|| {
                        student.github_username.as_ref().map(|username| {
                            format!("https://github.com/{}/{}", username, project_name)
                        })
                    });

                if let Some(url) = github_url {
                    projects.push(CreateProject {
                        student_id,
                        name: project_name.clone(),
                        description: student.project_description.clone(),
                        github_url: url,
                        technology_stack: None, // Will be detected later
                    });
                }
            }
        }

        projects
    }

    pub async fn export_results_to_sheet(
        &self,
        spreadsheet_id: &str,
        range: &str,
        results: &[ExportRow],
    ) -> Result<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| anyhow!("HTTP client not initialized"))?;

        let credentials = self.auth_service.get_stored_credentials()?;
        let access_token = credentials.google_access_token
            .ok_or_else(|| anyhow!("No Google access token available"))?;

        let values: Vec<Vec<String>> = results
            .iter()
            .map(|row| vec![
                row.student_name.clone(),
                row.project_name.clone(),
                row.total_score.map(|s| s.to_string()).unwrap_or_default(),
                row.code_quality_score.map(|s| s.to_string()).unwrap_or_default(),
                row.structure_score.map(|s| s.to_string()).unwrap_or_default(),
                row.documentation_score.map(|s| s.to_string()).unwrap_or_default(),
                row.functionality_score.map(|s| s.to_string()).unwrap_or_default(),
                row.feedback.clone().unwrap_or_default(),
            ])
            .collect();

        let update_data = serde_json::json!({
            "values": values
        });

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            spreadsheet_id, range
        );

        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&update_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to export results: {}", response.status()));
        }

        Ok(())
    }

    fn build_header_indices(&self, headers: &[String], mapping: &SheetMapping) -> Result<HashMap<String, usize>> {
        let mut indices = HashMap::new();

        // Required field
        let name_index = self.find_header_index(headers, &mapping.name_column)
            .ok_or_else(|| anyhow!("Name column '{}' not found", mapping.name_column))?;
        indices.insert("name".to_string(), name_index);

        // Optional fields
        if let Some(email_col) = &mapping.email_column {
            if let Some(index) = self.find_header_index(headers, email_col) {
                indices.insert("email".to_string(), index);
            }
        }

        if let Some(github_username_col) = &mapping.github_username_column {
            if let Some(index) = self.find_header_index(headers, github_username_col) {
                indices.insert("github_username".to_string(), index);
            }
        }

        if let Some(github_url_col) = &mapping.github_url_column {
            if let Some(index) = self.find_header_index(headers, github_url_col) {
                indices.insert("github_url".to_string(), index);
            }
        }

        if let Some(project_name_col) = &mapping.project_name_column {
            if let Some(index) = self.find_header_index(headers, project_name_col) {
                indices.insert("project_name".to_string(), index);
            }
        }

        if let Some(project_desc_col) = &mapping.project_description_column {
            if let Some(index) = self.find_header_index(headers, project_desc_col) {
                indices.insert("project_description".to_string(), index);
            }
        }

        if let Some(cohort_col) = &mapping.cohort_column {
            if let Some(index) = self.find_header_index(headers, cohort_col) {
                indices.insert("cohort".to_string(), index);
            }
        }

        Ok(indices)
    }

    fn find_header_index(&self, headers: &[String], target: &str) -> Option<usize> {
        headers.iter().position(|h| h.trim().eq_ignore_ascii_case(target.trim()))
    }

    fn get_cell_value(&self, row: &[String], index: Option<&usize>) -> Option<String> {
        index.and_then(|&i| row.get(i).map(|s| s.trim().to_string()))
    }

    fn is_valid_email(&self, email: &str) -> bool {
        regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .unwrap()
            .is_match(email)
    }

    fn is_valid_github_url(&self, url: &str) -> bool {
        regex::Regex::new(r"^https://github\.com/[^/]+/[^/]+/?$")
            .unwrap()
            .is_match(url)
    }

    pub fn extract_spreadsheet_id(&self, url: &str) -> Option<String> {
        regex::Regex::new(r"/spreadsheets/d/([a-zA-Z0-9-_]+)")
            .unwrap()
            .captures(url)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRow {
    pub student_name: String,
    pub project_name: String,
    pub total_score: Option<i32>,
    pub code_quality_score: Option<i32>,
    pub structure_score: Option<i32>,
    pub documentation_score: Option<i32>,
    pub functionality_score: Option<i32>,
    pub feedback: Option<String>,
} 