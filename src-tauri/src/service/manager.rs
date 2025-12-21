use log::{error, info, warn};
/// ServiceManager - 统一的服务管理器
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::service::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::service::dto::{ServiceStatusDTO, ServiceStatusListDTO};
use crate::service::events::{current_timestamp, EventBus, ServiceEvent};
use crate::service::metrics::MetricsCollector;
use crate::service::state::ServiceState;
use crate::service::trait_def::{HealthStatus, ServiceHandle};

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
pub struct ServiceManager {
    /// 所有注册的服务
    services: Arc<Mutex<HashMap<String, ServiceHandle>>>,
    /// 监控线程是否运行
    monitoring: Arc<Mutex<bool>>,
    /// 事件总线
    event_bus: Arc<Mutex<EventBus>>,
    /// 服务熔断器（按服务 ID）
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    /// 指标收集器
    metrics: Arc<Mutex<MetricsCollector>>,
    restart_policy: RestartPolicy,
    recovery: Arc<Mutex<HashMap<String, RecoveryState>>>,
}

impl ServiceManager {
    /// 创建新的服务管理器
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
            monitoring: Arc::new(Mutex::new(false)),
            event_bus: Arc::new(Mutex::new(EventBus::new())),
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(MetricsCollector::new())),
            restart_policy: RestartPolicy::default(),
            recovery: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取指标收集器
    pub fn metrics(&self) -> Arc<Mutex<MetricsCollector>> {
        Arc::clone(&self.metrics)
    }

    /// 获取 Prometheus 格式的指标
    pub fn get_prometheus_metrics(&self) -> String {
        let metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "ServiceManager.metrics");
        metrics.to_prometheus_format()
    }

    /// 获取事件总线（用于订阅事件）
    #[allow(dead_code)]
    pub fn event_bus(&self) -> Arc<Mutex<EventBus>> {
        Arc::clone(&self.event_bus)
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
            crate::utils::lock_or_recover(self.services.as_ref(), "ServiceManager.services");
        let id = {
            let s = crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
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
            crate::utils::lock_or_recover(self.services.as_ref(), "ServiceManager.services");
        if let Some(service) = services.remove(id) {
            info!("[ServiceManager] 注销服务: {}", id);
            // 尝试停止服务
            let mut service_guard =
                crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
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
            crate::utils::lock_or_recover(self.services.as_ref(), "ServiceManager.services");
        services.get(id).map(|s| Arc::clone(s))
    }

    /// 获取所有服务状态
    pub fn get_all_status(&self) -> ServiceStatusListDTO {
        let services =
            crate::utils::lock_or_recover(self.services.as_ref(), "ServiceManager.services");
        let mut status_list = Vec::new();

        for (_, service) in services.iter() {
            let service_guard = crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
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
                ServiceState::Stopped => 4,
            };
            let b_priority = match b.state {
                ServiceState::Idle | ServiceState::Busy => 0,
                ServiceState::Degraded => 1,
                ServiceState::Starting | ServiceState::Warmup | ServiceState::Stopping => 2,
                ServiceState::Unhealthy | ServiceState::Restarting => 3,
                ServiceState::Stopped => 4,
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
            crate::utils::lock_or_recover(self.services.as_ref(), "ServiceManager.services");
        services.get(id).map(|service| {
            let service_guard = crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
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

        let services =
            crate::utils::lock_or_recover(self.services.as_ref(), "ServiceManager.services");
        let service = services
            .get(id)
            .ok_or_else(|| format!("服务 {} 不存在", id))?;

        let from_state = {
            let s = crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
            s.state()
        };

        if matches!(
            from_state,
            ServiceState::Idle
                | ServiceState::Busy
                | ServiceState::Degraded
                | ServiceState::Starting
                | ServiceState::Warmup
        ) {
            info!(
                "[ServiceManager] 服务 {} 当前状态为 {}，无需重复启动",
                id, from_state
            );
            return Ok(());
        }

        let mut service_guard = crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
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
                    let metrics = crate::utils::lock_or_recover(
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
                let to_state = ServiceState::Unhealthy;
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
                    let metrics = crate::utils::lock_or_recover(
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
        let services =
            crate::utils::lock_or_recover(self.services.as_ref(), "ServiceManager.services");
        let service = services
            .get(id)
            .ok_or_else(|| format!("服务 {} 不存在", id))?;

        let mut service_guard = crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
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
        thread::sleep(Duration::from_millis(500));
        self.start_service(id)
    }

    /// 启动监控循环（后台线程）
    pub fn start_monitoring(&self) {
        let mut monitoring =
            crate::utils::lock_or_recover(self.monitoring.as_ref(), "ServiceManager.monitoring");
        if *monitoring {
            warn!("[ServiceManager] 监控线程已在运行");
            return;
        }
        *monitoring = true;
        drop(monitoring);

        let services = Arc::clone(&self.services);
        let monitoring_flag = Arc::clone(&self.monitoring);
        let metrics = Arc::clone(&self.metrics);
        let event_bus = Arc::clone(&self.event_bus);
        let recovery = Arc::clone(&self.recovery);
        let restart_policy = self.restart_policy.clone();

        thread::spawn(move || {
            info!("[ServiceManager] 监控线程已启动");

            loop {
                // 检查是否应该停止监控
                {
                    let flag = crate::utils::lock_or_recover(
                        monitoring_flag.as_ref(),
                        "ServiceManager.monitoring",
                    );
                    if !*flag {
                        info!("[ServiceManager] 监控线程已停止");
                        break;
                    }
                }

                let now = Instant::now();
                let mut restarts_due: Vec<(String, ServiceHandle)> = Vec::new();
                let mut stops_due: Vec<(String, ServiceHandle)> = Vec::new();
                {
                    let services_guard =
                        crate::utils::lock_or_recover(services.as_ref(), "ServiceManager.services");
                    for (id, service) in services_guard.iter() {
                        let mut service_guard =
                            crate::utils::lock_or_recover(service.as_ref(), "ServiceHandle");
                        let current_state = service_guard.state();

                        // 跳过停止/停止中状态的服务
                        if current_state == ServiceState::Stopped
                            || current_state == ServiceState::Stopping
                        {
                            continue;
                        }

                        {
                            let mut recovery_guard = crate::utils::lock_or_recover(
                                recovery.as_ref(),
                                "ServiceManager.recovery",
                            );
                            let entry = recovery_guard.entry(id.clone()).or_default();
                            if entry.paused {
                                continue;
                            }
                            if entry.restart_in_progress {
                                continue;
                            }
                            if current_state == ServiceState::Starting
                                && entry.starting_since.is_none()
                            {
                                entry.starting_since = Some(now);
                            }
                            if let Some(until) = entry.backoff_until {
                                if now >= until && entry.dead_since.is_some() {
                                    entry.restart_in_progress = true;
                                    entry.backoff_until = None;
                                    restarts_due.push((id.clone(), Arc::clone(service)));
                                    continue;
                                }
                                if now < until {
                                    continue;
                                }
                            }
                        }

                        // 执行健康检查
                        let health_result = service_guard.health_check();
                        let current_state = service_guard.state();

                        // 记录指标
                        {
                            let metrics_guard = crate::utils::lock_or_recover(
                                metrics.as_ref(),
                                "ServiceManager.metrics",
                            );
                            let is_healthy = matches!(health_result, HealthStatus::Healthy);
                            metrics_guard.record_health_check(id, is_healthy);
                        }

                        // 发送健康检查事件
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

                        {
                            let mut recovery_guard = crate::utils::lock_or_recover(
                                recovery.as_ref(),
                                "ServiceManager.recovery",
                            );
                            let entry = recovery_guard.entry(id.clone()).or_default();
                            if entry.paused {
                                continue;
                            }

                            let in_grace = entry
                                .starting_since
                                .map(|since| {
                                    now.duration_since(since) < restart_policy.grace_period
                                })
                                .unwrap_or(false);

                            match health_result {
                                HealthStatus::Healthy => {
                                    entry.degraded_since = None;
                                    entry.dead_since = None;
                                    entry.starting_since = None;
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
                                }
                            }

                            if entry.dead_since.is_some() {
                                match restart_policy.can_restart(&mut entry.restart_history, now) {
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

                        if let Some((from, to)) = state_change {
                            if let Err(e) = service_guard.set_state(to) {
                                warn!("[ServiceManager] 服务 {} 状态切换失败: {}", id, e);
                            } else {
                                {
                                    let metrics_guard = crate::utils::lock_or_recover(
                                        metrics.as_ref(),
                                        "ServiceManager.metrics",
                                    );
                                    metrics_guard.record_state_change(id);
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
                            stops_due.push((id.clone(), Arc::clone(service)));
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
                }

                for (id, service) in stops_due {
                    let service_clone = Arc::clone(&service);
                    let recovery_clone = Arc::clone(&recovery);
                    let event_bus_clone = Arc::clone(&event_bus);
                    thread::spawn(move || {
                        let mut s =
                            crate::utils::lock_or_recover(service_clone.as_ref(), "ServiceHandle");
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
                    let service_clone = Arc::clone(&service);
                    let recovery_clone = Arc::clone(&recovery);
                    let event_bus_clone = Arc::clone(&event_bus);
                    let metrics_clone = Arc::clone(&metrics);
                    thread::spawn(move || {
                        let mut s =
                            crate::utils::lock_or_recover(service_clone.as_ref(), "ServiceHandle");
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
                                    let metrics_guard = crate::utils::lock_or_recover(
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
                                s.set_state_unchecked(ServiceState::Unhealthy);
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

                // 每 10 秒检查一次（优化：减少检查频率，降低资源占用）
                thread::sleep(Duration::from_secs(10));
            }
        });
    }

    /// 停止监控
    pub fn stop_monitoring(&self) {
        let mut monitoring =
            crate::utils::lock_or_recover(self.monitoring.as_ref(), "ServiceManager.monitoring");
        *monitoring = false;
        info!("[ServiceManager] 监控线程已停止");
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
