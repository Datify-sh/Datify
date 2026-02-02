use axum::{
    extract::{Path, State},
    Json,
};

use crate::api::extractors::AuthUser;
use crate::api::handlers::DatabaseServiceState;
use crate::domain::models::{
    DatabaseConfigResponse, UpdateDatabaseConfigRequest, UpdateDatabaseConfigResponse,
};
use crate::error::AppResult;

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/config",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "Database config retrieved", body = DatabaseConfigResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Config",
    security(("bearer" = []))
)]
pub async fn get_database_config(
    State(database_service): State<DatabaseServiceState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<DatabaseConfigResponse>> {
    let config = database_service
        .get_config(&id, auth_user.id(), auth_user.is_admin())
        .await?;
    Ok(Json(config))
}

#[utoipa::path(
    put,
    path = "/api/v1/databases/{id}/config",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    request_body = UpdateDatabaseConfigRequest,
    responses(
        (status = 200, description = "Database config updated", body = UpdateDatabaseConfigResponse),
        (status = 400, description = "Invalid config"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Config",
    security(("bearer" = []))
)]
pub async fn update_database_config(
    State(database_service): State<DatabaseServiceState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Json(payload): Json<UpdateDatabaseConfigRequest>,
) -> AppResult<Json<UpdateDatabaseConfigResponse>> {
    let response = database_service
        .update_config(&id, auth_user.id(), auth_user.is_admin(), &payload.content)
        .await?;
    Ok(Json(response))
}
