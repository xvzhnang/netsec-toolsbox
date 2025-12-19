# -*- coding: utf-8 -*-
"""
AI Gateway 主服务
对应 One API 的 main.go
"""
import sys
import io
import os
from http.server import HTTPServer
from typing import Optional

# Windows 编码修复
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')

import sys
import os

# 添加当前目录到 Python 路径
current_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, current_dir)

from core.registry import ModelRegistry
from core.router import Router
from api.openai_handler import AIRequestHandler


class GatewayHTTPServer(HTTPServer):
    """自定义 HTTP 服务器，传递 router 到 Handler"""
    
    def __init__(self, server_address, router: Router):
        self.router = router
        super().__init__(server_address, self._make_handler)
    
    def _make_handler(self, *args, **kwargs):
        """创建 Handler 实例，传递 router"""
        return AIRequestHandler(*args, router=self.router, **kwargs)


def run_server(port: int = 8765, config_path: Optional[str] = None):
    """
    启动 AI Gateway 服务
    
    Args:
        port: 服务端口
        config_path: 配置文件路径
    """
    print(f"🚀 启动 AI Gateway 服务...", flush=True)
    
    # 初始化 Registry
    print(f"📂 加载模型配置...", flush=True)
    registry = ModelRegistry(config_path)
    
    if len(registry.adapters) == 0:
        print(f"⚠️ 没有可用的模型，服务将无法处理请求", flush=True)
    
    # 初始化 Router
    router = Router(registry)
    
    # 启动 HTTP 服务器
    server_address = ('127.0.0.1', port)
    httpd = GatewayHTTPServer(server_address, router)
    
    print(f"✅ AI Gateway 服务已启动", flush=True)
    print(f"📍 监听地址: http://127.0.0.1:{port}", flush=True)
    print(f"📋 可用模型: {', '.join(registry.adapters.keys())}", flush=True)
    print(f"", flush=True)
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print(f"\n🛑 服务已停止", flush=True)
        httpd.shutdown()


if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='AI Gateway Service')
    parser.add_argument('--port', type=int, default=8765, help='服务端口 (默认: 8765)')
    parser.add_argument('--config', type=str, default=None, help='配置文件路径')
    args = parser.parse_args()
    
    run_server(port=args.port, config_path=args.config)

