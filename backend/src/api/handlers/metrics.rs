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
    MetricsHistory, MetricsHistoryQuery, MetricsStreamMessage, QueryLogsQuery, QueryLogsResponse,
    TimeRange, UnifiedMetricsResponse,
};
use crate::domain::services::MetricsService;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct MetricsState {
    pub metrics_service: Arc<MetricsService>,
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/metrics",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 200, description = "Current metrics snapshot", body = UnifiedMetricsResponse),
        (status = 400, description = "Database not running"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Metrics",
    security(("bearer" = []))
)]
pub async fn get_database_metrics(
    State(state): State<MetricsState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<UnifiedMetricsResponse>> {
    if !state
        .metrics_service
        .check_access(&id, auth_user.id())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let metrics = state.metrics_service.get_current_metrics(&id).await?;

    Ok(Json(metrics))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/metrics/history",
    params(
        ("id" = String, Path, description = "Database ID"),
        ("range" = Option<String>, Query, description = "Time range: realtime, last_5_min, last_15_min, last_30_min, last_1_hour, last_24_hours")
    ),
    responses(
        (status = 200, description = "Historical metrics data", body = MetricsHistory),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Metrics",
    security(("bearer" = []))
)]
pub async fn get_database_metrics_history(
    State(state): State<MetricsState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Query(query): Query<MetricsHistoryQuery>,
) -> AppResult<Json<MetricsHistory>> {
    if !state
        .metrics_service
        .check_access(&id, auth_user.id())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let time_range = query
        .range
        .as_deref()
        .map(|r| r.parse::<TimeRange>())
        .transpose()
        .map_err(AppError::Validation)?
        .unwrap_or_default();

    let history = state
        .metrics_service
        .get_metrics_history(&id, time_range)
        .await?;

    Ok(Json(history))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/queries",
    params(
        ("id" = String, Path, description = "Database ID"),
        ("limit" = Option<i32>, Query, description = "Maximum number of entries (default: 50)"),
        ("sort_by" = Option<String>, Query, description = "Sort by: total_time, avg_time, calls (default: total_time)")
    ),
    responses(
        (status = 200, description = "Query logs", body = QueryLogsResponse),
        (status = 400, description = "Database not running"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Metrics",
    security(("bearer" = []))
)]
pub async fn get_database_queries(
    State(state): State<MetricsState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    Query(query): Query<QueryLogsQuery>,
) -> AppResult<Json<QueryLogsResponse>> {
    if !state
        .metrics_service
        .check_access(&id, auth_user.id())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let limit = query.limit.unwrap_or(50).min(100);
    let sort_by = query.sort_by.as_deref().unwrap_or("total_time");

    let logs = state
        .metrics_service
        .get_query_logs(&id, limit, sort_by)
        .await?;

    Ok(Json(logs))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/metrics/stream",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 101, description = "WebSocket connection established for metrics streaming"),
        (status = 400, description = "Database not running"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database not found")
    ),
    tag = "Metrics",
    security(("bearer" = []))
)]
pub async fn stream_database_metrics(
    State(state): State<MetricsState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .metrics_service
        .check_access(&id, auth_user.id())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    Ok(ws.on_upgrade(move |socket| handle_metrics_stream(socket, state, id)))
}

async fn handle_metrics_stream(socket: WebSocket, state: MetricsState, database_id: String) {
    let (mut sender, mut receiver) = socket.split();

    let connected_msg = serde_json::to_string(&MetricsStreamMessage::Connected {
        database_id: database_id.clone(),
    })
    .unwrap();

    if sender
        .send(Message::Text(connected_msg.into()))
        .await
        .is_err()
    {
        return;
    }

    let mut metrics_interval = tokio::time::interval(std::time::Duration::from_secs(1));

    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(30));

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

            _ = metrics_interval.tick() => {
                match state.metrics_service.get_current_metrics(&database_id).await {
                    Ok(response) => {
                        let msg = MetricsStreamMessage::Metrics {
                            metrics: response.metrics,
                        };
                        let json = serde_json::to_string(&msg).unwrap();
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let msg = MetricsStreamMessage::Error {
                            message: e.to_string(),
                        };
                        let json = serde_json::to_string(&msg).unwrap();
                        let _ = sender.send(Message::Text(json.into())).await;
                    }
                }
            }

            _ = heartbeat.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }
}
