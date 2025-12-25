//! Approval gate handlers.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use oxide_core::approval::{ApprovalGate, ApprovalStatus, ApproverAction};
use oxide_core::ids::ApprovalGateId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ApprovalRequest {
    pub action: ApproverAction,
    pub user_id: String,
    pub user_name: Option<String>,
    pub user_email: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApprovalResponse {
    pub gate_id: ApprovalGateId,
    pub status: ApprovalStatus,
    pub current_approvals: u32,
    pub required_approvals: u32,
    pub fully_approved: bool,
    pub message: String,
}

/// List pending approval gates.
pub async fn list_approvals(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApprovalGate>>, StatusCode> {
    let gates = state.approvals.list(None).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(gates))
}

/// Get a specific approval gate.
pub async fn get_approval(
    State(state): State<Arc<AppState>>,
    Path(gate_id): Path<String>,
) -> Result<Json<ApprovalGate>, StatusCode> {
    let gate_id: ApprovalGateId = gate_id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    match state.approvals.get(gate_id).await {
        Ok(Some(gate)) => Ok(Json(gate)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Approve or reject an approval gate.
pub async fn respond_to_approval(
    State(state): State<Arc<AppState>>,
    Path(gate_id): Path<String>,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let gate_id: ApprovalGateId = gate_id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut gate = match state.approvals.get(gate_id).await {
        Ok(Some(g)) => g,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    if gate.status != ApprovalStatus::Pending {
         return Err(StatusCode::CONFLICT); // Already decided
    }

    if !gate.can_approve(&request.user_id, None) { // TODO: pass triggered_by if known from context/auth
        return Err(StatusCode::FORBIDDEN);
    }

    let approver = oxide_core::approval::Approver {
        user_id: request.user_id,
        user_name: request.user_name,
        user_email: request.user_email,
        action: request.action,
        comment: request.comment,
        acted_at: chrono::Utc::now(),
    };

    match request.action {
        ApproverAction::Approved => gate.approve(approver),
        ApproverAction::Rejected => gate.reject(approver),
    }

    state.approvals.update(&gate).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Publish event
    // TODO: Publish ApprovalGranted/Rejected event via state.event_bus

    Ok(Json(ApprovalResponse {
        gate_id: gate.id,
        status: gate.status,
        current_approvals: gate.current_approvals,
        required_approvals: gate.required_approvers,
        fully_approved: gate.is_fully_approved(),
        message: format!("Approval {}", match gate.status {
            ApprovalStatus::Approved => "granted",
            ApprovalStatus::Rejected => "rejected",
            _ => "recorded",
        }),
    }))
}

/// Bypass an approval gate (admin only).
pub async fn bypass_approval(
    State(state): State<Arc<AppState>>,
    Path(gate_id): Path<String>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let gate_id: ApprovalGateId = gate_id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut gate = match state.approvals.get(gate_id).await {
        Ok(Some(g)) => g,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    gate.status = ApprovalStatus::Bypassed;
    state.approvals.update(&gate).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Publish event

    Ok(Json(ApprovalResponse {
        gate_id: gate.id,
        status: ApprovalStatus::Bypassed,
        current_approvals: gate.current_approvals,
        required_approvals: gate.required_approvers,
        fully_approved: true, // Bypass implies approval
        message: "Approval bypassed by admin".to_string(),
    }))
}
