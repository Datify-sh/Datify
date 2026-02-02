use slug::slugify;

use crate::domain::models::{Project, ProjectResponse, ProjectWithStats};
use crate::error::{AppError, AppResult};
use crate::repositories::{DatabaseRepository, ProjectRepository};

#[derive(Clone)]
pub struct ProjectService {
    project_repo: ProjectRepository,
    database_repo: DatabaseRepository,
}

impl ProjectService {
    pub fn new(project_repo: ProjectRepository, database_repo: DatabaseRepository) -> Self {
        Self {
            project_repo,
            database_repo,
        }
    }

    pub async fn create(
        &self,
        user_id: &str,
        name: &str,
        description: Option<&str>,
        settings: Option<&str>,
    ) -> AppResult<ProjectResponse> {
        if name.trim().is_empty() {
            return Err(AppError::Validation(
                "Project name cannot be empty".to_string(),
            ));
        }

        if name.len() > 100 {
            return Err(AppError::Validation(
                "Project name must be 100 characters or less".to_string(),
            ));
        }

        let base_slug = slugify(name);
        let mut slug = base_slug.clone();
        let mut counter = 1;

        while self.project_repo.find_by_slug(&slug).await?.is_some() {
            slug = format!("{}-{}", base_slug, counter);
            counter += 1;
        }

        let project = self
            .project_repo
            .create(user_id, name, &slug, description, settings)
            .await?;

        Ok(project.into())
    }

    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<Project>> {
        self.project_repo.find_by_id(id).await
    }

    pub async fn get_by_slug(&self, slug: &str) -> AppResult<Option<Project>> {
        self.project_repo.find_by_slug(slug).await
    }

    pub async fn get_by_id_with_stats(&self, id: &str) -> AppResult<Option<ProjectWithStats>> {
        let project = match self.project_repo.find_by_id(id).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let database_count = self.database_repo.count_by_project(id).await?;

        Ok(Some(ProjectWithStats {
            project: project.into(),
            database_count,
        }))
    }

    pub async fn list_by_user(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<ProjectResponse>> {
        let projects = self
            .project_repo
            .find_by_user_id(user_id, limit, offset)
            .await?;

        Ok(projects.into_iter().map(|p| p.into()).collect())
    }

    pub async fn list_all(&self, limit: i64, offset: i64) -> AppResult<Vec<ProjectResponse>> {
        let projects = self.project_repo.find_all(limit, offset).await?;
        Ok(projects.into_iter().map(|p| p.into()).collect())
    }

    pub async fn list_by_user_with_stats(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<ProjectWithStats>> {
        let projects = self
            .project_repo
            .find_by_user_id(user_id, limit, offset)
            .await?;

        let mut results = Vec::with_capacity(projects.len());

        for project in projects {
            let database_count = self.database_repo.count_by_project(&project.id).await?;

            results.push(ProjectWithStats {
                project: project.into(),
                database_count,
            });
        }

        Ok(results)
    }

    pub async fn list_all_with_stats(
        &self,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<ProjectWithStats>> {
        let projects = self.project_repo.find_all(limit, offset).await?;

        let mut results = Vec::with_capacity(projects.len());

        for project in projects {
            let database_count = self.database_repo.count_by_project(&project.id).await?;

            results.push(ProjectWithStats {
                project: project.into(),
                database_count,
            });
        }

        Ok(results)
    }

    pub async fn update(
        &self,
        id: &str,
        user_id: &str,
        is_admin: bool,
        name: Option<&str>,
        description: Option<&str>,
        settings: Option<&str>,
    ) -> AppResult<ProjectResponse> {
        if !is_admin && !self.project_repo.is_owner(id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err(AppError::Validation(
                    "Project name cannot be empty".to_string(),
                ));
            }
            if n.len() > 100 {
                return Err(AppError::Validation(
                    "Project name must be 100 characters or less".to_string(),
                ));
            }
        }

        let project = self
            .project_repo
            .update(id, name, description, settings)
            .await?;

        Ok(project.into())
    }

    pub async fn delete(&self, id: &str, user_id: &str, is_admin: bool) -> AppResult<()> {
        if !is_admin && !self.project_repo.is_owner(id, user_id).await? {
            return Err(AppError::Forbidden);
        }

        self.project_repo.delete(id).await
    }

    pub async fn is_owner(&self, project_id: &str, user_id: &str) -> AppResult<bool> {
        self.project_repo.is_owner(project_id, user_id).await
    }

    pub async fn count_by_user(&self, user_id: &str) -> AppResult<i64> {
        self.project_repo.count_by_user(user_id).await
    }

    pub async fn count_all(&self) -> AppResult<i64> {
        self.project_repo.count_all().await
    }
}
