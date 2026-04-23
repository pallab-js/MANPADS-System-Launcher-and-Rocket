use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, info};
use crate::lib::{TelemetryMessage, ControlCommand};

lazy_static::lazy_static! {
    static ref SOCKET: tokio::sync::RwLock<Option<Arc<UdpSocket>>> = tokio::sync::RwLock::new(None);
    static ref TARGET: tokio::sync::RwLock<Option<SocketAddr>> = tokio::sync::RwLock::new(None);
}

pub async fn connect(ip: &str, port: u16) -> Result<(), String> {
    let local_addr = format!("0.0.0.0:{}", port + 1);
    
    let socket = Arc::new(
        UdpSocket::bind(&local_addr)
            .await
            .map_err(|e| format!("Failed to bind socket: {}", e))?
    );

    let target_addr: SocketAddr = format!("{}:{}", ip, port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    info!("Connected to {}:{}", ip, port);
    
    *SOCKET.write().await = Some(socket);
    *TARGET.write().await = Some(target_addr);
    
    Ok(())
}

pub async fn disconnect() {
    info!("Disconnected from UDP");
    *SOCKET.write().await = None;
    *TARGET.write().await = None;
}

pub async fn send(cmd: &ControlCommand) -> Result<(), String> {
    let socket = SOCKET.read().await;
    let socket = socket.as_ref().ok_or("Not connected")?;
    
    let target = TARGET.read().await;
    let target = target.ok_or("No target address")?;

    let data = serialize_command(cmd);
    
    socket
        .send_to(data.as_bytes(), target)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;

    debug!("Sent: {:?}", cmd);
    Ok(())
}

pub async fn receive(buffer: &mut [u8]) -> Result<(usize, SocketAddr), String> {
    let socket = SOCKET.read().await;
    let socket = socket.as_ref().ok_or("Not connected")?;
    
    socket
        .recv_from(buffer)
        .await
        .map_err(|e| format!("Receive failed: {}", e))
}

pub async fn is_connected() -> bool {
    SOCKET.read().await.is_some() && TARGET.read().await.is_some()
}

pub fn parse_incoming_data(data: &[u8]) -> Vec<TelemetryMessage> {
    let text = match std::str::from_utf8(data) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    text.lines()
        .filter_map(|line| {
            let msg = parse_telemetry(line);
            if msg.is_some() {
                debug!("Parsed: {}", line);
            }
            msg
        })
        .collect()
}

fn parse_telemetry(line: &str) -> Option<TelemetryMessage> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let parts: Vec<&str> = line.split(',').collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "ROCKET" => {
            if parts.len() >= 5 {
                Some(TelemetryMessage::Rocket {
                    timestamp_ms: parts[1].parse().unwrap_or(0),
                    roll_deg: parts[2].parse().unwrap_or(0.0),
                    rotation_rate: parts[3].parse().unwrap_or(0.0),
                    servo_output: parts[4].parse().unwrap_or(0),
                })
            } else {
                None
            }
        }
        "STATUS" => {
            if parts.len() >= 5 {
                Some(TelemetryMessage::RocketStatus {
                    state: parts[1].to_string(),
                    kp: parts[2].parse().unwrap_or(0.0),
                    kd: parts[3].parse().unwrap_or(0.0),
                    skew: parts[4].parse().unwrap_or(0.0),
                })
            } else {
                None
            }
        }
        "LAUNCHER" => {
            if parts.len() >= 7 {
                Some(TelemetryMessage::Launcher {
                    latitude: parts[1].parse().unwrap_or(0.0),
                    longitude: parts[2].parse().unwrap_or(0.0),
                    altitude_m: parts[3].parse().unwrap_or(0.0),
                    pressure: parts[4].parse().unwrap_or(0.0),
                    temperature: parts[5].parse().unwrap_or(0.0),
                    heading: parts[6].parse().unwrap_or(0.0),
                })
            } else {
                None
            }
        }
        "HEADING" => {
            if parts.len() >= 2 {
                Some(TelemetryMessage::Launcher {
                    latitude: 0.0,
                    longitude: 0.0,
                    altitude_m: 0.0,
                    pressure: 0.0,
                    temperature: parts[1].parse().unwrap_or(0.0),
                    heading: parts[1].parse().unwrap_or(0.0),
                })
            } else {
                None
            }
        }
        "DEBUG" => {
            Some(TelemetryMessage::Debug {
                message: parts.get(1).unwrap_or(&"").to_string(),
            })
        }
        _ => None
    }
}

fn serialize_command(cmd: &ControlCommand) -> String {
    match cmd {
        ControlCommand::UpdatePid { kp, kd } => {
            format!("PID,{},{}\n", kp, kd)
        }
        ControlCommand::Launch => "LAUNCH\n".to_string(),
        ControlCommand::Calibrate => "CALIBRATE\n".to_string(),
        ControlCommand::EmergencyStop => "ESTOP\n".to_string(),
        ControlCommand::Arm => "ARM\n".to_string(),
        ControlCommand::Disarm => "DISARM\n".to_string(),
    }
}