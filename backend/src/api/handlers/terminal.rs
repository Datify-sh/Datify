use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use bollard::exec::{StartExecOptions, StartExecResults};
use futures::{
    future::{AbortHandle, Abortable},
    SinkExt, StreamExt,
};
use tokio::io::AsyncWriteExt;

use crate::api::extractors::AuthUser;
use crate::domain::models::{TerminalInputMessage, TerminalOutputMessage};
use crate::domain::services::DatabaseService;
use crate::error::AppError;
use crate::infrastructure::docker::DockerManager;

#[derive(Clone)]
pub struct TerminalState {
    pub database_service: Arc<DatabaseService>,
    pub docker: Arc<DockerManager>,
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/terminal",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 101, description = "WebSocket connection established for interactive terminal"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database or container not found"),
        (status = 409, description = "Container is not running")
    ),
    tag = "Terminal",
    security(("bearer" = []))
)]
pub async fn database_terminal(
    State(state): State<TerminalState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .database_service
        .check_access(&id, auth_user.id(), auth_user.is_admin())
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

    let status = state.docker.get_container_status(&container_id).await?;
    if status != "running" {
        return Err(AppError::Conflict(format!(
            "Container is not running (status: {})",
            status
        )));
    }

    tracing::info!(
        user_id = %auth_user.id(),
        user_email = %auth_user.email(),
        database_id = %id,
        container_id = %container_id,
        session_type = "shell",
        "Terminal session started"
    );

    Ok(ws.on_upgrade(move |socket| handle_terminal(socket, state, container_id)))
}

async fn handle_terminal(socket: WebSocket, state: TerminalState, container_id: String) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let exec_id = match state
        .docker
        .create_exec(&container_id, vec!["/bin/bash", "-l"], true)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            match state
                .docker
                .create_exec(&container_id, vec!["/bin/sh"], true)
                .await
            {
                Ok(id) => id,
                Err(_) => {
                    let msg = TerminalOutputMessage::Error {
                        message: format!("Failed to create exec session: {}", e),
                    };
                    let json = serde_json::to_string(&msg).unwrap();
                    let _ = ws_sender.send(Message::Text(json.into())).await;
                    return;
                },
            }
        },
    };

    let connected_msg = TerminalOutputMessage::Connected {
        exec_id: exec_id.clone(),
    };
    let json = serde_json::to_string(&connected_msg).unwrap();
    if ws_sender.send(Message::Text(json.into())).await.is_err() {
        return;
    }

    let exec_options = StartExecOptions {
        detach: false,
        tty: true,
        output_capacity: None,
    };

    let exec_result = match state
        .docker
        .docker()
        .start_exec(&exec_id, Some(exec_options))
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let msg = TerminalOutputMessage::Error {
                message: format!("Failed to start exec: {}", e),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
            return;
        },
    };

    match exec_result {
        StartExecResults::Attached {
            mut output,
            mut input,
        } => {
            let exec_id_for_resize = exec_id.clone();
            let docker_for_resize = state.docker.clone();

            let (output_abort, output_reg) = AbortHandle::new_pair();
            let mut output_task = tokio::spawn(Abortable::new(
                async move {
                    while let Some(result) = output.next().await {
                        match result {
                            Ok(output) => {
                                let data = match output {
                                    bollard::container::LogOutput::StdOut { message } => message,
                                    bollard::container::LogOutput::StdErr { message } => message,
                                    bollard::container::LogOutput::Console { message } => message,
                                    _ => continue,
                                };

                                let text = String::from_utf8_lossy(&data).to_string();
                                let msg = TerminalOutputMessage::Output { data: text };
                                let json = serde_json::to_string(&msg).unwrap();

                                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            },
                            Err(e) => {
                                let msg = TerminalOutputMessage::Error {
                                    message: format!("Stream error: {}", e),
                                };
                                let json = serde_json::to_string(&msg).unwrap();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                                break;
                            },
                        }
                    }
                },
                output_reg,
            ));

            let (input_abort, input_reg) = AbortHandle::new_pair();
            let mut input_task = tokio::spawn(Abortable::new(
                async move {
                    while let Some(msg) = ws_receiver.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(terminal_msg) =
                                    serde_json::from_str::<TerminalInputMessage>(&text)
                                {
                                    match terminal_msg {
                                        TerminalInputMessage::Input { data } => {
                                            if input.write_all(data.as_bytes()).await.is_err() {
                                                break;
                                            }
                                            if input.flush().await.is_err() {
                                                break;
                                            }
                                        },
                                        TerminalInputMessage::Resize { cols, rows } => {
                                            let _ = docker_for_resize
                                                .resize_exec(&exec_id_for_resize, cols, rows)
                                                .await;
                                        },
                                        TerminalInputMessage::Ping => {},
                                    }
                                }
                            },
                            Ok(Message::Binary(data)) => {
                                if input.write_all(&data).await.is_err() {
                                    break;
                                }
                                if input.flush().await.is_err() {
                                    break;
                                }
                            },
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {},
                        }
                    }
                },
                input_reg,
            ));

            tokio::select! {
                _ = &mut output_task => {
                    input_abort.abort();
                }
                _ = &mut input_task => {
                    output_abort.abort();
                }
            }

            let _ = output_task.await;
            let _ = input_task.await;
        },
        StartExecResults::Detached => {
            let msg = TerminalOutputMessage::Error {
                message: "Exec started in detached mode".to_string(),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/psql",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 101, description = "WebSocket connection established for psql terminal"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database or container not found"),
        (status = 409, description = "Container is not running")
    ),
    tag = "Terminal",
    security(("bearer" = []))
)]
pub async fn database_psql(
    State(state): State<TerminalState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .database_service
        .check_access(&id, auth_user.id(), auth_user.is_admin())
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

    let status = state.docker.get_container_status(&container_id).await?;
    if status != "running" {
        return Err(AppError::Conflict(format!(
            "Container is not running (status: {})",
            status
        )));
    }

    let username = database.username;

    tracing::info!(
        user_id = %auth_user.id(),
        user_email = %auth_user.email(),
        database_id = %id,
        container_id = %container_id,
        session_type = "psql",
        "PSQL session started"
    );

    Ok(ws.on_upgrade(move |socket| handle_psql_terminal(socket, state, container_id, username)))
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/valkey-cli",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 101, description = "WebSocket connection established for valkey-cli terminal"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database or container not found"),
        (status = 409, description = "Container is not running or not a Valkey database")
    ),
    tag = "Terminal",
    security(("bearer" = []))
)]
pub async fn database_valkey_cli(
    State(state): State<TerminalState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .database_service
        .check_access(&id, auth_user.id(), auth_user.is_admin())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let database = state
        .database_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

    if database.database_type != "valkey" {
        return Err(AppError::Conflict(
            "This endpoint is only for Valkey databases".to_string(),
        ));
    }

    let container_id = database
        .container_id
        .ok_or_else(|| AppError::NotFound("Database has no container".to_string()))?;

    let status = state.docker.get_container_status(&container_id).await?;
    if status != "running" {
        return Err(AppError::Conflict(format!(
            "Container is not running (status: {})",
            status
        )));
    }

    let password = database
        .password_encrypted
        .as_ref()
        .map(|_| "********".to_string())
        .unwrap_or_default();

    tracing::info!(
        user_id = %auth_user.id(),
        user_email = %auth_user.email(),
        database_id = %id,
        container_id = %container_id,
        session_type = "valkey-cli",
        "Valkey CLI session started"
    );

    Ok(ws.on_upgrade(move |socket| {
        handle_valkey_cli_terminal(socket, state, container_id, password)
    }))
}

async fn handle_valkey_cli_terminal(
    socket: WebSocket,
    state: TerminalState,
    container_id: String,
    _password: String,
) {
    handle_kv_cli_terminal(
        socket,
        state,
        container_id,
        vec!["valkey-cli"],
        "valkey-cli",
    )
    .await;
}

#[utoipa::path(
    get,
    path = "/api/v1/databases/{id}/redis-cli",
    params(
        ("id" = String, Path, description = "Database ID")
    ),
    responses(
        (status = 101, description = "WebSocket connection established for redis-cli terminal"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - no access to database"),
        (status = 404, description = "Database or container not found"),
        (status = 409, description = "Container is not running or not a Redis database")
    ),
    tag = "Terminal",
    security(("bearer" = []))
)]
pub async fn database_redis_cli(
    State(state): State<TerminalState>,
    auth_user: AuthUser,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    if !state
        .database_service
        .check_access(&id, auth_user.id(), auth_user.is_admin())
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let database = state
        .database_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Database '{}' not found", id)))?;

    if database.database_type != "redis" {
        return Err(AppError::Conflict(
            "This endpoint is only for Redis databases".to_string(),
        ));
    }

    let container_id = database
        .container_id
        .ok_or_else(|| AppError::NotFound("Database has no container".to_string()))?;

    let status = state.docker.get_container_status(&container_id).await?;
    if status != "running" {
        return Err(AppError::Conflict(format!(
            "Container is not running (status: {})",
            status
        )));
    }

    let password = database
        .password_encrypted
        .as_ref()
        .map(|_| "********".to_string())
        .unwrap_or_default();

    tracing::info!(
        user_id = %auth_user.id(),
        user_email = %auth_user.email(),
        database_id = %id,
        container_id = %container_id,
        session_type = "redis-cli",
        "Redis CLI session started"
    );

    Ok(ws
        .on_upgrade(move |socket| handle_redis_cli_terminal(socket, state, container_id, password)))
}

async fn handle_redis_cli_terminal(
    socket: WebSocket,
    state: TerminalState,
    container_id: String,
    _password: String,
) {
    handle_kv_cli_terminal(socket, state, container_id, vec!["redis-cli"], "redis-cli").await;
}

async fn handle_kv_cli_terminal(
    socket: WebSocket,
    state: TerminalState,
    container_id: String,
    cmd: Vec<&str>,
    cli_name: &str,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let exec_id = match state.docker.create_exec(&container_id, cmd, true).await {
        Ok(id) => id,
        Err(e) => {
            let msg = TerminalOutputMessage::Error {
                message: format!("Failed to create {} session: {}", cli_name, e),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
            return;
        },
    };

    let connected_msg = TerminalOutputMessage::Connected {
        exec_id: exec_id.clone(),
    };
    let json = serde_json::to_string(&connected_msg).unwrap();
    if ws_sender.send(Message::Text(json.into())).await.is_err() {
        return;
    }

    let exec_options = StartExecOptions {
        detach: false,
        tty: true,
        output_capacity: None,
    };

    let exec_result = match state
        .docker
        .docker()
        .start_exec(&exec_id, Some(exec_options))
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let msg = TerminalOutputMessage::Error {
                message: format!("Failed to start {}: {}", cli_name, e),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
            return;
        },
    };

    match exec_result {
        StartExecResults::Attached {
            mut output,
            mut input,
        } => {
            let exec_id_for_resize = exec_id.clone();
            let docker_for_resize = state.docker.clone();

            let (output_abort, output_reg) = AbortHandle::new_pair();
            let mut output_task = tokio::spawn(Abortable::new(
                async move {
                    while let Some(result) = output.next().await {
                        match result {
                            Ok(output) => {
                                let data = match output {
                                    bollard::container::LogOutput::StdOut { message } => message,
                                    bollard::container::LogOutput::StdErr { message } => message,
                                    bollard::container::LogOutput::Console { message } => message,
                                    _ => continue,
                                };

                                let text = String::from_utf8_lossy(&data).to_string();
                                let msg = TerminalOutputMessage::Output { data: text };
                                let json = serde_json::to_string(&msg).unwrap();

                                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            },
                            Err(e) => {
                                let msg = TerminalOutputMessage::Error {
                                    message: format!("Stream error: {}", e),
                                };
                                let json = serde_json::to_string(&msg).unwrap();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                                break;
                            },
                        }
                    }
                },
                output_reg,
            ));

            let (input_abort, input_reg) = AbortHandle::new_pair();
            let mut input_task = tokio::spawn(Abortable::new(
                async move {
                    while let Some(msg) = ws_receiver.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(terminal_msg) =
                                    serde_json::from_str::<TerminalInputMessage>(&text)
                                {
                                    match terminal_msg {
                                        TerminalInputMessage::Input { data } => {
                                            if input.write_all(data.as_bytes()).await.is_err() {
                                                break;
                                            }
                                            if input.flush().await.is_err() {
                                                break;
                                            }
                                        },
                                        TerminalInputMessage::Resize { cols, rows } => {
                                            let _ = docker_for_resize
                                                .resize_exec(&exec_id_for_resize, cols, rows)
                                                .await;
                                        },
                                        TerminalInputMessage::Ping => {},
                                    }
                                }
                            },
                            Ok(Message::Binary(data)) => {
                                if input.write_all(&data).await.is_err() {
                                    break;
                                }
                                if input.flush().await.is_err() {
                                    break;
                                }
                            },
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {},
                        }
                    }
                },
                input_reg,
            ));

            tokio::select! {
                _ = &mut output_task => {
                    input_abort.abort();
                }
                _ = &mut input_task => {
                    output_abort.abort();
                }
            }

            let _ = output_task.await;
            let _ = input_task.await;
        },
        StartExecResults::Detached => {
            let msg = TerminalOutputMessage::Error {
                message: "Exec started in detached mode".to_string(),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
        },
    }
}

async fn handle_psql_terminal(
    socket: WebSocket,
    state: TerminalState,
    container_id: String,
    username: String,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let psql_cmd = vec!["psql", "-U", &username, "-d", "postgres"];
    let exec_id = match state
        .docker
        .create_exec(&container_id, psql_cmd, true)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            let msg = TerminalOutputMessage::Error {
                message: format!("Failed to create psql session: {}", e),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
            return;
        },
    };

    let connected_msg = TerminalOutputMessage::Connected {
        exec_id: exec_id.clone(),
    };
    let json = serde_json::to_string(&connected_msg).unwrap();
    if ws_sender.send(Message::Text(json.into())).await.is_err() {
        return;
    }

    let exec_options = StartExecOptions {
        detach: false,
        tty: true,
        output_capacity: None,
    };

    let exec_result = match state
        .docker
        .docker()
        .start_exec(&exec_id, Some(exec_options))
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let msg = TerminalOutputMessage::Error {
                message: format!("Failed to start psql: {}", e),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
            return;
        },
    };

    match exec_result {
        StartExecResults::Attached {
            mut output,
            mut input,
        } => {
            let exec_id_for_resize = exec_id.clone();
            let docker_for_resize = state.docker.clone();

            let (output_abort, output_reg) = AbortHandle::new_pair();
            let mut output_task = tokio::spawn(Abortable::new(
                async move {
                    while let Some(result) = output.next().await {
                        match result {
                            Ok(output) => {
                                let data = match output {
                                    bollard::container::LogOutput::StdOut { message } => message,
                                    bollard::container::LogOutput::StdErr { message } => message,
                                    bollard::container::LogOutput::Console { message } => message,
                                    _ => continue,
                                };

                                let text = String::from_utf8_lossy(&data).to_string();
                                let msg = TerminalOutputMessage::Output { data: text };
                                let json = serde_json::to_string(&msg).unwrap();

                                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            },
                            Err(e) => {
                                let msg = TerminalOutputMessage::Error {
                                    message: format!("Stream error: {}", e),
                                };
                                let json = serde_json::to_string(&msg).unwrap();
                                let _ = ws_sender.send(Message::Text(json.into())).await;
                                break;
                            },
                        }
                    }
                },
                output_reg,
            ));

            let (input_abort, input_reg) = AbortHandle::new_pair();
            let mut input_task = tokio::spawn(Abortable::new(
                async move {
                    while let Some(msg) = ws_receiver.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(terminal_msg) =
                                    serde_json::from_str::<TerminalInputMessage>(&text)
                                {
                                    match terminal_msg {
                                        TerminalInputMessage::Input { data } => {
                                            if input.write_all(data.as_bytes()).await.is_err() {
                                                break;
                                            }
                                            if input.flush().await.is_err() {
                                                break;
                                            }
                                        },
                                        TerminalInputMessage::Resize { cols, rows } => {
                                            let _ = docker_for_resize
                                                .resize_exec(&exec_id_for_resize, cols, rows)
                                                .await;
                                        },
                                        TerminalInputMessage::Ping => {},
                                    }
                                }
                            },
                            Ok(Message::Binary(data)) => {
                                if input.write_all(&data).await.is_err() {
                                    break;
                                }
                                if input.flush().await.is_err() {
                                    break;
                                }
                            },
                            Ok(Message::Close(_)) => break,
                            Err(_) => break,
                            _ => {},
                        }
                    }
                },
                input_reg,
            ));

            tokio::select! {
                _ = &mut output_task => {
                    input_abort.abort();
                }
                _ = &mut input_task => {
                    output_abort.abort();
                }
            }

            let _ = output_task.await;
            let _ = input_task.await;
        },
        StartExecResults::Detached => {
            let msg = TerminalOutputMessage::Error {
                message: "Exec started in detached mode".to_string(),
            };
            let json = serde_json::to_string(&msg).unwrap();
            let _ = ws_sender.send(Message::Text(json.into())).await;
        },
    }
}
