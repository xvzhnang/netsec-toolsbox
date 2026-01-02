use crate::utils::Heartbeat;
/// ✅ 工程原则：循环改为 channel 驱动
/// 使用 channel 替代轮询，更高效、更可控
use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

/// Channel 驱动的循环消息类型
#[derive(Debug, Clone)]
pub enum LoopMessage {
    /// 执行一次检查/任务
    Tick,
}

/// Channel 驱动的循环控制器
///
/// # 优势
/// - 无消息时不消耗 CPU（channel recv 会阻塞）
/// - 支持精确控制（发送消息触发）
/// - 天然支持心跳（定期发送 Tick）
pub struct ChannelLoopController {
    tx: Sender<LoopMessage>,
    rx: Receiver<LoopMessage>,
    heartbeat: Arc<Heartbeat>,
}

impl ChannelLoopController {
    /// 创建新的 channel 循环控制器
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            tx,
            rx,
            heartbeat: Arc::new(Heartbeat::new()),
        }
    }

    /// 获取接收端（用于循环接收）
    pub fn receiver(&self) -> &Receiver<LoopMessage> {
        &self.rx
    }

    /// 获取心跳监控器
    pub fn heartbeat(&self) -> &Arc<Heartbeat> {
        &self.heartbeat
    }

    /// 发送 Tick 消息（触发一次检查）
    pub fn tick(&self) {
        let _ = self.tx.send(LoopMessage::Tick);
    }
}

impl Default for ChannelLoopController {
    fn default() -> Self {
        Self::new()
    }
}

/// Channel 驱动的循环模板
///
/// # 使用示例
/// ```rust
/// let controller = ChannelLoopController::new();
/// let heartbeat = controller.heartbeat().clone();
///
/// // 启动心跳发送线程
/// let tick_tx = controller.sender().clone();
/// std::thread::spawn(move || {
///     loop {
///         std::thread::sleep(Duration::from_secs(10));
///         if let Err(_) = tick_tx.send(LoopMessage::Tick) {
///             break; // 接收端已关闭
///         }
///     }
/// });
///
/// // 主循环
/// loop {
///     match controller.receiver().recv_timeout(Duration::from_secs(1)) {
///         Ok(LoopMessage::Tick) => {
///             heartbeat.ping();
///             do_work();
///         }
///         Ok(LoopMessage::Shutdown) => break,
///         Err(RecvTimeoutError::Timeout) => {
///             heartbeat.ping(); // 超时也算心跳
///         }
///         Err(RecvTimeoutError::Disconnected) => break,
///     }
/// }
/// ```
pub fn run_channel_loop<F>(controller: &ChannelLoopController, timeout: Duration, mut handler: F)
where
    F: FnMut(&LoopMessage) -> bool, // 返回 true 继续，false 退出
{
    let heartbeat = controller.heartbeat();

    loop {
        // ✅ 工程原则：channel recv 有 timeout，不会无限等待
        match controller.receiver().recv_timeout(timeout) {
            Ok(msg) => {
                heartbeat.ping();
                if !handler(&msg) {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                // ✅ 超时也算心跳（说明循环还在运行）
                heartbeat.ping();

                // 可以在这里执行定期任务
                if !handler(&LoopMessage::Tick) {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
}
