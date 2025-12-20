/// Service trait 定义（统一接口）
use crate::service::state::ServiceState;
use std::sync::{Arc, Mutex};

/// 健康状态（健康检查结果）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 降级（慢但可用）
    Degraded,
    /// 不健康（需要恢复）
    Unhealthy,
}

/// Service trait - 所有服务必须实现的统一接口
/// 
/// 为什么这样设计：
/// - ServiceManager 不关心实现细节
/// - AI 是 Python / Wiki 是 HTTP / Tool 是 exe → 无所谓
/// - 统一的生命周期管理
pub trait Service: Send + Sync {
    /// 获取服务优先级（0-100，数字越大优先级越高）
    fn priority(&self) -> u8 {
        50 // 默认中等优先级
    }
    
    /// 获取服务分组（用于分组管理）
    fn group(&self) -> Option<&str> {
        None
    }
    /// 获取服务 ID（唯一标识）
    fn id(&self) -> &str;
    
    /// 获取当前状态
    fn state(&self) -> ServiceState;
    
    /// 设置状态（带合法性校验）
    fn set_state(&mut self, new_state: ServiceState) -> Result<(), String> {
        let current = self.state();
        if !ServiceState::can_transit(current, new_state) {
            return Err(format!(
                "非法状态转换: {} -> {}",
                current, new_state
            ));
        }
        self.set_state_unchecked(new_state);
        Ok(())
    }
    
    /// 设置状态（不校验，内部使用）
    fn set_state_unchecked(&mut self, new_state: ServiceState);
    
    /// 启动服务
    fn start(&mut self) -> anyhow::Result<()>;
    
    /// 停止服务
    fn stop(&mut self) -> anyhow::Result<()>;
    
    /// 健康检查
    fn health_check(&mut self) -> HealthStatus;
    
    /// 获取服务名称（用于显示）
    fn name(&self) -> &str {
        self.id()
    }
    
    /// 获取服务描述
    fn description(&self) -> Option<&str> {
        None
    }
    
    /// 获取服务消息（当前状态说明）
    fn message(&self) -> Option<String> {
        None
    }
}

/// Service 的 Arc<Mutex<dyn Service>> 类型别名
pub type ServiceHandle = Arc<Mutex<dyn Service>>;

