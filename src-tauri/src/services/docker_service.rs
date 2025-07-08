use anyhow::{Result, anyhow};
use bollard::{
    Docker,
    container::{
        Config, CreateContainerOptions, StartContainerOptions, StopContainerOptions,
        RemoveContainerOptions, ListContainersOptions, WaitContainerOptions,
    },
    image::{CreateImageOptions, ListImagesOptions},
    models::{ContainerSummary, HostConfig, PortBinding, ExposedPorts},
    network::{CreateNetworkOptions},
    volume::{CreateVolumeOptions},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::database::models::{TechnologyStack, CreatePlaygroundSession, PlaygroundSession};
use futures::stream::TryStreamExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundInfo {
    pub container_id: String,
    pub port: u16,
    pub url: String,
    pub status: PlaygroundStatus,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percentage: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub network_rx: u64,
    pub network_tx: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlaygroundStatus {
    Starting,
    Running,
    Stopped,
    Error,
    Building,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub image: String,
    pub dockerfile_content: Option<String>,
    pub port: u16,
    pub setup_commands: Vec<String>,
    pub start_command: String,
    pub health_check_path: String,
    pub working_dir: String,
}

pub struct DockerService {
    docker: Docker,
    network_name: String,
}

impl DockerService {
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        
        // Test Docker connection
        docker.ping().await?;
        
        let service = Self {
            docker,
            network_name: "r3viewer-network".to_string(),
        };
        
        // Initialize Docker environment
        service.initialize().await?;
        
        Ok(service)
    }

    async fn initialize(&self) -> Result<()> {
        // Create network if it doesn't exist
        self.ensure_network_exists().await?;
        
        // Pull base images
        self.pull_base_images().await?;
        
        Ok(())
    }

    pub async fn start_playground(&self, project_path: &Path, tech_stack: &[TechnologyStack]) -> Result<PlaygroundInfo> {
        let project_name = project_path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Invalid project path"))?;

        // Detect environment configuration
        let env_config = self.detect_environment_config(project_path, tech_stack).await?;
        
        // Find available port
        let port = self.find_available_port().await?;
        
        // Create container
        let container_id = self.create_container(project_name, project_path, &env_config, port).await?;
        
        // Start container
        self.docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await?;

        // Run setup commands
        for command in &env_config.setup_commands {
            self.execute_command(&container_id, command).await?;
        }

        // Wait for service to be ready
        self.wait_for_service_ready(&container_id, &env_config).await?;

        let url = format!("http://localhost:{}", port);
        
        Ok(PlaygroundInfo {
            container_id,
            port,
            url,
            status: PlaygroundStatus::Running,
            resource_usage: self.get_resource_usage(&container_id).await?,
        })
    }

    pub async fn stop_playground(&self, container_id: &str) -> Result<()> {
        // Stop container
        self.docker
            .stop_container(container_id, Some(StopContainerOptions { t: 10 }))
            .await?;

        // Remove container
        self.docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    v: true, // Remove associated volumes
                    ..Default::default()
                }),
            )
            .await?;

        Ok(())
    }

    pub async fn get_playground_status(&self, container_id: &str) -> Result<PlaygroundStatus> {
        let containers = self.docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                filters: {
                    let mut filters = HashMap::new();
                    filters.insert("id".to_string(), vec![container_id.to_string()]);
                    filters
                },
                ..Default::default()
            }))
            .await?;

        if let Some(container) = containers.first() {
            match container.state.as_deref() {
                Some("running") => Ok(PlaygroundStatus::Running),
                Some("exited") | Some("stopped") => Ok(PlaygroundStatus::Stopped),
                Some("created") => Ok(PlaygroundStatus::Starting),
                _ => Ok(PlaygroundStatus::Error),
            }
        } else {
            Ok(PlaygroundStatus::Error)
        }
    }

    pub async fn get_resource_usage(&self, container_id: &str) -> Result<ResourceUsage> {
        let stats = self.docker.stats(container_id, Some(false)).try_collect::<Vec<_>>().await?;
        
        if let Some(stat) = stats.first() {
            let cpu_percentage = self.calculate_cpu_percentage(stat)?;
            let memory_usage = stat.memory_stats.usage.unwrap_or(0);
            let memory_limit = stat.memory_stats.limit.unwrap_or(0);
            
            let (network_rx, network_tx) = stat.networks.as_ref()
                .and_then(|nets| nets.get("eth0"))
                .map(|net| (net.rx_bytes, net.tx_bytes))
                .unwrap_or((0, 0));

            Ok(ResourceUsage {
                cpu_percentage,
                memory_usage,
                memory_limit,
                network_rx,
                network_tx,
            })
        } else {
            Err(anyhow!("No stats available for container"))
        }
    }

    pub async fn list_active_playgrounds(&self) -> Result<Vec<ContainerSummary>> {
        let containers = self.docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: false,
                filters: {
                    let mut filters = HashMap::new();
                    filters.insert("label".to_string(), vec!["r3viewer.playground=true".to_string()]);
                    filters
                },
                ..Default::default()
            }))
            .await?;

        Ok(containers)
    }

    pub async fn cleanup_old_containers(&self, max_age_hours: u64) -> Result<usize> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs() - (max_age_hours * 3600);

        let containers = self.docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                filters: {
                    let mut filters = HashMap::new();
                    filters.insert("label".to_string(), vec!["r3viewer.playground=true".to_string()]);
                    filters
                },
                ..Default::default()
            }))
            .await?;

        let mut cleaned_count = 0;

        for container in containers {
            if let Some(created) = container.created {
                if (created as u64) < cutoff_time {
                    if let Some(id) = &container.id {
                        let _ = self.stop_playground(id).await;
                        cleaned_count += 1;
                    }
                }
            }
        }

        Ok(cleaned_count)
    }

    async fn detect_environment_config(&self, project_path: &Path, tech_stack: &[TechnologyStack]) -> Result<EnvironmentConfig> {
        // Check for Dockerfile first
        let dockerfile_path = project_path.join("Dockerfile");
        if dockerfile_path.exists() {
            return self.create_custom_dockerfile_config(project_path).await;
        }

        // Use predefined configurations based on tech stack
        for stack in tech_stack {
            match stack {
                TechnologyStack::NodeJS | TechnologyStack::React | TechnologyStack::Vue | TechnologyStack::Angular => {
                    return self.create_nodejs_config(project_path).await;
                }
                TechnologyStack::Python | TechnologyStack::Django | TechnologyStack::Flask => {
                    return self.create_python_config(project_path).await;
                }
                TechnologyStack::Java | TechnologyStack::SpringBoot => {
                    return self.create_java_config(project_path).await;
                }
                TechnologyStack::Rust => {
                    return self.create_rust_config(project_path).await;
                }
                TechnologyStack::Go => {
                    return self.create_go_config(project_path).await;
                }
                TechnologyStack::PHP => {
                    return self.create_php_config(project_path).await;
                }
                TechnologyStack::Ruby => {
                    return self.create_ruby_config(project_path).await;
                }
                _ => continue,
            }
        }

        // Default to generic configuration
        self.create_generic_config(project_path).await
    }

    async fn create_nodejs_config(&self, project_path: &Path) -> Result<EnvironmentConfig> {
        let package_json_path = project_path.join("package.json");
        let mut setup_commands = vec![
            "npm install".to_string(),
        ];

        let start_command = if package_json_path.exists() {
            let content = std::fs::read_to_string(&package_json_path)?;
            let package: serde_json::Value = serde_json::from_str(&content)?;
            
            if package["scripts"]["dev"].is_string() {
                "npm run dev".to_string()
            } else if package["scripts"]["start"].is_string() {
                "npm start".to_string()
            } else {
                "node index.js".to_string()
            }
        } else {
            "node index.js".to_string()
        };

        // Check if it's a React/Vue/Angular app
        if package_json_path.exists() {
            let content = std::fs::read_to_string(&package_json_path)?;
            if content.contains("\"react\"") || content.contains("\"vue\"") || content.contains("\"@angular/core\"") {
                setup_commands.push("npm run build".to_string());
            }
        }

        Ok(EnvironmentConfig {
            image: "node:18-alpine".to_string(),
            dockerfile_content: None,
            port: 3000,
            setup_commands,
            start_command,
            health_check_path: "/".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_python_config(&self, project_path: &Path) -> Result<EnvironmentConfig> {
        let requirements_path = project_path.join("requirements.txt");
        let mut setup_commands = vec![];

        if requirements_path.exists() {
            setup_commands.push("pip install -r requirements.txt".to_string());
        }

        let start_command = if project_path.join("manage.py").exists() {
            // Django project
            setup_commands.push("python manage.py migrate".to_string());
            "python manage.py runserver 0.0.0.0:8000".to_string()
        } else if project_path.join("app.py").exists() {
            // Flask project
            "python app.py".to_string()
        } else if project_path.join("main.py").exists() {
            "python main.py".to_string()
        } else {
            "python app.py".to_string()
        };

        Ok(EnvironmentConfig {
            image: "python:3.11-slim".to_string(),
            dockerfile_content: None,
            port: 8000,
            setup_commands,
            start_command,
            health_check_path: "/".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_java_config(&self, project_path: &Path) -> Result<EnvironmentConfig> {
        let mut setup_commands = vec![];
        let start_command = if project_path.join("pom.xml").exists() {
            // Maven project
            setup_commands.push("mvn clean install -DskipTests".to_string());
            "mvn spring-boot:run".to_string()
        } else if project_path.join("build.gradle").exists() {
            // Gradle project
            setup_commands.push("./gradlew build -x test".to_string());
            "./gradlew bootRun".to_string()
        } else {
            "java -jar app.jar".to_string()
        };

        Ok(EnvironmentConfig {
            image: "openjdk:17-slim".to_string(),
            dockerfile_content: None,
            port: 8080,
            setup_commands,
            start_command,
            health_check_path: "/actuator/health".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_rust_config(&self, _project_path: &Path) -> Result<EnvironmentConfig> {
        Ok(EnvironmentConfig {
            image: "rust:1.70".to_string(),
            dockerfile_content: None,
            port: 8000,
            setup_commands: vec!["cargo build --release".to_string()],
            start_command: "cargo run --release".to_string(),
            health_check_path: "/".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_go_config(&self, _project_path: &Path) -> Result<EnvironmentConfig> {
        Ok(EnvironmentConfig {
            image: "golang:1.21-alpine".to_string(),
            dockerfile_content: None,
            port: 8080,
            setup_commands: vec!["go mod download".to_string(), "go build -o main .".to_string()],
            start_command: "./main".to_string(),
            health_check_path: "/".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_php_config(&self, _project_path: &Path) -> Result<EnvironmentConfig> {
        Ok(EnvironmentConfig {
            image: "php:8.2-apache".to_string(),
            dockerfile_content: None,
            port: 80,
            setup_commands: vec!["composer install".to_string()],
            start_command: "apache2-foreground".to_string(),
            health_check_path: "/".to_string(),
            working_dir: "/var/www/html".to_string(),
        })
    }

    async fn create_ruby_config(&self, _project_path: &Path) -> Result<EnvironmentConfig> {
        Ok(EnvironmentConfig {
            image: "ruby:3.2".to_string(),
            dockerfile_content: None,
            port: 3000,
            setup_commands: vec!["bundle install".to_string()],
            start_command: "rails server -b 0.0.0.0".to_string(),
            health_check_path: "/".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_custom_dockerfile_config(&self, project_path: &Path) -> Result<EnvironmentConfig> {
        let dockerfile_content = std::fs::read_to_string(project_path.join("Dockerfile"))?;
        
        Ok(EnvironmentConfig {
            image: "".to_string(), // Will be built from Dockerfile
            dockerfile_content: Some(dockerfile_content),
            port: 8080, // Default port, might be overridden
            setup_commands: vec![],
            start_command: "".to_string(), // Will be defined in Dockerfile
            health_check_path: "/".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_generic_config(&self, _project_path: &Path) -> Result<EnvironmentConfig> {
        Ok(EnvironmentConfig {
            image: "alpine:latest".to_string(),
            dockerfile_content: None,
            port: 8080,
            setup_commands: vec![],
            start_command: "echo 'No start command configured'".to_string(),
            health_check_path: "/".to_string(),
            working_dir: "/app".to_string(),
        })
    }

    async fn create_container(&self, project_name: &str, project_path: &Path, config: &EnvironmentConfig, port: u16) -> Result<String> {
        let container_name = format!("r3viewer-{}-{}", project_name, port);
        
        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            format!("{}/tcp", config.port),
            Some(vec![PortBinding {
                host_ip: Some("127.0.0.1".to_string()),
                host_port: Some(port.to_string()),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(format!("{}/tcp", config.port), HashMap::new());

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            memory: Some(1_073_741_824), // 1GB memory limit
            cpu_shares: Some(1024),
            network_mode: Some(self.network_name.clone()),
            binds: Some(vec![format!("{}:{}", project_path.display(), config.working_dir)]),
            ..Default::default()
        };

        let mut labels = HashMap::new();
        labels.insert("r3viewer.playground".to_string(), "true".to_string());
        labels.insert("r3viewer.project".to_string(), project_name.to_string());

        let container_config = Config {
            image: Some(config.image.clone()),
            working_dir: Some(config.working_dir.clone()),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            labels: Some(labels),
            env: Some(vec![
                "NODE_ENV=development".to_string(),
                "PORT=3000".to_string(),
            ]),
            ..Default::default()
        };

        let container = self.docker
            .create_container(
                Some(CreateContainerOptions { name: container_name }),
                container_config,
            )
            .await?;

        Ok(container.id)
    }

    async fn execute_command(&self, container_id: &str, command: &str) -> Result<()> {
        use bollard::exec::{CreateExecOptions, StartExecResults};
        
        let exec = self.docker
            .create_exec(
                container_id,
                CreateExecOptions {
                    cmd: Some(vec!["sh", "-c", command]),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await?;

        if let StartExecResults::Attached { output, .. } = self.docker.start_exec(&exec.id, None).await? {
            output.try_collect::<Vec<_>>().await?;
        }

        Ok(())
    }

    async fn wait_for_service_ready(&self, container_id: &str, config: &EnvironmentConfig) -> Result<()> {
        let max_attempts = 30;
        let mut attempts = 0;

        while attempts < max_attempts {
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            if let Ok(status) = self.get_playground_status(container_id).await {
                if matches!(status, PlaygroundStatus::Running) {
                    // Additional health check if specified
                    if !config.health_check_path.is_empty() {
                        // Could implement HTTP health check here
                        return Ok(());
                    }
                    return Ok(());
                }
            }
            
            attempts += 1;
        }

        Err(anyhow!("Service failed to start within timeout"))
    }

    async fn find_available_port(&self) -> Result<u16> {
        use std::net::{TcpListener, SocketAddr};
        
        for port in 8000..9000 {
            if let Ok(addr) = format!("127.0.0.1:{}", port).parse::<SocketAddr>() {
                if TcpListener::bind(addr).is_ok() {
                    return Ok(port);
                }
            }
        }
        
        Err(anyhow!("No available ports found"))
    }

    async fn ensure_network_exists(&self) -> Result<()> {
        // Check if network exists
        let networks = self.docker.list_networks::<String>(None).await?;
        
        for network in networks {
            if network.name == Some(self.network_name.clone()) {
                return Ok(());
            }
        }

        // Create network
        self.docker
            .create_network(CreateNetworkOptions {
                name: self.network_name.clone(),
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    async fn pull_base_images(&self) -> Result<()> {
        let base_images = vec![
            "node:18-alpine",
            "python:3.11-slim",
            "openjdk:17-slim",
            "rust:1.70",
            "golang:1.21-alpine",
            "php:8.2-apache",
            "ruby:3.2",
            "alpine:latest",
        ];

        for image in base_images {
            let _ = self.docker
                .create_image(
                    Some(CreateImageOptions {
                        from_image: image,
                        ..Default::default()
                    }),
                    None,
                    None,
                )
                .try_collect::<Vec<_>>()
                .await;
        }

        Ok(())
    }

    fn calculate_cpu_percentage(&self, stats: &bollard::models::Stats) -> Result<f64> {
        if let (Some(cpu_stats), Some(precpu_stats)) = (&stats.cpu_stats, &stats.precpu_stats) {
            let cpu_delta = cpu_stats.cpu_usage.total_usage as f64 - precpu_stats.cpu_usage.total_usage as f64;
            let system_delta = cpu_stats.system_cpu_usage.unwrap_or(0) as f64 - precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
            
            if system_delta > 0.0 && cpu_delta > 0.0 {
                let cpu_count = cpu_stats.cpu_usage.percpu_usage.as_ref().map(|v| v.len()).unwrap_or(1) as f64;
                return Ok((cpu_delta / system_delta) * cpu_count * 100.0);
            }
        }
        
        Ok(0.0)
    }
} 