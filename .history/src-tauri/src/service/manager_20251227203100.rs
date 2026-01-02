use crate::utils::{ChannelLoopController, LoopMessage};
use log::{error, info, warn};
use parking_lot::Mutex;
/// ServiceManager - 统一的服务管理器
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;
use std::time::{Duration, Instant};

use crate::service::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::service::dto::{ServiceStatusDTO, ServiceStatusListDTO};
use crate::service::events::{current_timestamp, EventBus, ServiceEvent};
use crate::service::metrics::MetricsCollector;
use crate::service::state::ServiceState;
use crate::service::state_machine::get_state_machine;
use crate::service::trait_def::{HealthStatus, ServiceHandle};
use crate::service::watchdog::Watchdog;

#[derive(Clone)]
struct RestartPolicy {
    max_restarts: usize,
    window: Duration,
    base_backoff: Duration,
    max_backoff: Duration,
    grace_period: Duration,
    degraded_to_dead: Duration,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 3,
            window: Duration::from_secs(300),
            base_backoff: Duration::from_secs(5),
            max_backoff: Duration::from_secs(120),
            grace_period: Duration::from_secs(30),
            degraded_to_dead: Duration::from_secs(60),
        }
    }
}

impl RestartPolicy {
    fn can_restart(&self, history: &mut Vec<Instant>, now: Instant) -> Option<Duration> {
        history.retain(|t| now.duration_since(*t) < self.window);
        if history.len() >= self.max_restarts {
            return None;
        }
        let exp = history.len() as u32;
        let multiplier = 1u32.checked_shl(exp).unwrap_or(u32::MAX);
        let backoff = self.base_backoff.saturating_mul(multiplier);
        history.push(now);
        Some(backoff.min(self.max_backoff))
    }
}

#[derive(Default, Clone)]
struct RecoveryState {
    starting_since: Option<Instant>,
    degraded_since: Option<Instant>,
    dead_since: Option<Instant>,
    backoff_until: Option<Instant>,
    restart_history: Vec<Instant>,
    restart_in_progress: bool,
    paused: bool,
}

/// 服务管理器（统一管理所有服务）
#[derive(Clone)]
pub struct ServiceManager {
    /// 所有注册的服务（使用 std::sync::Mutex，因为 ServiceHandle 是 Arc<Mutex<dyn Service>>）
    services: Arc<StdMutex<HashMap<String, ServiceHandle>>>,
    /// 监控线程是否运行（使用 parking_lot::Mutex）
    monitoring: Arc<Mutex<bool>>,
    /// 事件总线（使用 parking_lot::Mutex）
    event_bus: Arc<Mutex<EventBus>>,
    /// 服务熔断器（按服务 ID）（使用 parking_lot::Mutex）
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    /// 指标收集器（使用 std::sync::Mutex）
    metrics: Arc<StdMutex<MetricsCollector>>,
    restart_policy: RestartPolicy,
    /// 恢复状态（使用 parking_lot::Mutex）
    recovery: Arc<Mutex<HashMap<String, RecoveryState>>>,
    /// ✅ 工程原则：Channel 驱动的监控循环
    monitoring_controller: Option<Arc<crate::utils::ChannelLoopController>>,
    /// ✅ 工程原则：Watchdog 自动恢复机制（使用 parking_lot::Mutex）
    watchdog: Arc<Mutex<Option<Watchdog>>>,
}

impl ServiceManager {
    /// 创建新的服务管理器
    pub fn new() -> Self {
        Self {
            services: Arc::new(StdMutex::new(HashMap::new())),
            monitoring: Arc::new(Mutex::new(false)),
            event_bus: Arc::new(Mutex::new(EventBus::new())),
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(StdMutex::new(MetricsCollector::new())),
            restart_policy: RestartPolicy::default(),
            recovery: Arc::new(Mutex::new(HashMap::new())),
            monitoring_controller: Some(Arc::new(ChannelLoopController::new())),
            watchdog: Arc::new(Mutex::new(Some(Watchdog::new()))),
        }
    }

    /// 获取 Watchdog（用于监控面板）
    pub fn watchdog(&self) -> Arc<parking_lot::Mutex<Option<Watchdog>>> {
        Arc::clone(&self.watchdog)
    }

    /// 获取指标收集器
    pub fn metrics(&self) -> Arc<std::sync::Mutex<MetricsCollector>> {
        Arc::clone(&self.metrics)
    }

    /// 获取 Prometheus 格式的指标
    pub fn get_prometheus_metrics(&self) -> String {
        let metrics =
            crate::utils::lock_or_recover_std(self.metrics.as_ref(), "ServiceManager.metrics");
        metrics.to_prometheus_format()
    }

    /// 获取事件总线（用于订阅事件）
    #[allow(dead_code)]
    pub fn event_bus(&self) -> Arc<parking_lot::Mutex<EventBus>> {
        Arc::clone(&self.event_bus)
    }

    /// ✅ 1️⃣ 关键保护：检查服务是否正在运行（幂等性检查）
    pub fn is_running(&self, id: &str) -> bool {
        // ✅ 关键优化：缩小锁范围
        let service = {
            let services = crate::utils::lock_or_recover_std(
                self.services.as_ref(),
                "ServiceManager.services",
            );
            services.get(id).cloned()
        };

        if let Some(service) = service {
            let state = {
                let s = crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
                s.state()
            };
            matches!(
                state,
                ServiceState::Idle
                    | ServiceState::Busy
                    | ServiceState::Degraded
                    | ServiceState::Warmup
            )
        } else {
            false
        }
    }

    /// 发送事件
    fn emit_event(&self, event: ServiceEvent) {
        let bus =
            crate::utils::lock_or_recover(self.event_bus.as_ref(), "ServiceManager.event_bus");
        bus.emit(&event);
    }

    /// 注册服务
    pub fn register(&self, service: ServiceHandle) -> Result<(), String> {
        let mut services =
            crate::utils::lock_or_recover_std(self.services.as_ref(), "ServiceManager.services");
        let id = {
            let s = crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
            s.id().to_string()
        };

        if services.contains_key(&id) {
            return Err(format!("服务 {} 已存在", id));
        }

        info!("[ServiceManager] 注册服务: {}", id);

        // 为服务创建熔断器
        let mut breakers = crate::utils::lock_or_recover(
            self.circuit_breakers.as_ref(),
            "ServiceManager.circuit_breakers",
        );
        breakers.insert(
            id.clone(),
            CircuitBreaker::new(CircuitBreakerConfig::default()),
        );
        drop(breakers);

        services.insert(id.clone(), service);
        {
            let mut recovery =
                crate::utils::lock_or_recover(self.recovery.as_ref(), "ServiceManager.recovery");
            recovery.insert(id.clone(), RecoveryState::default());
        }

        // 发送注册事件
        self.emit_event(ServiceEvent::Started {
            service_id: id,
            timestamp: current_timestamp(),
        });

        Ok(())
    }

    /// 注销服务
    pub fn unregister(&self, id: &str) -> Result<(), String> {
        let mut services =
            crate::utils::lock_or_recover_std(self.services.as_ref(), "ServiceManager.services");
        if let Some(service) = services.remove(id) {
            info!("[ServiceManager] 注销服务: {}", id);
            // 尝试停止服务
            let mut service_guard =
                crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
            if let Err(e) = service_guard.stop() {
                warn!("[ServiceManager] 停止服务 {} 失败: {}", id, e);
            }
            {
                let mut recovery = crate::utils::lock_or_recover(
                    self.recovery.as_ref(),
                    "ServiceManager.recovery",
                );
                recovery.remove(id);
            }
            Ok(())
        } else {
            Err(format!("服务 {} 不存在", id))
        }
    }

    /// 获取服务
    #[allow(dead_code)]
    pub fn get_service(&self, id: &str) -> Option<ServiceHandle> {
        let services =
            crate::utils::lock_or_recover_std(self.services.as_ref(), "ServiceManager.services");
        services.get(id).map(Arc::clone)
    }

    /// 获取所有服务状态
    pub fn get_all_status(&self) -> ServiceStatusListDTO {
        // ✅ 关键优化：收集 Handle 后立即释放 Map 锁
        let handles: Vec<ServiceHandle> = {
            let services = crate::utils::lock_or_recover_std(
                self.services.as_ref(),
                "ServiceManager.services",
            );
            services.values().cloned().collect()
        };

        let mut status_list = Vec::new();

        for service in handles {
            let service_guard =
                crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
            let dto = ServiceStatusDTO::from_service(&*service_guard);
            status_list.push(dto);
        }

        // 按状态排序：健康优先，错误最后
        status_list.sort_by(|a, b| {
            let a_priority = match a.state {
                ServiceState::Idle | ServiceState::Busy => 0,
                ServiceState::Degraded => 1,
                ServiceState::Starting | ServiceState::Warmup | ServiceState::Stopping => 2,
                ServiceState::Unhealthy | ServiceState::Restarting => 3,
                ServiceState::Failed => 4,
                ServiceState::Stopped => 5,
            };
            let b_priority = match b.state {
                ServiceState::Idle | ServiceState::Busy => 0,
                ServiceState::Degraded => 1,
                ServiceState::Starting | ServiceState::Warmup | ServiceState::Stopping => 2,
                ServiceState::Unhealthy | ServiceState::Restarting => 3,
                ServiceState::Failed => 4,
                ServiceState::Stopped => 5,
            };
            a_priority.cmp(&b_priority)
        });

        ServiceStatusListDTO {
            services: status_list,
        }
    }

    /// 获取单个服务状态
    pub fn get_status(&self, id: &str) -> Option<ServiceStatusDTO> {
        let services =
            crate::utils::lock_or_recover_std(self.services.as_ref(), "ServiceManager.services");
        services.get(id).map(|service| {
            let service_guard =
                crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
            ServiceStatusDTO::from_service(&*service_guard)
        })
    }

    /// 启动服务
    pub fn start_service(&self, id: &str) -> Result<(), String> {
        // 检查熔断器
        let breakers = crate::utils::lock_or_recover(
            self.circuit_breakers.as_ref(),
            "ServiceManager.circuit_breakers",
        );
        if let Some(breaker) = breakers.get(id) {
            if !breaker.can_execute() {
                return Err(format!("服务 {} 处于熔断状态，无法启动", id));
            }
        }
        drop(breakers);

        // ✅ 关键优化：缩小锁范围，获取 Handle 后立即释放 Map 锁
        let service = {
            let services = crate::utils::lock_or_recover_std(
                self.services.as_ref(),
                "ServiceManager.services",
            );
            services
                .get(id)
                .cloned()
                .ok_or_else(|| format!("服务 {} 不存在", id))?
        };

        // ✅ 1️⃣ 关键保护：启动幂等性检查（必须）
        // 检查服务是否已在运行，避免重复启动
        if self.is_running(id) {
            info!(
                "[ServiceManager] 服务 {} 已在运行，跳过启动（幂等性保护）",
                id
            );
            return Ok(());
        }

        let from_state = {
            let s = crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
            s.state()
        };

        // ✅ 工程原则：使用状态机验证状态转换
        let state_machine = get_state_machine();
        if !state_machine.can_transit(from_state, ServiceState::Starting) {
            return Err(format!(
                "服务 {} 状态转换不合法: {:?} -> Starting",
                id, from_state
            ));
        }

        let mut service_guard =
            crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
        info!("[ServiceManager] 启动服务: {}", id);

        {
            let mut recovery =
                crate::utils::lock_or_recover(self.recovery.as_ref(), "ServiceManager.recovery");
            let entry = recovery.entry(id.to_string()).or_default();
            entry.starting_since = Some(Instant::now());
            entry.degraded_since = None;
            entry.dead_since = None;
            entry.backoff_until = None;
            entry.restart_in_progress = false;
            entry.paused = false;
            entry.restart_history.clear();
        }

        service_guard
            .set_state(ServiceState::Starting)
            .unwrap_or_else(|e| {
                warn!("[ServiceManager] 设置服务 {} 状态失败: {}", id, e);
            });

        // 发送状态变化事件
        self.emit_event(ServiceEvent::StateChanged {
            service_id: id.to_string(),
            from: from_state,
            to: ServiceState::Starting,
            timestamp: current_timestamp(),
        });

        match service_guard.start() {
            Ok(_) => {
                let to_state = ServiceState::Idle;
                service_guard.set_state(to_state).unwrap_or_else(|e| {
                    warn!("[ServiceManager] 设置服务 {} 状态失败: {}", id, e);
                });

                // 记录成功，重置熔断器
                let mut breakers = crate::utils::lock_or_recover(
                    self.circuit_breakers.as_ref(),
                    "ServiceManager.circuit_breakers",
                );
                if let Some(breaker) = breakers.get_mut(id) {
                    breaker.record_success();
                }
                drop(breakers);

                // 记录指标
                {
                    let metrics = crate::utils::lock_or_recover_std(
                        self.metrics.as_ref(),
                        "ServiceManager.metrics",
                    );
                    metrics.record_start(id);
                    metrics.record_state_change(id);
                }

                // 发送事件
                self.emit_event(ServiceEvent::StateChanged {
                    service_id: id.to_string(),
                    from: ServiceState::Starting,
                    to: to_state,
                    timestamp: current_timestamp(),
                });
                self.emit_event(ServiceEvent::Started {
                    service_id: id.to_string(),
                    timestamp: current_timestamp(),
                });

                Ok(())
            }
            Err(e) => {
                error!("[ServiceManager] 启动服务 {} 失败: {}", id, e);
                let to_state = if service_guard.state() == ServiceState::Failed {
                    ServiceState::Failed
                } else {
                    ServiceState::Unhealthy
                };
                service_guard.set_state_unchecked(to_state);

                // 记录失败
                let mut breakers = crate::utils::lock_or_recover(
                    self.circuit_breakers.as_ref(),
                    "ServiceManager.circuit_breakers",
                );
                if let Some(breaker) = breakers.get_mut(id) {
                    breaker.record_failure();
                }
                drop(breakers);

                // 记录指标
                {
                    let metrics = crate::utils::lock_or_recover_std(
                        self.metrics.as_ref(),
                        "ServiceManager.metrics",
                    );
                    metrics.record_error(id, format!("启动失败: {}", e));
                    metrics.record_state_change(id);
                }

                // 发送事件
                self.emit_event(ServiceEvent::StateChanged {
                    service_id: id.to_string(),
                    from: ServiceState::Starting,
                    to: to_state,
                    timestamp: current_timestamp(),
                });
                self.emit_event(ServiceEvent::Error {
                    service_id: id.to_string(),
                    error: format!("启动失败: {}", e),
                    timestamp: current_timestamp(),
                });

                Err(format!("启动失败: {}", e))
            }
        }
    }

    /// 停止服务
    pub fn stop_service(&self, id: &str) -> Result<(), String> {
        // ✅ 关键优化：缩小锁范围
        let service = {
            let services = crate::utils::lock_or_recover_std(
                self.services.as_ref(),
                "ServiceManager.services",
            );
            services
                .get(id)
                .cloned()
                .ok_or_else(|| format!("服务 {} 不存在", id))?
        };

        let mut service_guard =
            crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
        info!("[ServiceManager] 停止服务: {}", id);

        {
            let mut recovery =
                crate::utils::lock_or_recover(self.recovery.as_ref(), "ServiceManager.recovery");
            if let Some(entry) = recovery.get_mut(id) {
                entry.starting_since = None;
                entry.degraded_since = None;
                entry.dead_since = None;
                entry.backoff_until = None;
                entry.restart_in_progress = false;
                entry.paused = false;
                entry.restart_history.clear();
            }
        }

        service_guard
            .set_state(ServiceState::Stopping)
            .unwrap_or_else(|e| {
                warn!("[ServiceManager] 设置服务 {} 状态失败: {}", id, e);
            });

        match service_guard.stop() {
            Ok(_) => {
                service_guard.set_state_unchecked(ServiceState::Stopped);
                Ok(())
            }
            Err(e) => {
                error!("[ServiceManager] 停止服务 {} 失败: {}", id, e);
                Err(format!("停止失败: {}", e))
            }
        }
    }

    /// 重启服务
    pub fn restart_service(&self, id: &str) -> Result<(), String> {
        info!("[ServiceManager] 重启服务: {}", id);
        self.stop_service(id)?;
        // 优化：减少固定等待时间，使用更短的延迟
        thread::sleep(Duration::from_millis(200));
        self.start_service(id)
    }

    /// ✅ 工程原则：启动监控循环
    pub fn start_monitoring(&self) {
        let mut monitoring =
            crate::utils::lock_or_recover(self.monitoring.as_ref(), "ServiceManager.monitoring");
        if *monitoring {
            return;
        }
        *monitoring = true;
        drop(monitoring);

        let controller = self
            .monitoring_controller
            .as_ref()
            .map(Arc::clone)
            .unwrap_or_else(|| Arc::new(ChannelLoopController::new()));

        let services = Arc::clone(&self.services);
        let monitoring_flag = Arc::clone(&self.monitoring);
        let metrics = Arc::clone(&self.metrics);
        let event_bus = Arc::clone(&self.event_bus);
        let recovery = Arc::clone(&self.recovery);
        let restart_policy = self.restart_policy.clone();
        let controller_clone = Arc::clone(&controller);

        while controller.receiver().try_recv().is_ok() {}

        {
            let watchdog = crate::utils::lock_or_recover(
                self.watchdog.as_ref(),
                "ServiceManager.watchdog",
            );
            if let Some(wd) = watchdog.as_ref() {
                wd.register_thread(
                    "ServiceManager.monitoring".to_string(),
                    controller.heartbeat().clone(),
                );
            }
        }

        thread::spawn(move || {
            // 关键修复：使用 catch_unwind 捕获 panic，避免监控线程崩溃导致服务管理失效
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                info!("[ServiceManager] 监控线程已启动");

                crate::utils::run_channel_loop(
                    controller_clone.as_ref(),
                    Duration::from_secs(10),
                    |_| {
                        let continue_flag = {
                            let flag = crate::utils::lock::try_lock_or_timeout(
                                monitoring_flag.as_ref(),
                                "ServiceManager.monitoring",
                                std::time::Duration::from_millis(50),
                            );
                            flag.map(|f| *f).unwrap_or(false)
                        };

                        if !continue_flag {
                            info!("[ServiceManager] 监控线程已停止");
                            return false;
                        }

                        let tick_start = Instant::now();
                        let now = Instant::now();
                        let mut restarts_due: Vec<(String, ServiceHandle)> = Vec::new();
                        let mut stops_due: Vec<(String, ServiceHandle)> = Vec::new();

                        // ✅ 关键修复：先收集需要处理的服务ID，避免长时间持有 services 锁
                        let service_ids: Vec<(String, ServiceHandle)> = {
                            let services_guard = crate::utils::lock_or_recover_std(
                                services.as_ref(),
                                "ServiceManager.services",
                            );
                            services_guard
                                .iter()
                                .map(|(id, service)| (id.clone(), Arc::clone(service)))
                                .collect()
                        };

                    // ✅ 关键修复：在锁外处理每个服务，避免死锁和长时间持有锁
                    for (id, service) in service_ids {
                        let mut service_guard =
                            crate::utils::lock_or_recover_std(service.as_ref(), "ServiceHandle");
                        let current_state = service_guard.state();

                        // 跳过停止/停止中状态的服务
                        if current_state == ServiceState::Stopped
                            || current_state == ServiceState::Stopping
                            || current_state == ServiceState::Failed
                        {
                            continue;
                        }

                        // ✅ 关键修复：快速检查恢复状态，立即释放锁
                        let (should_skip, needs_restart, needs_start_tracking) = {
                            let mut recovery_guard = crate::utils::lock_or_recover(
                                recovery.as_ref(),
                                "ServiceManager.recovery",
                            );
                            let entry = recovery_guard.entry(id.clone()).or_default();
                            if entry.paused || entry.restart_in_progress {
                                (true, false, false)
                            } else {
                                let needs_start_tracking = current_state == ServiceState::Starting
                                    && entry.starting_since.is_none();
                                if needs_start_tracking {
                                    entry.starting_since = Some(now);
                                }
                                let needs_restart = if let Some(until) = entry.backoff_until {
                                    if now >= until && entry.dead_since.is_some() {
                                        entry.restart_in_progress = true;
                                        entry.backoff_until = None;
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                };
                                (false, needs_restart, needs_start_tracking)
                            }
                        };

                        if should_skip {
                            continue;
                        }

                        if needs_restart {
                            restarts_due.push((id.clone(), Arc::clone(&service)));
                            continue;
                        }

                        // 执行健康检查
                        let health_result = service_guard.health_check();
                        let current_state = service_guard.state();

                        // ✅ 关键修复：快速记录指标，立即释放锁
                        {
                            let metrics_guard = crate::utils::lock_or_recover_std(
                                metrics.as_ref(),
                                "ServiceManager.metrics",
                            );
                            let is_healthy = matches!(health_result, HealthStatus::Healthy);
                            metrics_guard.record_health_check(&id, is_healthy);
                        }

                        // ✅ 关键修复：快速发送事件，立即释放锁
                        let health_event_status = match health_result {
                            HealthStatus::Healthy => {
                                crate::service::events::HealthCheckResult::Healthy
                            }
                            HealthStatus::Degraded => {
                                crate::service::events::HealthCheckResult::Degraded
                            }
                            HealthStatus::Unhealthy => {
                                crate::service::events::HealthCheckResult::Unhealthy
                            }
                        };

                        {
                            let bus = crate::utils::lock_or_recover(
                                event_bus.as_ref(),
                                "ServiceManager.event_bus",
                            );
                            bus.emit(&crate::service::events::ServiceEvent::HealthCheck {
                                service_id: id.clone(),
                                status: health_event_status,
                                timestamp: crate::service::events::current_timestamp(),
                            });
                        }

                        let mut state_change: Option<(ServiceState, ServiceState)> = None;
                        let mut schedule_restart: Option<Duration> = None;
                        let mut pause_service = false;

                        // ✅ 关键修复：快速检查恢复状态，立即释放锁
                        let (in_grace, is_paused) = {
                            let mut recovery_guard = crate::utils::lock_or_recover(
                                recovery.as_ref(),
                                "ServiceManager.recovery",
                            );
                            let entry = recovery_guard.entry(id.clone()).or_default();
                            if entry.paused {
                                (false, true)
                            } else {
                                let in_grace = entry
                                    .starting_since
                                    .map(|since| {
                                        now.duration_since(since) < restart_policy.grace_period
                                    })
                                    .unwrap_or(false);
                                (in_grace, false)
                            }
                        };

                        if is_paused {
                            continue;
                        }

                        match health_result {
                            HealthStatus::Healthy => {
                                {
                                    let mut recovery_guard = crate::utils::lock_or_recover(
                                        recovery.as_ref(),
                                        "ServiceManager.recovery",
                                    );
                                    let entry = recovery_guard.entry(id.clone()).or_default();
                                    entry.degraded_since = None;
                                    entry.dead_since = None;
                                    entry.starting_since = None;
                                }
                                if matches!(
                                    current_state,
                                    ServiceState::Starting
                                        | ServiceState::Degraded
                                        | ServiceState::Unhealthy
                                        | ServiceState::Restarting
                                ) {
                                    state_change = Some((current_state, ServiceState::Idle));
                                }
                            }
                            HealthStatus::Degraded | HealthStatus::Unhealthy => {
                                if in_grace && current_state == ServiceState::Starting {
                                    warn!(
                                        "[ServiceManager] 服务 {} 健康异常({:?})，仍在宽限期内",
                                        id, health_result
                                    );
                                } else {
                                    {
                                        let mut recovery_guard = crate::utils::lock_or_recover(
                                            recovery.as_ref(),
                                            "ServiceManager.recovery",
                                        );
                                        let entry = recovery_guard.entry(id.clone()).or_default();
                                        if current_state != ServiceState::Degraded
                                            && current_state != ServiceState::Unhealthy
                                            && current_state != ServiceState::Restarting
                                        {
                                            state_change =
                                                Some((current_state, ServiceState::Degraded));
                                        }
                                        if entry.degraded_since.is_none() {
                                            entry.degraded_since = Some(now);
                                        }
                                        entry.starting_since = None;
                                        if entry
                                            .degraded_since
                                            .map(|since| {
                                                now.duration_since(since)
                                                    >= restart_policy.degraded_to_dead
                                            })
                                            .unwrap_or(false)
                                        {
                                            entry.dead_since.get_or_insert(now);
                                            if current_state != ServiceState::Unhealthy {
                                                state_change =
                                                    Some((current_state, ServiceState::Unhealthy));
                                            }
                                        }
                                    }

                                    {
                                        let mut recovery_guard = crate::utils::lock_or_recover(
                                            recovery.as_ref(),
                                            "ServiceManager.recovery",
                                        );
                                        let entry = recovery_guard.entry(id.clone()).or_default();
                                        if entry.dead_since.is_some() {
                                            match restart_policy
                                                .can_restart(&mut entry.restart_history, now)
                                            {
                                                Some(delay) => {
                                                    schedule_restart = Some(delay);
                                                }
                                                None => {
                                                    pause_service = true;
                                                    entry.paused = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some((from, to)) = state_change {
                            if let Err(e) = service_guard.set_state(to) {
                                warn!("[ServiceManager] 服务 {} 状态切换失败: {}", id, e);
                            } else {
                                {
                                    let metrics_guard: std::sync::MutexGuard<'_, MetricsCollector> =
                                        crate::utils::lock_or_recover_std(
                                            metrics.as_ref(),
                                            "ServiceManager.metrics",
                                        );
                                    metrics_guard.record_state_change(id.as_str());
                                }
                                let bus = crate::utils::lock_or_recover(
                                    event_bus.as_ref(),
                                    "ServiceManager.event_bus",
                                );
                                bus.emit(&crate::service::events::ServiceEvent::StateChanged {
                                    service_id: id.clone(),
                                    from,
                                    to,
                                    timestamp: crate::service::events::current_timestamp(),
                                });
                            }
                        }

                        if pause_service {
                            warn!("[ServiceManager] 服务 {} 触发重启熔断，进入保护停机", id);
                            service_guard.set_state_unchecked(ServiceState::Stopped);
                            {
                                let mut recovery_guard = crate::utils::lock_or_recover(
                                    recovery.as_ref(),
                                    "ServiceManager.recovery",
                                );
                                let entry = recovery_guard.entry(id.clone()).or_default();
                                entry.restart_in_progress = true;
                            }
                            let bus = crate::utils::lock_or_recover(
                                event_bus.as_ref(),
                                "ServiceManager.event_bus",
                            );
                            bus.emit(&crate::service::events::ServiceEvent::Error {
                                service_id: id.clone(),
                                error: "重启熔断触发，服务已暂停".to_string(),
                                timestamp: crate::service::events::current_timestamp(),
                            });
                            stops_due.push((id.clone(), Arc::clone(&service)));
                            continue;
                        }

                        if let Some(delay) = schedule_restart {
                            let until = now + delay;
                            {
                                let mut recovery_guard = crate::utils::lock_or_recover(
                                    recovery.as_ref(),
                                    "ServiceManager.recovery",
                                );
                                let entry = recovery_guard.entry(id.clone()).or_default();
                                entry.backoff_until = Some(until);
                            }
                            if current_state != ServiceState::Restarting {
                                service_guard.set_state_unchecked(ServiceState::Restarting);
                                let bus = crate::utils::lock_or_recover(
                                    event_bus.as_ref(),
                                    "ServiceManager.event_bus",
                                );
                                bus.emit(&crate::service::events::ServiceEvent::StateChanged {
                                    service_id: id.clone(),
                                    from: current_state,
                                    to: ServiceState::Restarting,
                                    timestamp: crate::service::events::current_timestamp(),
                                });
                            }
                            warn!("[ServiceManager] 服务 {} 允许重启，退避 {:?}", id, delay);
                        }
                    }

                    for (id, service) in stops_due {
                        let service_clone = service;
                        let recovery_clone = Arc::clone(&recovery);
                        let event_bus_clone = Arc::clone(&event_bus);
                        thread::spawn(move || {
                            let mut s = crate::utils::lock_or_recover_std(
                                service_clone.as_ref(),
                                "ServiceHandle",
                            );
                            let from = s.state();
                            s.set_state_unchecked(ServiceState::Stopping);
                            if let Err(e) = s.stop() {
                                error!("[ServiceManager] 服务 {} stop 失败: {}", id, e);
                            }
                            s.set_state_unchecked(ServiceState::Stopped);
                            let bus = crate::utils::lock_or_recover(
                                event_bus_clone.as_ref(),
                                "ServiceManager.event_bus",
                            );
                            bus.emit(&ServiceEvent::StateChanged {
                                service_id: id.clone(),
                                from,
                                to: ServiceState::Stopped,
                                timestamp: current_timestamp(),
                            });
                            bus.emit(&ServiceEvent::Stopped {
                                service_id: id.clone(),
                                timestamp: current_timestamp(),
                            });
                            let mut recovery_guard = crate::utils::lock_or_recover(
                                recovery_clone.as_ref(),
                                "ServiceManager.recovery",
                            );
                            if let Some(entry) = recovery_guard.get_mut(&id) {
                                entry.restart_in_progress = false;
                            }
                        });
                    }

                    for (id, service) in restarts_due {
                        let service_clone = service;
                        let recovery_clone = Arc::clone(&recovery);
                        let event_bus_clone = Arc::clone(&event_bus);
                        let metrics_clone: Arc<StdMutex<MetricsCollector>> = Arc::clone(&metrics);
                        thread::spawn(move || {
                            let mut s = crate::utils::lock_or_recover_std(
                                service_clone.as_ref(),
                                "ServiceHandle",
                            );
                            let from = s.state();

                            s.set_state_unchecked(ServiceState::Stopping);
                            if let Err(e) = s.stop() {
                                error!("[ServiceManager] 服务 {} stop 失败: {}", id, e);
                            }

                            thread::sleep(Duration::from_millis(1000));

                            s.set_state_unchecked(ServiceState::Starting);
                            let start_result = s.start();
                            match start_result {
                                Ok(_) => {
                                    s.set_state_unchecked(ServiceState::Idle);
                                    {
                                        let metrics_guard: std::sync::MutexGuard<
                                            '_,
                                            MetricsCollector,
                                        > = crate::utils::lock_or_recover_std(
                                            metrics_clone.as_ref(),
                                            "ServiceManager.metrics",
                                        );
                                        metrics_guard.record_state_change(&id);
                                        metrics_guard.record_restart(&id);
                                    }
                                    let bus = crate::utils::lock_or_recover(
                                        event_bus_clone.as_ref(),
                                        "ServiceManager.event_bus",
                                    );
                                    bus.emit(&ServiceEvent::StateChanged {
                                        service_id: id.clone(),
                                        from,
                                        to: ServiceState::Idle,
                                        timestamp: current_timestamp(),
                                    });
                                    bus.emit(&ServiceEvent::Restarted {
                                        service_id: id.clone(),
                                        timestamp: current_timestamp(),
                                    });
                                    let mut recovery_guard = crate::utils::lock_or_recover(
                                        recovery_clone.as_ref(),
                                        "ServiceManager.recovery",
                                    );
                                    if let Some(entry) = recovery_guard.get_mut(&id) {
                                        entry.starting_since = None;
                                        entry.degraded_since = None;
                                        entry.dead_since = None;
                                        entry.restart_in_progress = false;
                                    }
                                }
                                Err(e) => {
                                    error!("[ServiceManager] 服务 {} restart 失败: {}", id, e);
                                    let to_state = if s.state() == ServiceState::Failed {
                                        ServiceState::Failed
                                    } else {
                                        ServiceState::Unhealthy
                                    };
                                    s.set_state_unchecked(to_state);
                                    let bus = crate::utils::lock_or_recover(
                                        event_bus_clone.as_ref(),
                                        "ServiceManager.event_bus",
                                    );
                                    bus.emit(&ServiceEvent::Error {
                                        service_id: id.clone(),
                                        error: format!("重启失败: {}", e),
                                        timestamp: current_timestamp(),
                                    });
                                    let mut recovery_guard = crate::utils::lock_or_recover(
                                        recovery_clone.as_ref(),
                                        "ServiceManager.recovery",
                                    );
                                    if let Some(entry) = recovery_guard.get_mut(&id) {
                                        entry.dead_since.get_or_insert(Instant::now());
                                        entry.restart_in_progress = false;
                                    }
                                }
                            }
                        });
                    }

                        let elapsed = tick_start.elapsed();
                        if elapsed > Duration::from_secs(30) {
                            warn!(
                                "[ServiceManager] 监控周期耗时过长: {:?}，可能存在阻塞",
                                elapsed
                            );
                        }

                        true
                    },
                );
            }));

            // 关键修复：如果监控线程 panic，记录但不影响主进程
            if let Err(panic_info) = result {
                error!("[ServiceManager] 监控线程 panic: {:?}", panic_info);
                // 重置监控标志，允许重新启动监控
                let mut flag = crate::utils::lock_or_recover(
                    monitoring_flag.as_ref(),
                    "ServiceManager.monitoring",
                );
                *flag = false;
            }
        });
    }

    /// 停止监控
    pub fn stop_monitoring(&self) {
        let mut monitoring =
            crate::utils::lock_or_recover(self.monitoring.as_ref(), "ServiceManager.monitoring");
        *monitoring = false;
        if let Some(controller) = self.monitoring_controller.as_ref() {
            let _ = controller.sender().send(LoopMessage::Shutdown);
        }
        info!("[ServiceManager] 监控线程已停止");
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
