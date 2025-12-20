/// GatewayPool 的 Service trait 实现包装器
use std::sync::{Arc, Mutex};
use anyhow::Result;
use log::{info, warn, error};

use crate::service::trait_def::{Service, HealthStatus};
use crate::service::state::ServiceState;
use crate::ai_service::pool::GatewayPool;

/// GatewayPool 的 Service 包装器
pub struct GatewayPoolService {
    id: String,
    name: String,
    state: ServiceState,
    pool: Arc<Mutex<Option<GatewayPool>>>,
}

impl GatewayPoolService {
    /// 创建新的 GatewayPoolService
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            state: ServiceState::Stopped,
            pool: Arc::new(Mutex::new(None)),
        }
    }

    /// 获取内部池（用于直接操作）
    pub fn get_pool(&self) -> Arc<Mutex<Option<GatewayPool>>> {
        Arc::clone(&self.pool)
    }
}

impl Service for GatewayPoolService {
    fn id(&self) -> &str {
        &self.id
    }

    fn state(&self) -> ServiceState {
        self.state
    }

    fn set_state_unchecked(&mut self, new_state: ServiceState) {
        self.state = new_state;
    }

    fn start(&mut self) -> Result<()> {
        info!("[GatewayPoolService] 启动服务: {}", self.id);
        self.set_state_unchecked(ServiceState::Starting);

        let result = {
            let mut pool_guard = self.pool.lock().unwrap();
            
            // 如果池已存在，直接启动
            if pool_guard.is_some() {
                let pool = pool_guard.as_ref().unwrap();
                pool.start_all()
            } else {
                // 创建新池
                let pool_size = 3;
                let base_port = 8765;
                let pool = GatewayPool::new(pool_size, base_port);
                
                // 启动所有 Worker
                let result = pool.start_all();
                if result.is_ok() {
                    info!("[GatewayPoolService] 连接池初始化成功");
                    
                    // 启动健康检查线程
                    pool.start_health_check_thread();
                    
                    *pool_guard = Some(pool);
                }
                result
            }
        };
        
        match result {
            Ok(_) => {
                self.set_state_unchecked(ServiceState::Idle);
                Ok(())
            }
            Err(e) => {
                error!("[GatewayPoolService] 启动失败: {}", e);
                self.set_state_unchecked(ServiceState::Unhealthy);
                anyhow::bail!("启动失败: {}", e)
            }
        }
    }

    fn stop(&mut self) -> Result<()> {
        info!("[GatewayPoolService] 停止服务: {}", self.id);
        self.set_state_unchecked(ServiceState::Stopping);

        let result = {
            let pool_guard = self.pool.lock().unwrap();
            if let Some(pool) = pool_guard.as_ref() {
                pool.stop_all().map(|_| ())
            } else {
                Ok(())
            }
        };
        
        match result {
            Ok(_) => {
                self.set_state_unchecked(ServiceState::Stopped);
                Ok(())
            }
            Err(e) => {
                error!("[GatewayPoolService] 停止失败: {}", e);
                anyhow::bail!("停止失败: {}", e)
            }
        }
    }

    fn health_check(&mut self) -> HealthStatus {
        let gateway_state = {
            let pool_guard = self.pool.lock().unwrap();
            if pool_guard.is_none() {
                return HealthStatus::Unhealthy;
            }
            pool_guard.as_ref().unwrap().get_gateway_state()
        };
        
        match gateway_state {
            crate::ai_service::pool::GatewayState::Healthy => {
                if self.state != ServiceState::Idle && self.state != ServiceState::Busy {
                    self.set_state_unchecked(ServiceState::Idle);
                }
                HealthStatus::Healthy
            }
            crate::ai_service::pool::GatewayState::Busy => {
                if self.state != ServiceState::Busy {
                    self.set_state_unchecked(ServiceState::Busy);
                }
                HealthStatus::Healthy
            }
            crate::ai_service::pool::GatewayState::Degraded => {
                if self.state != ServiceState::Degraded {
                    self.set_state_unchecked(ServiceState::Degraded);
                }
                HealthStatus::Degraded
            }
            crate::ai_service::pool::GatewayState::Unavailable => {
                if self.state != ServiceState::Unhealthy {
                    self.set_state_unchecked(ServiceState::Unhealthy);
                }
                HealthStatus::Unhealthy
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some("AI Gateway 连接池服务，管理多个 Worker 实例")
    }

    fn message(&self) -> Option<String> {
        let pool_guard = self.pool.lock().unwrap();
        if let Some(pool) = pool_guard.as_ref() {
            let workers = pool.get_workers();
            let mut healthy_count = 0;
            let mut idle_count = 0;
            let mut busy_count = 0;
            let mut degraded_count = 0;
            let mut unhealthy_count = 0;
            let mut total_active_requests = 0;
            let total_count = workers.len();
            
            for worker in workers {
                let wg = worker.lock().unwrap();
                let state = wg.status();
                if wg.is_healthy() {
                    healthy_count += 1;
                }
                total_active_requests += wg.active_requests() as u64;
                match state {
                    crate::ai_service::pool::WorkerState::Idle => idle_count += 1,
                    crate::ai_service::pool::WorkerState::BusyStreaming 
                    | crate::ai_service::pool::WorkerState::BusyBlocked => busy_count += 1,
                    crate::ai_service::pool::WorkerState::Degraded => degraded_count += 1,
                    crate::ai_service::pool::WorkerState::Unhealthy 
                    | crate::ai_service::pool::WorkerState::Dead => unhealthy_count += 1,
                    _ => {}
                }
            }
            
            if unhealthy_count > 0 {
                Some(format!("{}/{} Workers 健康 ({} 空闲, {} 忙碌, {} 降级, {} 异常) | 活跃请求: {}", 
                    healthy_count, total_count, idle_count, busy_count, degraded_count, unhealthy_count, total_active_requests))
            } else if total_active_requests > 0 {
                Some(format!("{}/{} Workers 健康 ({} 空闲, {} 忙碌, {} 降级) | 活跃请求: {}", 
                    healthy_count, total_count, idle_count, busy_count, degraded_count, total_active_requests))
            } else {
                Some(format!("{}/{} Workers 健康 ({} 空闲, {} 忙碌, {} 降级)", 
                    healthy_count, total_count, idle_count, busy_count, degraded_count))
            }
        } else {
            Some("连接池未初始化".to_string())
        }
    }
}

