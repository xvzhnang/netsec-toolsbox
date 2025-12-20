/// SSE (Server-Sent Events) 处理器（用于前端事件推送）
use std::sync::{Arc, Mutex};
use crate::service::events::{ServiceEvent, EventListener};
use crate::service::websocket::SSEEventStream;

/// SSE 事件监听器（将事件转换为 SSE 格式并发送给前端）
pub struct SSEEventListener {
    stream: Arc<Mutex<SSEEventStream>>,
    client_id: String,
}

impl SSEEventListener {
    pub fn new(client_id: String) -> Self {
        Self {
            stream: Arc::new(Mutex::new(SSEEventStream::new())),
            client_id,
        }
    }

    pub fn get_stream(&self) -> Arc<Mutex<SSEEventStream>> {
        Arc::clone(&self.stream)
    }
}

impl EventListener for SSEEventListener {
    fn on_event(&self, event: &ServiceEvent) {
        // 将事件格式化为 SSE 消息
        let sse_message = SSEEventStream::format_sse_message(event);
        
        // 在实际实现中，这里应该通过 WebSocket 或 HTTP SSE 连接发送给前端
        // 目前先记录日志
        log::debug!("[SSE] 发送事件给客户端 {}: {}", self.client_id, sse_message);
    }
}

