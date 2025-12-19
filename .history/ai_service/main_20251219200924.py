#!/usr/bin/env python3
"""
AI 安全助手服务
提供 HTTP API 接口，支持多个 AI 模型提供商
"""

import json
import os
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
import threading
from typing import Optional, Dict, Any
import traceback

# 导入模型提供商
try:
    from providers.openai_provider import OpenAIProvider
    HAS_OPENAI = True
except ImportError:
    HAS_OPENAI = False

try:
    from providers.deepseek_provider import DeepSeekProvider
    HAS_DEEPSEEK = True
except ImportError:
    HAS_DEEPSEEK = False

try:
    from providers.ollama_provider import OllamaProvider
    HAS_OLLAMA = True
except ImportError:
    HAS_OLLAMA = False

try:
    from providers.lmstudio_provider import LMStudioProvider
    HAS_LMSTUDIO = True
except ImportError:
    HAS_LMSTUDIO = False

try:
    from providers.llamacpp_provider import LlamaCppProvider
    HAS_LLAMACPP = True
except ImportError:
    HAS_LLAMACPP = False


class AIRequestHandler(BaseHTTPRequestHandler):
    """处理 AI API 请求"""
    
    def __init__(self, *args, **kwargs):
        try:
            self.providers = {}
            self._init_providers()
            super().__init__(*args, **kwargs)
        except Exception as e:
            print(f"❌ AIRequestHandler 初始化失败: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            raise
    
    def _init_providers(self):
        """初始化模型提供商"""
        # 从配置文件加载
        config_path = self._get_config_path()
        config = self._load_config(config_path)
        
        if HAS_OPENAI and config.get('openai', {}).get('api_key'):
            self.providers['openai'] = OpenAIProvider(config.get('openai', {}))
        
        if HAS_DEEPSEEK and config.get('deepseek', {}).get('api_key'):
            self.providers['deepseek'] = DeepSeekProvider(config.get('deepseek', {}))
        
        if HAS_LOCAL:
            self.providers['local'] = LocalProvider(config.get('local', {}))
    
    def _get_config_path(self) -> str:
        """获取配置文件路径"""
        # 优先使用环境变量
        config_dir = os.environ.get('NETSEC_TOOLBOX_CONFIG_DIR')
        if config_dir:
            return os.path.join(config_dir, 'ai.json')
        
        # 默认路径：用户配置目录下的 .config/ai.json
        if sys.platform == 'win32':
            appdata = os.environ.get('APPDATA', '')
            if appdata:
                return os.path.join(appdata, 'netsec-toolbox', '.config', 'ai.json')
        
        # 回退到当前目录
        return os.path.join(os.path.dirname(__file__), 'ai_config.json')
    
    def _load_config(self, config_path: str) -> Dict[str, Any]:
        """加载配置文件"""
        if os.path.exists(config_path):
            try:
                with open(config_path, 'r', encoding='utf-8') as f:
                    return json.load(f)
            except Exception as e:
                print(f"⚠️ 加载配置文件失败: {e}", file=sys.stderr)
        return {}
    
    def _send_json_response(self, data: Dict[str, Any], status_code: int = 200):
        """发送 JSON 响应"""
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()
        
        response = json.dumps(data, ensure_ascii=False).encode('utf-8')
        self.wfile.write(response)
    
    def _send_error(self, message: str, status_code: int = 400):
        """发送错误响应"""
        self._send_json_response({
            'success': False,
            'error': message
        }, status_code)
    
    def do_OPTIONS(self):
        """处理 CORS 预检请求"""
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()
    
    def do_GET(self):
        """处理 GET 请求"""
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/health':
            self._send_json_response({
                'success': True,
                'status': 'healthy',
                'providers': list(self.providers.keys())
            })
        elif parsed_path.path == '/providers':
            self._send_json_response({
                'success': True,
                'providers': list(self.providers.keys()),
                'available': {
                    'openai': HAS_OPENAI,
                    'deepseek': HAS_DEEPSEEK,
                    'local': HAS_LOCAL
                }
            })
        else:
            self._send_error('Not found', 404)
    
    def do_POST(self):
        """处理 POST 请求"""
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/chat':
            try:
                content_length = int(self.headers.get('Content-Length', 0))
                body = self.rfile.read(content_length)
                data = json.loads(body.decode('utf-8'))
                
                provider_name = data.get('provider', 'openai')
                messages = data.get('messages', [])
                
                if provider_name not in self.providers:
                    self._send_error(f'Provider "{provider_name}" not available', 400)
                    return
                
                provider = self.providers[provider_name]
                
                # 调用模型生成回复
                response_text = provider.chat(messages)
                
                self._send_json_response({
                    'success': True,
                    'response': response_text
                })
                
            except json.JSONDecodeError:
                self._send_error('Invalid JSON', 400)
            except Exception as e:
                error_msg = str(e)
                traceback.print_exc()
                self._send_error(f'Internal error: {error_msg}', 500)
        else:
            self._send_error('Not found', 404)
    
    def log_message(self, format, *args):
        """自定义日志格式"""
        print(f"[AI Service] {args[0]}")


def run_server(port: int = 8765, host: str = '127.0.0.1'):
    """启动 HTTP 服务器"""
    httpd = None
    try:
        server_address = (host, port)
        httpd = HTTPServer(server_address, AIRequestHandler)
        
        print(f"🤖 AI 安全助手服务已启动", flush=True)
        print(f"📍 地址: http://{host}:{port}", flush=True)
        print(f"🔗 健康检查: http://{host}:{port}/health", flush=True)
        print(f"📋 可用提供商: http://{host}:{port}/providers", flush=True)
        print(f"💬 聊天接口: http://{host}:{port}/chat", flush=True)
        print("\n按 Ctrl+C 停止服务\n", flush=True)
        
        httpd.serve_forever()
    except OSError as e:
        if hasattr(e, 'errno') and e.errno == 10048:  # Windows: Address already in use
            print(f"❌ 错误: 端口 {port} 已被占用", file=sys.stderr, flush=True)
        elif hasattr(e, 'errno') and e.errno == 98:  # Linux: Address already in use
            print(f"❌ 错误: 端口 {port} 已被占用", file=sys.stderr, flush=True)
        else:
            print(f"❌ 启动服务器失败: {e}", file=sys.stderr, flush=True)
        sys.exit(1)
    except KeyboardInterrupt:
        print("\n\n🛑 正在停止服务...", flush=True)
        if httpd:
            httpd.shutdown()
        print("✅ 服务已停止", flush=True)
    except Exception as e:
        print(f"❌ 服务器运行错误: {e}", file=sys.stderr, flush=True)
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='AI 安全助手服务')
    parser.add_argument('--port', type=int, default=8765, help='服务端口 (默认: 8765)')
    parser.add_argument('--host', type=str, default='127.0.0.1', help='服务地址 (默认: 127.0.0.1)')
    
    args = parser.parse_args()
    
    run_server(port=args.port, host=args.host)

