# -*- coding: utf-8 -*-
"""
错误重试机制
对应 One API 中可能的请求重试逻辑
"""
import asyncio
import time
from typing import Callable, Optional, Type, Tuple, Any
from enum import Enum


class RetryableErrorType(Enum):
    """可重试的错误类型"""
    NETWORK_ERROR = "network_error"  # 网络错误（连接失败、超时等）
    RATE_LIMIT = "rate_limit"  # 速率限制
    TEMPORARY_ERROR = "temporary_error"  # 临时服务器错误（5xx）
    TIMEOUT = "timeout"  # 超时错误


class NonRetryableErrorType(Enum):
    """不可重试的错误类型"""
    AUTHENTICATION_ERROR = "authentication_error"  # 认证错误（401, 403）
    INVALID_REQUEST = "invalid_request"  # 请求错误（400, 422）
    NOT_FOUND = "not_found"  # 资源不存在（404）
    MODEL_NOT_FOUND = "model_not_found"  # 模型不存在


def is_retryable_error(error: Exception) -> Tuple[bool, Optional[RetryableErrorType]]:
    """
    判断错误是否可重试
    
    Args:
        error: 异常对象
    
    Returns:
        (是否可重试, 错误类型)
    """
    error_msg = str(error).lower()
    error_type = type(error).__name__
    
    # 网络错误
    if any(keyword in error_msg for keyword in [
        'connection', 'network', 'timeout', 'refused', 'reset',
        'dns', 'unreachable', 'socket'
    ]) or any(err_type in error_type for err_type in [
        'ConnectionError', 'TimeoutError', 'NetworkError', 'ClientError'
    ]):
        return True, RetryableErrorType.NETWORK_ERROR
    
    # 速率限制
    if any(keyword in error_msg for keyword in [
        'rate limit', 'ratelimit', 'too many requests', '429'
    ]):
        return True, RetryableErrorType.RATE_LIMIT
    
    # 临时服务器错误
    if any(keyword in error_msg for keyword in [
        '500', '502', '503', '504', 'internal server error',
        'bad gateway', 'service unavailable', 'gateway timeout'
    ]):
        return True, RetryableErrorType.TEMPORARY_ERROR
    
    # 超时错误
    if 'timeout' in error_msg or 'Timeout' in error_type:
        return True, RetryableErrorType.TIMEOUT
    
    # 认证错误（不可重试）
    if any(keyword in error_msg for keyword in [
        '401', '403', 'unauthorized', 'forbidden', 'authentication',
        'api_key', 'api key', 'invalid key', 'invalid api'
    ]):
        return False, None
    
    # 请求错误（不可重试）
    if any(keyword in error_msg for keyword in [
        '400', '422', 'invalid request', 'bad request', 'validation'
    ]):
        return False, None
    
    # 资源不存在（不可重试）
    if any(keyword in error_msg for keyword in [
        '404', 'not found', 'model not found'
    ]):
        return False, None
    
    # 默认：网络相关错误可重试，其他不可重试
    if any(err_type in error_type for err_type in [
        'ConnectionError', 'TimeoutError', 'NetworkError'
    ]):
        return True, RetryableErrorType.NETWORK_ERROR
    
    return False, None


class RetryConfig:
    """重试配置"""
    
    def __init__(
        self,
        max_retries: int = 3,
        initial_delay: float = 1.0,
        max_delay: float = 60.0,
        exponential_base: float = 2.0,
        jitter: bool = True
    ):
        """
        初始化重试配置
        
        Args:
            max_retries: 最大重试次数（不包括首次请求）
            initial_delay: 初始延迟（秒）
            max_delay: 最大延迟（秒）
            exponential_base: 指数退避基数
            jitter: 是否添加随机抖动
        """
        self.max_retries = max_retries
        self.initial_delay = initial_delay
        self.max_delay = max_delay
        self.exponential_base = exponential_base
        self.jitter = jitter
    
    def get_delay(self, attempt: int) -> float:
        """
        计算第 attempt 次重试的延迟时间
        
        Args:
            attempt: 重试次数（0 为首次重试）
        
        Returns:
            延迟时间（秒）
        """
        # 指数退避：delay = initial_delay * (base ^ attempt)
        delay = self.initial_delay * (self.exponential_base ** attempt)
        
        # 限制最大延迟
        delay = min(delay, self.max_delay)
        
        # 添加随机抖动（±25%）
        if self.jitter:
            import random
            jitter_amount = delay * 0.25 * (random.random() * 2 - 1)  # -25% 到 +25%
            delay = max(0.1, delay + jitter_amount)  # 确保延迟 >= 0.1 秒
        
        return delay


async def retry_with_backoff(
    func: Callable,
    config: Optional[RetryConfig] = None,
    on_retry: Optional[Callable[[Exception, int, float], None]] = None,
    **kwargs
) -> Any:
    """
    带指数退避的重试装饰器
    
    Args:
        func: 要执行的异步函数
        config: 重试配置（默认使用 RetryConfig()）
        on_retry: 重试回调函数 (error, attempt, delay) -> None
        **kwargs: 传递给 func 的参数
    
    Returns:
        函数执行结果
    
    Raises:
        最后一次尝试的异常
    """
    if config is None:
        config = RetryConfig()
    
    last_error = None
    
    # 首次尝试
    try:
        return await func(**kwargs)
    except Exception as e:
        last_error = e
        is_retryable, error_type = is_retryable_error(e)
        
        if not is_retryable:
            # 不可重试，直接抛出
            raise e
    
    # 重试
    for attempt in range(config.max_retries):
        delay = config.get_delay(attempt)
        
        # 调用重试回调
        if on_retry:
            try:
                on_retry(last_error, attempt + 1, delay)
            except Exception:
                pass  # 忽略回调错误
        
        # 等待
        await asyncio.sleep(delay)
        
        # 重试
        try:
            return await func(**kwargs)
        except Exception as e:
            last_error = e
            is_retryable, error_type = is_retryable_error(e)
            
            if not is_retryable:
                # 不可重试，直接抛出
                raise e
            
            # 最后一次重试失败，抛出异常
            if attempt == config.max_retries - 1:
                raise e
    
    # 所有重试都失败
    raise last_error


def create_retry_config_from_dict(config_dict: dict) -> RetryConfig:
    """
    从配置字典创建 RetryConfig
    
    Args:
        config_dict: 配置字典
    
    Returns:
        RetryConfig 实例
    """
    return RetryConfig(
        max_retries=config_dict.get('max_retries', 3),
        initial_delay=config_dict.get('initial_delay', 1.0),
        max_delay=config_dict.get('max_delay', 60.0),
        exponential_base=config_dict.get('exponential_base', 2.0),
        jitter=config_dict.get('jitter', True)
    )

