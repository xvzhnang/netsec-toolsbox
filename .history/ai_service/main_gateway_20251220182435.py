# -*- coding: utf-8 -*-
"""
AI Gateway 主服务
对应 One API 的 main.go
"""
import sys
import io
import os
import traceback
from http.server import HTTPServer
from typing import Optional

# Windows 编码修复
# 确保 stderr 使用 UTF-8 编码，以便 Rust 后端能正确读取
if sys.platform == 'win32':
    try:
        # 重新包装 stderr 以确保使用 UTF-8 编码
        if hasattr(sys.stderr, 'buffer'):
            try:
                # 检查 stderr 是否已经被包装过
                if not isinstance(sys.stderr, io.TextIOWrapper) or sys.stderr.encoding != 'utf-8':
                    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace', line_buffering=True)
            except (ValueError, AttributeError, OSError):
                # 如果流已关闭或无法重新包装，跳过
                pass
    except (AttributeError, OSError):
        # 如果无法访问 buffer 属性，跳过
        pass

# 添加当前目录到 Python 路径
current_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, current_dir)

# 在导入前输出日志
try:
    print(f"[MAIN] 开始导入模块...", file=sys.stderr, flush=True)
    from core.registry import ModelRegistry
    print(f"[MAIN] ModelRegistry 导入成功", file=sys.stderr, flush=True)
    from core.router import Router
    print(f"[MAIN] Router 导入成功", file=sys.stderr, flush=True)
    from api.openai_handler import AIRequestHandler
    print(f"[MAIN] AIRequestHandler 导入成功", file=sys.stderr, flush=True)
    print(f"[MAIN] 所有模块导入完成", file=sys.stderr, flush=True)
except Exception as e:
    print(f"[MAIN] [FATAL] 模块导入失败: {type(e).__name__}: {e}", file=sys.stderr, flush=True)
    import traceback
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)


class GatewayHTTPServer(HTTPServer):
    """自定义 HTTP 服务器，传递 router 到 Handler"""
    
    def __init__(self, server_address, router: Router):
        self.router = router
        super().__init__(server_address, self._make_handler)
    
    def _make_handler(self, *args, **kwargs):
        """创建 Handler 实例，传递 router"""
        return AIRequestHandler(*args, router=self.router, **kwargs)


def safe_print(*args, **kwargs):
    """安全打印函数，在 stdout 不可用时跳过"""
    try:
        if sys.stdout and not sys.stdout.closed:
            print(*args, **kwargs)
    except (ValueError, OSError, AttributeError):
        # stdout 已关闭或不可用，跳过输出
        pass


def run_server(port: int = 8765, config_path: Optional[str] = None):
    """
    启动 AI Gateway 服务
    
    Args:
        port: 服务端口
        config_path: 配置文件路径
    """
    try:
        # 输出到 stderr 以便被 Rust 后端捕获
        print(f"[INIT] 启动 AI Gateway 服务...", file=sys.stderr, flush=True)
        safe_print(f"🚀 启动 AI Gateway 服务...", flush=True)
        
        # 初始化 Registry
        print(f"[INIT] 加载模型配置...", file=sys.stderr, flush=True)
        safe_print(f"📂 加载模型配置...", flush=True)
        
        try:
            registry = ModelRegistry(config_path)
        except Exception as e:
            error_msg = f"加载模型配置失败: {str(e)}"
            print(f"[ERROR] {error_msg}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            sys.exit(1)
        
        if len(registry.adapters) == 0:
            print(f"[WARN] 没有可用的模型，服务将无法处理请求", file=sys.stderr, flush=True)
            safe_print(f"⚠️ 没有可用的模型，服务将无法处理请求", flush=True)
        
        # 初始化 Router
        print(f"[INIT] 初始化路由器...", file=sys.stderr, flush=True)
        try:
            router = Router(registry)
        except Exception as e:
            error_msg = f"初始化路由器失败: {str(e)}"
            print(f"[ERROR] {error_msg}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            sys.exit(1)
        
        # 启动 HTTP 服务器
        print(f"[INIT] 启动 HTTP 服务器，端口: {port}...", file=sys.stderr, flush=True)
        server_address = ('127.0.0.1', port)
        
        try:
            httpd = GatewayHTTPServer(server_address, router)
        except OSError as e:
            if "Address already in use" in str(e) or "address is already in use" in str(e).lower():
                error_msg = f"端口 {port} 已被占用，请检查是否有其他服务正在使用该端口"
            else:
                error_msg = f"启动 HTTP 服务器失败: {str(e)}"
            print(f"[ERROR] {error_msg}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            sys.exit(1)
        except Exception as e:
            error_msg = f"启动 HTTP 服务器失败: {str(e)}"
            print(f"[ERROR] {error_msg}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            sys.exit(1)
        
        # 输出到 stderr 以便被 Rust 后端捕获
        print(f"[READY] AI Gateway 服务已启动", file=sys.stderr, flush=True)
        print(f"[READY] 监听地址: http://127.0.0.1:{port}", file=sys.stderr, flush=True)
        print(f"[READY] 可用模型: {', '.join(registry.adapters.keys()) if registry.adapters else '(无)'}", file=sys.stderr, flush=True)
        print(f"", file=sys.stderr, flush=True)
        safe_print(f"✅ AI Gateway 服务已启动", flush=True)
        safe_print(f"📍 监听地址: http://127.0.0.1:{port}", flush=True)
        safe_print(f"📋 可用模型: {', '.join(registry.adapters.keys()) if registry.adapters else '(无)'}", flush=True)
    
        
        # 注册退出处理
        import atexit
        
        def exit_handler():
            """进程退出时的处理"""
            exc_type, exc_value, exc_traceback = sys.exc_info()
            if exc_type is not None:
                try:
                    print(f"[EXIT] 进程因异常退出: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
                    traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
                except:
                    pass
        
        atexit.register(exit_handler)
        
        # 设置自定义异常处理
        original_excepthook = sys.excepthook
        
        def custom_excepthook(exc_type, exc_value, exc_traceback):
            """捕获未处理的异常"""
            try:
                print(f"[UNHANDLED] 未捕获的异常: {exc_type.__name__}: {exc_value}", file=sys.stderr, flush=True)
                traceback.print_exception(exc_type, exc_value, exc_traceback, file=sys.stderr)
            except:
                pass
            original_excepthook(exc_type, exc_value, exc_traceback)
        
        sys.excepthook = custom_excepthook
        
        # 服务状态标志
        _service_running = True
        
        # 开始服务
        print(f"[SERVICE] 服务启动完成，开始监听请求...", file=sys.stderr, flush=True)
        print(f"[SERVER] 开始监听请求...", file=sys.stderr, flush=True)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            _service_running = False
            print(f"[SERVICE] 服务停止完成 (KeyboardInterrupt)", file=sys.stderr, flush=True)
            print(f"[STOP] 服务已停止 (KeyboardInterrupt)", file=sys.stderr, flush=True)
            safe_print(f"\n🛑 服务已停止", flush=True)
            try:
                httpd.shutdown()
            except:
                pass
        except SystemExit:
            # 重新抛出 SystemExit，让进程正常退出
            _service_running = False
            print(f"[SERVICE] 服务停止完成 (SystemExit)", file=sys.stderr, flush=True)
            print(f"[STOP] 服务已停止 (SystemExit)", file=sys.stderr, flush=True)
            try:
                httpd.shutdown()
            except:
                pass
            raise
        except Exception as e:
            # 捕获所有异常，记录详细信息
            _service_running = False
            error_msg = f"服务异常退出: {type(e).__name__}: {str(e)}"
            print(f"[SERVICE] 服务停止完成 (异常)", file=sys.stderr, flush=True)
            print(f"[ERROR] {error_msg}", file=sys.stderr, flush=True)
            print(f"[ERROR] 异常类型: {type(e).__name__}", file=sys.stderr, flush=True)
            print(f"[ERROR] 异常值: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            safe_print(f"\n❌ {error_msg}", flush=True)
            try:
                httpd.shutdown()
            except:
                pass
            # 只有在严重错误时才退出
            sys.exit(1)
    
    except SystemExit:
        # 重新抛出 SystemExit，让进程正常退出
        raise
    except Exception as e:
        # 捕获所有其他异常
        error_msg = f"服务启动失败: {str(e)}"
        print(f"[FATAL] {error_msg}", file=sys.stderr, flush=True)
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='AI Gateway Service')
    parser.add_argument('--port', type=int, default=8765, help='服务端口 (默认: 8765)')
    parser.add_argument('--config', type=str, default=None, help='配置文件路径')
    args = parser.parse_args()
    
    run_server(port=args.port, config_path=args.config)

