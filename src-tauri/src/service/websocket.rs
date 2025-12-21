use crate::service::events::{EventListener, ServiceEvent};
/// WebSocket/SSE 前端事件推送
use std::sync::{Arc, Mutex};

/// WebSocket 事件监听器（用于向前端推送事件）
#[allow(dead_code)]
pub struct WebSocketEventListener {
    /// 连接的客户端 ID
    client_id: String,
    /// 事件发送器（实际实现中应该使用 WebSocket 发送）
    sender: Arc<Mutex<Option<Box<dyn Fn(&ServiceEvent) + Send + Sync>>>>,
}

impl WebSocketEventListener {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            sender: Arc::new(Mutex::new(None)),
        }
    }

    #[allow(dead_code)]
    pub fn set_sender(&self, sender: Box<dyn Fn(&ServiceEvent) + Send + Sync>) {
        let mut s =
            crate::utils::lock_or_recover(self.sender.as_ref(), "WebSocketEventListener.sender");
        *s = Some(sender);
    }
}

impl EventListener for WebSocketEventListener {
    fn on_event(&self, event: &ServiceEvent) {
        let sender =
            crate::utils::lock_or_recover(self.sender.as_ref(), "WebSocketEventListener.sender");
        if let Some(ref send_fn) = *sender {
            send_fn(event);
        }
    }
}

/// SSE 事件流管理器
pub struct SSEEventStream {
    /// 活跃的客户端连接
    clients: Arc<Mutex<Vec<String>>>,
}

impl SSEEventStream {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 添加客户端
    #[allow(dead_code)]
    pub fn add_client(&self, client_id: String) {
        let mut clients =
            crate::utils::lock_or_recover(self.clients.as_ref(), "SSEEventStream.clients");
        if !clients.contains(&client_id) {
            clients.push(client_id);
        }
    }

    /// 移除客户端
    #[allow(dead_code)]
    pub fn remove_client(&self, client_id: &str) {
        let mut clients =
            crate::utils::lock_or_recover(self.clients.as_ref(), "SSEEventStream.clients");
        clients.retain(|id| id != client_id);
    }

    /// 获取所有客户端
    #[allow(dead_code)]
    pub fn get_clients(&self) -> Vec<String> {
        let clients =
            crate::utils::lock_or_recover(self.clients.as_ref(), "SSEEventStream.clients");
        clients.clone()
    }

    /// 格式化 SSE 消息
    #[allow(dead_code)]
    pub fn format_sse_message(event: &ServiceEvent) -> String {
        // 将事件序列化为 JSON
        let json = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
        format!("data: {}\n\n", json)
    }
}

impl Default for SSEEventStream {
    fn default() -> Self {
        Self::new()
    }
}
