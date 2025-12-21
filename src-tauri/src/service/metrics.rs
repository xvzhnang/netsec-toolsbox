/// 监控与指标系统
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 服务指标
#[derive(Debug, Clone)]
pub struct ServiceMetrics {
    /// 服务 ID
    pub service_id: String,
    /// 总请求数
    pub total_requests: u64,
    /// 总成功数
    pub total_successes: u64,
    /// 总失败数
    pub total_failures: u64,
    /// 平均响应时间（毫秒）
    pub avg_response_time_ms: f64,
    /// 最后响应时间
    #[allow(dead_code)]
    pub last_response_time: Option<Instant>,
    /// 状态变化次数
    pub state_changes: u64,
    /// 最后状态变化时间
    pub last_state_change: Option<Instant>,
    /// 健康检查次数
    pub health_check_count: u64,
    /// 健康检查失败次数
    pub health_check_failures: u64,
    /// 启动次数
    pub start_count: u64,
    /// 重启次数
    pub restart_count: u64,
    /// 错误历史（最近 N 条）
    pub recent_errors: Vec<(Instant, String)>,
}

impl ServiceMetrics {
    pub fn new(service_id: String) -> Self {
        Self {
            service_id,
            total_requests: 0,
            total_successes: 0,
            total_failures: 0,
            avg_response_time_ms: 0.0,
            last_response_time: None,
            state_changes: 0,
            last_state_change: None,
            health_check_count: 0,
            health_check_failures: 0,
            start_count: 0,
            restart_count: 0,
            recent_errors: Vec::new(),
        }
    }

    /// 记录请求
    pub fn record_request(&mut self, success: bool, response_time_ms: u64) {
        self.total_requests += 1;
        if success {
            self.total_successes += 1;
        } else {
            self.total_failures += 1;
        }

        // 更新平均响应时间（滑动平均）
        if self.total_requests == 1 {
            self.avg_response_time_ms = response_time_ms as f64;
        } else {
            self.avg_response_time_ms =
                (self.avg_response_time_ms * 0.9) + (response_time_ms as f64 * 0.1);
        }

        self.last_response_time = Some(Instant::now());
    }

    /// 记录状态变化
    pub fn record_state_change(&mut self) {
        self.state_changes += 1;
        self.last_state_change = Some(Instant::now());
    }

    /// 记录健康检查
    pub fn record_health_check(&mut self, healthy: bool) {
        self.health_check_count += 1;
        if !healthy {
            self.health_check_failures += 1;
        }
    }

    /// 记录启动
    pub fn record_start(&mut self) {
        self.start_count += 1;
    }

    /// 记录重启
    pub fn record_restart(&mut self) {
        self.restart_count += 1;
    }

    /// 记录错误
    pub fn record_error(&mut self, error: String) {
        self.recent_errors.push((Instant::now(), error));
        // 只保留最近 100 条错误
        if self.recent_errors.len() > 100 {
            self.recent_errors.remove(0);
        }
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.total_successes as f64 / self.total_requests as f64
    }

    /// 获取失败率
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.total_failures as f64 / self.total_requests as f64
    }

    /// 获取健康检查成功率
    pub fn health_check_success_rate(&self) -> f64 {
        if self.health_check_count == 0 {
            return 1.0;
        }
        (self.health_check_count - self.health_check_failures) as f64
            / self.health_check_count as f64
    }
}

/// 指标收集器
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// 服务指标（按服务 ID）
    metrics: Arc<Mutex<HashMap<String, ServiceMetrics>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取或创建服务指标
    fn get_or_create_metrics(&self, service_id: &str) -> Arc<Mutex<ServiceMetrics>> {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let entry = metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceMetrics::new(service_id.to_string()));
        Arc::new(Mutex::new(entry.clone()))
    }

    /// 记录请求
    pub fn record_request(&self, service_id: &str, success: bool, response_time_ms: u64) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let metric = metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceMetrics::new(service_id.to_string()));
        metric.record_request(success, response_time_ms);
    }

    /// 记录状态变化
    pub fn record_state_change(&self, service_id: &str) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let metric = metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceMetrics::new(service_id.to_string()));
        metric.record_state_change();
    }

    /// 记录健康检查
    pub fn record_health_check(&self, service_id: &str, healthy: bool) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let metric = metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceMetrics::new(service_id.to_string()));
        metric.record_health_check(healthy);
    }

    /// 记录启动
    pub fn record_start(&self, service_id: &str) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let metric = metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceMetrics::new(service_id.to_string()));
        metric.record_start();
    }

    /// 记录重启
    pub fn record_restart(&self, service_id: &str) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let metric = metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceMetrics::new(service_id.to_string()));
        metric.record_restart();
    }

    /// 记录错误
    pub fn record_error(&self, service_id: &str, error: String) {
        let mut metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let metric = metrics
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceMetrics::new(service_id.to_string()));
        metric.record_error(error);
    }

    /// 获取服务指标
    pub fn get_metrics(&self, service_id: &str) -> Option<ServiceMetrics> {
        let metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        metrics.get(service_id).cloned()
    }

    /// 获取所有指标
    pub fn get_all_metrics(&self) -> HashMap<String, ServiceMetrics> {
        let metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        metrics.clone()
    }

    /// 获取 Prometheus 格式的指标（简化版）
    pub fn to_prometheus_format(&self) -> String {
        let metrics =
            crate::utils::lock_or_recover(self.metrics.as_ref(), "MetricsCollector.metrics");
        let mut output = String::new();

        for (service_id, metric) in metrics.iter() {
            output.push_str(&format!(
                "# HELP service_requests_total Total number of requests\n\
                 # TYPE service_requests_total counter\n\
                 service_requests_total{{service=\"{}\"}} {}\n",
                service_id, metric.total_requests
            ));

            output.push_str(&format!(
                "# HELP service_successes_total Total number of successful requests\n\
                 # TYPE service_successes_total counter\n\
                 service_successes_total{{service=\"{}\"}} {}\n",
                service_id, metric.total_successes
            ));

            output.push_str(&format!(
                "# HELP service_failures_total Total number of failed requests\n\
                 # TYPE service_failures_total counter\n\
                 service_failures_total{{service=\"{}\"}} {}\n",
                service_id, metric.total_failures
            ));

            output.push_str(&format!(
                "# HELP service_avg_response_time_ms Average response time in milliseconds\n\
                 # TYPE service_avg_response_time_ms gauge\n\
                 service_avg_response_time_ms{{service=\"{}\"}} {}\n",
                service_id, metric.avg_response_time_ms
            ));

            output.push_str(&format!(
                "# HELP service_success_rate Success rate (0-1)\n\
                 # TYPE service_success_rate gauge\n\
                 service_success_rate{{service=\"{}\"}} {}\n",
                service_id,
                metric.success_rate()
            ));
        }

        output
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
