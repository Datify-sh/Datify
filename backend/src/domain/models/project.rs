use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub settings: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    pub fn settings_json(&self) -> JsonValue {
        self.settings
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(JsonValue::Object(serde_json::Map::new()))
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub settings: Option<JsonValue>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<JsonValue>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub settings: JsonValue,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Project> for ProjectResponse {
    fn from(project: Project) -> Self {
        let settings = project.settings_json();
        Self {
            id: project.id,
            name: project.name,
            slug: project.slug,
            description: project.description,
            settings,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectWithStats {
    #[serde(flatten)]
    pub project: ProjectResponse,
    pub database_count: i64,
}
