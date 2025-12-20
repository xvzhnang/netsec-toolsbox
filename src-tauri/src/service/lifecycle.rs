/// 生命周期钩子系统
use anyhow::Result;

/// 生命周期钩子 trait
pub trait LifecycleHooks: Send + Sync {
    /// 启动前钩子
    fn on_before_start(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 启动后钩子
    fn on_after_start(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 停止前钩子
    fn on_before_stop(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 停止后钩子
    fn on_after_stop(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 错误钩子
    fn on_error(&mut self, error: &anyhow::Error) -> Result<()> {
        log::error!("[Lifecycle] Service error: {}", error);
        Ok(())
    }
    
    /// 重启前钩子
    fn on_before_restart(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 重启后钩子
    fn on_after_restart(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 健康检查前钩子
    fn on_before_health_check(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 健康检查后钩子
    fn on_after_health_check(&mut self, _healthy: bool) -> Result<()> {
        Ok(())
    }
}

/// 默认生命周期钩子实现（空实现）
pub struct DefaultLifecycleHooks;

impl LifecycleHooks for DefaultLifecycleHooks {}

