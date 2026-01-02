use anyhow::Result;
use log::{error, info};
/// GatewayPool 的 Service trait 实现包装器
use std::sync::{Arc, Mutex};

use crate::ai_service::get_global_pool;
use crate::ai_service::pool::GatewayPool;
use crate::service::state::ServiceState;
use crate::service::trait_def::{HealthStatus, Service};

/// GatewayPool 的 Service 包装器
pub struct GatewayPoolService {
    id: String,
    name: String,
    state: ServiceState,
    pool: Arc<Mutex<GatewayPool>>,
    initialized: Arc<Mutex<bool>>,
}

impl GatewayPoolService {
    /// 创建新的 GatewayPoolService
    pub fn new(id: String, name: String) -> Self {
        let pool = get_global_pool();

        Self {
            id,
            name,
            state: ServiceState::Stopped,
            pool,
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// 获取内部池（用于直接操作）
    #[allow(dead_code)]
    pub fn get_pool(&self) -> Arc<Mutex<GatewayPool>> {
        Arc::clone(&self.pool)
    }
}

impl Service for GatewayPoolService {
    fn id(&self) -> &str {
        &self.id
    }

    fn priority(&self) -> u8 {
        80 // AI Gateway 高优先级
    }

    fn group(&self) -> Option<&str> {
        Some("core") // 核心服务
    }

    fn state(&self) -> ServiceState {
        self.state
    }

    fn set_state_unchecked(&mut self, new_state: ServiceState) {
        self.state = new_state;
    }

    fn start(&mut self) -> Result<()> {
        info!("[GatewayPoolService] 启动服务: {}", self.id);

        // 检查是否已经启动
        if self.state == ServiceState::Idle || self.state == ServiceState::Busy {
            info!("[GatewayPoolService] 服务已在运行，跳过重复启动");
            return Ok(());
        }

        // 检查是否已初始化（一次性初始化保护）
        let is_initialized = {
            let initialized = crate::utils::lock_or_recover(
                self.initialized.as_ref(),
                "GatewayPoolService.initialized",
            );
            *initialized
        };

        if is_initialized {
            info!("[GatewayPoolService] 服务已初始化，跳过重复启动");
            return Ok(());
        }

        let result = {
            let pool_guard =
                crate::utils::lock_or_recover(self.pool.as_ref(), "GatewayPoolService.pool");
            let workers = pool_guard.get_workers();

            // 检查所有 Worker 是否已启动
            let mut all_started = true;
            for worker in workers {
                let wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
                if wg.process.is_none()
                    && !matches!(
                        wg.status(),
                        crate::ai_service::pool::WorkerState::FailedPermanent
                            | crate::ai_service::pool::WorkerState::Disabled
                    )
                {
                    all_started = false;
                    break;
                }
            }

            if all_started {
                info!("[GatewayPoolService] 所有 Worker 已启动，跳过重复启动");
                Ok(vec!["Workers already started".to_string()])
            } else {
                // 启动所有 Worker（只启动一次）
                let result = pool_guard.start_all();
                if result.is_ok() {
                    info!("[GatewayPoolService] 连接池初始化成功");

                    // 启动健康检查线程（只启动一次）
                    pool_guard.start_health_check_thread();
                }
                result
            }
        };

        match result {
            Ok(_) => {
                *crate::utils::lock_or_recover(
                    self.initialized.as_ref(),
                    "GatewayPoolService.initialized",
                ) = true;
                Ok(())
            }
            Err(e) => {
                error!("[GatewayPoolService] 启动失败: {}", e);
                anyhow::bail!("启动失败: {}", e)
            }
        }
    }

    fn stop(&mut self) -> Result<()> {
        info!("[GatewayPoolService] 停止服务: {}", self.id);

        let result = {
            let pool_guard =
                crate::utils::lock_or_recover(self.pool.as_ref(), "GatewayPoolService.pool");
            pool_guard.stop_all().map(|_| ())
        };

        // 重置初始化标志（允许重新启动）
        *crate::utils::lock_or_recover(
            self.initialized.as_ref(),
            "GatewayPoolService.initialized",
        ) = false;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("[GatewayPoolService] 停止失败: {}", e);
                anyhow::bail!("停止失败: {}", e)
            }
        }
    }

    fn health_check(&mut self) -> HealthStatus {
        let gateway_state = {
            let pool_guard =
                crate::utils::lock_or_recover(self.pool.as_ref(), "GatewayPoolService.pool");
            pool_guard.get_gateway_state()
        };

        match gateway_state {
            crate::ai_service::pool::GatewayState::Healthy
            | crate::ai_service::pool::GatewayState::Busy => HealthStatus::Healthy,
            crate::ai_service::pool::GatewayState::Degraded => HealthStatus::Degraded,
            crate::ai_service::pool::GatewayState::Unavailable => HealthStatus::Unhealthy,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        Some("AI Gateway 连接池服务，管理多个 Worker 实例")
    }

    fn message(&self) -> Option<String> {
        let pool_guard =
            crate::utils::lock_or_recover(self.pool.as_ref(), "GatewayPoolService.pool");
        let workers = pool_guard.get_workers();
        let mut healthy_count = 0;
        let mut idle_count = 0;
        let mut busy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut total_active_requests = 0;
        let total_count = workers.len();

        for worker in workers {
            let wg = crate::utils::lock_or_recover(worker.as_ref(), "GatewayWorker");
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
                | crate::ai_service::pool::WorkerState::Dead
                | crate::ai_service::pool::WorkerState::FailedPermanent
                | crate::ai_service::pool::WorkerState::Disabled => unhealthy_count += 1,
                _ => {}
            }
        }

        if unhealthy_count > 0 {
            Some(format!(
                "{}/{} Workers 健康 ({} 空闲, {} 忙碌, {} 降级, {} 异常) | 活跃请求: {}",
                healthy_count,
                total_count,
                idle_count,
                busy_count,
                degraded_count,
                unhealthy_count,
                total_active_requests
            ))
        } else if total_active_requests > 0 {
            Some(format!(
                "{}/{} Workers 健康 ({} 空闲, {} 忙碌, {} 降级) | 活跃请求: {}",
                healthy_count,
                total_count,
                idle_count,
                busy_count,
                degraded_count,
                total_active_requests
            ))
        } else {
            Some(format!(
                "{}/{} Workers 健康 ({} 空闲, {} 忙碌, {} 降级)",
                healthy_count, total_count, idle_count, busy_count, degraded_count
            ))
        }
    }
}
