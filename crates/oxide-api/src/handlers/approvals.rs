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
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApprovalGate>>, StatusCode> {
    // TODO: Fetch pending approvals from database
    Ok(Json(vec![]))
}

/// Get a specific approval gate.
pub async fn get_approval(
    State(_state): State<Arc<AppState>>,
    Path(gate_id): Path<String>,
) -> Result<Json<ApprovalGate>, StatusCode> {
    let _gate_id: ApprovalGateId = gate_id
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // TODO: Fetch approval gate from database
    Err(StatusCode::NOT_FOUND)
}

/// Approve or reject an approval gate.
pub async fn respond_to_approval(
    State(_state): State<Arc<AppState>>,
    Path(gate_id): Path<String>,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let gate_id: ApprovalGateId = gate_id
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // TODO: Fetch gate, validate user can approve, update state, publish event

    let response = match request.action {
        ApproverAction::Approved => ApprovalResponse {
            gate_id,
            status: ApprovalStatus::Pending,
            current_approvals: 1,
            required_approvals: 2,
            fully_approved: false,
            message: "Approval recorded".to_string(),
        },
        ApproverAction::Rejected => ApprovalResponse {
            gate_id,
            status: ApprovalStatus::Rejected,
            current_approvals: 0,
            required_approvals: 2,
            fully_approved: false,
            message: "Approval rejected".to_string(),
        },
    };

    Ok(Json(response))
}

/// Bypass an approval gate (admin only).
pub async fn bypass_approval(
    State(_state): State<Arc<AppState>>,
    Path(gate_id): Path<String>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let gate_id: ApprovalGateId = gate_id
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // TODO: Verify admin permissions, bypass gate, publish event

    Ok(Json(ApprovalResponse {
        gate_id,
        status: ApprovalStatus::Bypassed,
        current_approvals: 0,
        required_approvals: 0,
        fully_approved: true,
        message: "Approval bypassed by admin".to_string(),
    }))
}
