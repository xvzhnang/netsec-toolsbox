/// ServiceManager - 统一的服务管理器
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use log::{info, warn, error};

use crate::service::trait_def::{Service, ServiceHandle, HealthStatus};
use crate::service::state::ServiceState;
use crate::service::dto::{ServiceStatusDTO, ServiceStatusListDTO};
use crate::service::events::{EventBus, ServiceEvent, current_timestamp};
use crate::service::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

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
        }
    }
    
    /// 获取指标收集器
    pub fn metrics(&self) -> Arc<Mutex<MetricsCollector>> {
        Arc::clone(&self.metrics)
    }
    
    /// 获取 Prometheus 格式的指标
    pub fn get_prometheus_metrics(&self) -> String {
        let metrics = self.metrics.lock().unwrap();
        metrics.to_prometheus_format()
    }
    
    /// 获取事件总线（用于订阅事件）
    pub fn event_bus(&self) -> Arc<Mutex<EventBus>> {
        Arc::clone(&self.event_bus)
    }
    
    /// 发送事件
    fn emit_event(&self, event: ServiceEvent) {
        let bus = self.event_bus.lock().unwrap();
        bus.emit(&event);
    }

    /// 注册服务
    pub fn register(&self, service: ServiceHandle) -> Result<(), String> {
        let mut services = self.services.lock().unwrap();
        let id = {
            let s = service.lock().unwrap();
            s.id().to_string()
        };
        
        if services.contains_key(&id) {
            return Err(format!("服务 {} 已存在", id));
        }
        
        info!("[ServiceManager] 注册服务: {}", id);
        
        // 为服务创建熔断器
        let mut breakers = self.circuit_breakers.lock().unwrap();
        breakers.insert(id.clone(), CircuitBreaker::new(CircuitBreakerConfig::default()));
        drop(breakers);
        
        services.insert(id.clone(), service);
        
        // 发送注册事件
        self.emit_event(ServiceEvent::Started {
            service_id: id,
            timestamp: current_timestamp(),
        });
        
        Ok(())
    }

    /// 注销服务
    pub fn unregister(&self, id: &str) -> Result<(), String> {
        let mut services = self.services.lock().unwrap();
        if let Some(mut service) = services.remove(id) {
            info!("[ServiceManager] 注销服务: {}", id);
            // 尝试停止服务
            if let Err(e) = service.lock().unwrap().stop() {
                warn!("[ServiceManager] 停止服务 {} 失败: {}", id, e);
            }
            Ok(())
        } else {
            Err(format!("服务 {} 不存在", id))
        }
    }

    /// 获取服务
    pub fn get_service(&self, id: &str) -> Option<ServiceHandle> {
        let services = self.services.lock().unwrap();
        services.get(id).map(|s| Arc::clone(s))
    }

    /// 获取所有服务状态
    pub fn get_all_status(&self) -> ServiceStatusListDTO {
        let services = self.services.lock().unwrap();
        let mut status_list = Vec::new();
        
        for (_, service) in services.iter() {
            let service_guard = service.lock().unwrap();
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
        let services = self.services.lock().unwrap();
        services.get(id).map(|service| {
            let service_guard = service.lock().unwrap();
            ServiceStatusDTO::from_service(&*service_guard)
        })
    }

    /// 启动服务
    pub fn start_service(&self, id: &str) -> Result<(), String> {
        // 检查熔断器
        let breakers = self.circuit_breakers.lock().unwrap();
        if let Some(breaker) = breakers.get(id) {
            if !breaker.can_execute() {
                return Err(format!("服务 {} 处于熔断状态，无法启动", id));
            }
        }
        drop(breakers);
        
        let services = self.services.lock().unwrap();
        let service = services.get(id)
            .ok_or_else(|| format!("服务 {} 不存在", id))?;
        
        let from_state = {
            let s = service.lock().unwrap();
            s.state()
        };
        
        let mut service_guard = service.lock().unwrap();
        info!("[ServiceManager] 启动服务: {}", id);
        
        service_guard.set_state(ServiceState::Starting)
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
                service_guard.set_state(to_state)
                    .unwrap_or_else(|e| {
                        warn!("[ServiceManager] 设置服务 {} 状态失败: {}", id, e);
                    });
                
                // 记录成功，重置熔断器
                let mut breakers = self.circuit_breakers.lock().unwrap();
                if let Some(breaker) = breakers.get_mut(id) {
                    breaker.record_success();
                }
                drop(breakers);
                
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
                let mut breakers = self.circuit_breakers.lock().unwrap();
                if let Some(breaker) = breakers.get_mut(id) {
                    breaker.record_failure();
                }
                drop(breakers);
                
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
        let services = self.services.lock().unwrap();
        let service = services.get(id)
            .ok_or_else(|| format!("服务 {} 不存在", id))?;
        
        let mut service_guard = service.lock().unwrap();
        info!("[ServiceManager] 停止服务: {}", id);
        
        service_guard.set_state(ServiceState::Stopping)
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
        let mut monitoring = self.monitoring.lock().unwrap();
        if *monitoring {
            warn!("[ServiceManager] 监控线程已在运行");
            return;
        }
        *monitoring = true;
        drop(monitoring);

        let services = Arc::clone(&self.services);
        let monitoring_flag = Arc::clone(&self.monitoring);

        thread::spawn(move || {
            info!("[ServiceManager] 监控线程已启动");
            
            loop {
                // 检查是否应该停止监控
                {
                    let flag = monitoring_flag.lock().unwrap();
                    if !*flag {
                        info!("[ServiceManager] 监控线程已停止");
                        break;
                    }
                }

                // 执行健康检查
                let mut services_to_restart = Vec::new();
                {
                    let services_guard = services.lock().unwrap();
                    for (id, service) in services_guard.iter() {
                        let mut service_guard = service.lock().unwrap();
                        let current_state = service_guard.state();
                        
                        // 跳过停止状态的服务
                        if current_state == ServiceState::Stopped {
                            continue;
                        }

                    // 执行健康检查
                    let health_result = service_guard.health_check();
                    
                    // 发送健康检查事件
                    let health_event_status = match health_result {
                        HealthStatus::Healthy => crate::service::events::HealthCheckResult::Healthy,
                        HealthStatus::Degraded => crate::service::events::HealthCheckResult::Degraded,
                        HealthStatus::Unhealthy => crate::service::events::HealthCheckResult::Unhealthy,
                    };
                    
                    {
                        let manager = self.clone();
                        let id_clone = id.clone();
                        thread::spawn(move || {
                            manager.emit_event(&crate::service::events::ServiceEvent::HealthCheck {
                                service_id: id_clone,
                                status: health_event_status,
                                timestamp: crate::service::events::current_timestamp(),
                            });
                        });
                    }
                    
                    match health_result {
                        HealthStatus::Healthy => {
                            // 如果当前是降级或不健康，尝试恢复
                            if current_state == ServiceState::Degraded {
                                if let Err(e) = service_guard.set_state(ServiceState::Idle) {
                                    warn!("[ServiceManager] 服务 {} 状态恢复失败: {}", id, e);
                                } else {
                                    // 发送状态变化事件
                                    let manager = self.clone();
                                    let id_clone = id.clone();
                                    thread::spawn(move || {
                                        manager.emit_event(&crate::service::events::ServiceEvent::StateChanged {
                                            service_id: id_clone,
                                            from: ServiceState::Degraded,
                                            to: ServiceState::Idle,
                                            timestamp: crate::service::events::current_timestamp(),
                                        });
                                    });
                                }
                            }
                        }
                        HealthStatus::Degraded => {
                            if current_state != ServiceState::Degraded {
                                if let Err(e) = service_guard.set_state(ServiceState::Degraded) {
                                    warn!("[ServiceManager] 服务 {} 状态降级失败: {}", id, e);
                                } else {
                                    // 发送状态变化事件
                                    let manager = self.clone();
                                    let id_clone = id.clone();
                                    thread::spawn(move || {
                                        manager.emit_event(&crate::service::events::ServiceEvent::StateChanged {
                                            service_id: id_clone,
                                            from: current_state,
                                            to: ServiceState::Degraded,
                                            timestamp: crate::service::events::current_timestamp(),
                                        });
                                    });
                                }
                            }
                        }
                        HealthStatus::Unhealthy => {
                            if current_state != ServiceState::Unhealthy 
                                && current_state != ServiceState::Restarting {
                                warn!("[ServiceManager] 服务 {} 健康检查失败，准备重启", id);
                                
                                // 设置状态为重启中
                                service_guard.set_state_unchecked(ServiceState::Restarting);
                                
                                // 发送状态变化事件
                                let manager = self.clone();
                                let id_clone = id.clone();
                                let old_state = current_state;
                                thread::spawn(move || {
                                    manager.emit_event(&crate::service::events::ServiceEvent::StateChanged {
                                        service_id: id_clone.clone(),
                                        from: old_state,
                                        to: ServiceState::Restarting,
                                        timestamp: crate::service::events::current_timestamp(),
                                    });
                                });
                                
                                // 记录需要重启的服务
                                services_to_restart.push((id.clone(), Arc::clone(service)));
                            }
                        }
                    }
                    }
                }
                
                // 在锁外执行重启（避免死锁）
                for (id, service) in services_to_restart {
                    let service_clone = Arc::clone(&service);
                    thread::spawn(move || {
                        let mut s = service_clone.lock().unwrap();
                        if let Err(e) = s.stop() {
                            error!("[ServiceManager] 停止服务 {} 失败: {}", id, e);
                        }
                        thread::sleep(Duration::from_millis(1000));
                        if let Err(e) = s.start() {
                            error!("[ServiceManager] 重启服务 {} 失败: {}", id, e);
                        } else {
                            s.set_state_unchecked(ServiceState::Idle);
                        }
                    });
                }

                // 每 5 秒检查一次
                thread::sleep(Duration::from_secs(5));
            }
        });
    }

    /// 停止监控
    pub fn stop_monitoring(&self) {
        let mut monitoring = self.monitoring.lock().unwrap();
        *monitoring = false;
        info!("[ServiceManager] 监控线程已停止");
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

