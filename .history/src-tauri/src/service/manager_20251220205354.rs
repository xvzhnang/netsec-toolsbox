/// ServiceManager - 统一的服务管理器
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use log::{info, warn, error};

use crate::service::trait_def::{Service, ServiceHandle, HealthStatus};
use crate::service::state::ServiceState;
use crate::service::dto::{ServiceStatusDTO, ServiceStatusListDTO};

/// 服务管理器（统一管理所有服务）
pub struct ServiceManager {
    /// 所有注册的服务
    services: Arc<Mutex<HashMap<String, ServiceHandle>>>,
    /// 监控线程是否运行
    monitoring: Arc<Mutex<bool>>,
}

impl ServiceManager {
    /// 创建新的服务管理器
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
            monitoring: Arc::new(Mutex::new(false)),
        }
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
        services.insert(id, service);
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
            ServiceStatusDTO::from_service(service_guard.as_ref())
        })
    }

    /// 启动服务
    pub fn start_service(&self, id: &str) -> Result<(), String> {
        let services = self.services.lock().unwrap();
        let service = services.get(id)
            .ok_or_else(|| format!("服务 {} 不存在", id))?;
        
        let mut service_guard = service.lock().unwrap();
        info!("[ServiceManager] 启动服务: {}", id);
        
        match service_guard.start() {
            Ok(_) => {
                service_guard.set_state(ServiceState::Idle)
                    .unwrap_or_else(|e| {
                        warn!("[ServiceManager] 设置服务 {} 状态失败: {}", id, e);
                    });
                Ok(())
            }
            Err(e) => {
                error!("[ServiceManager] 启动服务 {} 失败: {}", id, e);
                service_guard.set_state_unchecked(ServiceState::Unhealthy);
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
                let services_guard = services.lock().unwrap();
                for (id, service) in services_guard.iter() {
                    let mut service_guard = service.lock().unwrap();
                    let current_state = service_guard.state();
                    
                    // 跳过停止状态的服务
                    if current_state == ServiceState::Stopped {
                        continue;
                    }

                    // 执行健康检查
                    match service_guard.health_check() {
                        HealthStatus::Healthy => {
                            // 如果当前是降级或不健康，尝试恢复
                            if current_state == ServiceState::Degraded {
                                if let Err(e) = service_guard.set_state(ServiceState::Idle) {
                                    warn!("[ServiceManager] 服务 {} 状态恢复失败: {}", id, e);
                                }
                            }
                        }
                        HealthStatus::Degraded => {
                            if current_state != ServiceState::Degraded {
                                if let Err(e) = service_guard.set_state(ServiceState::Degraded) {
                                    warn!("[ServiceManager] 服务 {} 状态降级失败: {}", id, e);
                                }
                            }
                        }
                        HealthStatus::Unhealthy => {
                            if current_state != ServiceState::Unhealthy 
                                && current_state != ServiceState::Restarting {
                                warn!("[ServiceManager] 服务 {} 健康检查失败，准备重启", id);
                                
                                // 设置状态为重启中
                                service_guard.set_state_unchecked(ServiceState::Restarting);
                                drop(service_guard);
                                
                                // 在另一个作用域中执行重启
                                let service_clone = Arc::clone(service);
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
                        }
                    }
                }
                drop(services_guard);

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

