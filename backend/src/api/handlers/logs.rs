use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::IntoResponse,
    Json,
};
use futures::{SinkExt, StreamExt};

use crate::api::extractors::AuthUser;
use crate::domain::models::{
    LogEntryResponse, LogQueryParams, LogStreamMessage, LogType, LogsResponse,
};
use crate::domain::services::DatabaseService;
use crate::error::{AppError, AppResult};
use crate::infrastructure::docker::DockerManager;

#[derive(Clone)]
pub struct LogsState {
    pub database_service: Arc<DatabaseService>,
    pub docker: Arc<DockerManager>,
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/logs",
    params(
        ("id" = String, Path, description = "Database ID"),
        ("tail" = Option<i64>, Query, description = "Number of log lines to retrieve (default: 100)"),
        ("since" = Option<i64>, Query, description = "Unix timestamp to retrieve logs since"),
        ("timestamps" = Option<bool>, Query, description = "Include timestamps in log entries")
    ),
    responses(
        (status = 200, description = "Database logs retrieved successfully", body = LogsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database or container not found")
    ),
    tag = "Logs",
    security(("bearer" = []))
)]
pub async fn get_database_logs(
    State(state): State<LogsState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Query(params): Query<LogQueryParams>,
) -> AppResult<Json<LogsResponse>> {
    if !state
        .database_service
        .check_access(&id, auth_user.id())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let database = state
        .database_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

    let container_id = database
        .container_id
        .as_ref()
        .ok_or_else(|| AppError::NotFound("Database has no container".to_string()))?;

    let logs = state
        .docker
        .get_container_logs(
            container_id,
            Some(params.tail),
            params.since,
            params.timestamps,
        )
        .await?;

    let entries: Vec<LogEntryResponse> = logs
        .entries
        .into_iter()
        .map(|entry| LogEntryResponse {
            timestamp: entry.timestamp,
            log_type: LogType::Runtime,
            stream: entry.stream,
            message: entry.message,
        })
        .collect();

    Ok(Json(LogsResponse {
        database_id: id,
        container_id: Some(container_id.clone()),
        entries,
        has_more: logs.has_more,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/logs/stream",
    params(
        ("id" = String, Path, description = "Database ID"),
        ("tail" = Option<i64>, Query, description = "Number of initial log lines (default: 100)")
    ),
    responses(
        (status = 101, description = "WebSocket connection established for log streaming"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database or container not found")
    ),
    tag = "Logs",
    security(("bearer" = []))
)]
pub async fn stream_database_logs(
    State(state): State<LogsState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Query(params): Query<LogQueryParams>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .database_service
        .check_access(&id, auth_user.id())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let database = state
        .database_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

    let container_id = database
        .container_id
        .ok_or_else(|| AppError::NotFound("Database has no container".to_string()))?;

    let tail = params.tail;

    Ok(ws.on_upgrade(move |socket| handle_log_stream(socket, state, container_id, tail)))
}

const MAX_PENDING_MESSAGES: usize = 100;

async fn handle_log_stream(socket: WebSocket, state: LogsState, container_id: String, tail: i64) {
    let (mut sender, mut receiver) = socket.split();

    let connected_msg = serde_json::to_string(&LogStreamMessage::Connected).unwrap();
    if sender
        .send(Message::Text(connected_msg.into()))
        .await
        .is_err()
    {
        return;
    }

    let log_stream = state
        .docker
        .stream_container_logs(&container_id, Some(tail));

    let mut log_stream = std::pin::pin!(log_stream);

    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(30));
    let mut pending_messages: std::collections::VecDeque<String> =
        std::collections::VecDeque::with_capacity(MAX_PENDING_MESSAGES);

    loop {
        tokio::select! {
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Close(_)) => break,
                    Ok(Message::Ping(data)) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                    _ => {}
                }
            }

            Some(result) = log_stream.next() => {
                match result {
                    Ok(entry) => {
                        let msg = LogStreamMessage::Log(LogEntryResponse {
                            timestamp: entry.timestamp,
                            log_type: LogType::Runtime,
                            stream: entry.stream,
                            message: entry.message,
                        });
                        let json = serde_json::to_string(&msg).unwrap();

                        if pending_messages.len() >= MAX_PENDING_MESSAGES {
                            pending_messages.pop_front();
                        }
                        pending_messages.push_back(json);

                        while let Some(queued) = pending_messages.pop_front() {
                            match sender.send(Message::Text(queued.clone().into())).await {
                                Ok(_) => {},
                                Err(_) => {
                                    pending_messages.push_front(queued);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let msg = LogStreamMessage::Error { message: e.to_string() };
                        let json = serde_json::to_string(&msg).unwrap();
                        let _ = sender.send(Message::Text(json.into())).await;
                        break;
                    }
                }
            }

            _ = heartbeat.tick() => {
                let msg = serde_json::to_string(&LogStreamMessage::Ping).unwrap();
                if sender.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}
