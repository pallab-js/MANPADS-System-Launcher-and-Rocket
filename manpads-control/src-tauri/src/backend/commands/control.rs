use crate::lib::{ControlCommand, AppError};
use crate::backend::udp::socket;
use tracing::{info, error};

#[tauri::command]
pub async fn update_pid(kp: f32, kd: f32) -> Result<(), AppError> {
    info!("Updating PID: kp={}, kd={}", kp, kd);
    
    if kp < 0.0 || kp > 10.0 {
        return Err(AppError::ParseError("kp must be between 0.0 and 10.0".to_string()));
    }
    if kd < 0.0 || kd > 5.0 {
        return Err(AppError::ParseError("kd must be between 0.0 and 5.0".to_string()));
    }
    
    socket::send(&ControlCommand::UpdatePid { kp, kd }).await.map_err(|e| {
        error!("PID update failed: {}", e);
        AppError::UdpError(e)
    })?;
    
    Ok(())
}