#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AI 安全助手服务
提供 HTTP API 接口，支持多个 AI 模型提供商
"""

import json
import os
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
import urllib.error
import threading
from typing import Optional, Dict, Any
import traceback
import io
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError

# 修复 Windows 下的编码问题
if sys.platform == 'win32':
    # 重新配置 stdout 和 stderr 为 UTF-8
    if sys.stdout.encoding != 'utf-8':
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace', line_buffering=True)
    if sys.stderr.encoding != 'utf-8':
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace', line_buffering=True)

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
    from providers.claude_provider import ClaudeProvider
    HAS_CLAUDE = True
except ImportError:
    HAS_CLAUDE = False

try:
    from providers.gemini_provider import GeminiProvider
    HAS_GEMINI = True
except ImportError:
    HAS_GEMINI = False

try:
    from providers.zhipu_provider import ZhipuProvider
    HAS_ZHIPU = True
except ImportError:
    HAS_ZHIPU = False

try:
    from providers.qwen_provider import QwenProvider
    HAS_QWEN = True
except ImportError:
    HAS_QWEN = False

try:
    from providers.mistral_provider import MistralProvider
    HAS_MISTRAL = True
except ImportError:
    HAS_MISTRAL = False

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


# 全局 providers 缓存（避免每次请求都重新初始化）
_global_providers = {}
_providers_lock = threading.Lock()
_last_config_mtime = 0

def _get_providers():
    """获取或初始化 providers（线程安全）"""
    global _global_providers, _last_config_mtime
    
    # 获取配置文件路径
    config_dir = os.environ.get('NETSEC_TOOLBOX_CONFIG_DIR')
    if config_dir:
        config_path = os.path.join(config_dir, 'ai.json')
    elif sys.platform == 'win32':
        appdata = os.environ.get('APPDATA', '')
        if appdata:
            config_path = os.path.join(appdata, 'netsec-toolbox', '.config', 'ai.json')
        else:
            config_path = os.path.join(os.path.dirname(__file__), 'ai_config.json')
    else:
        config_path = os.path.join(os.path.dirname(__file__), 'ai_config.json')
    
    # 检查配置文件是否更新
    current_mtime = 0
    if os.path.exists(config_path):
        current_mtime = os.path.getmtime(config_path)
    
    # 如果配置未更新且 providers 已初始化，直接返回
    with _providers_lock:
        if current_mtime == _last_config_mtime and _global_providers:
            return _global_providers
        
        # 重新加载配置
        config = {}
        if os.path.exists(config_path):
            try:
                with open(config_path, 'r', encoding='utf-8') as f:
                    config = json.load(f)
            except Exception as e:
                print(f"⚠️ 加载配置文件失败: {e}", file=sys.stderr, flush=True)
        
        # 初始化 providers
        providers = {}
        if HAS_OPENAI and config.get('openai', {}).get('api_key'):
            try:
                providers['openai'] = OpenAIProvider(config.get('openai', {}))
            except Exception as e:
                print(f"⚠️ 初始化 OpenAI provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_DEEPSEEK and config.get('deepseek', {}).get('api_key'):
            try:
                providers['deepseek'] = DeepSeekProvider(config.get('deepseek', {}))
            except Exception as e:
                print(f"⚠️ 初始化 DeepSeek provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_CLAUDE and config.get('claude', {}).get('api_key'):
            try:
                providers['claude'] = ClaudeProvider(config.get('claude', {}))
            except Exception as e:
                print(f"⚠️ 初始化 Claude provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_GEMINI and config.get('gemini', {}).get('api_key'):
            try:
                providers['gemini'] = GeminiProvider(config.get('gemini', {}))
            except Exception as e:
                print(f"⚠️ 初始化 Gemini provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_ZHIPU and config.get('zhipu', {}).get('api_key'):
            try:
                providers['zhipu'] = ZhipuProvider(config.get('zhipu', {}))
            except Exception as e:
                print(f"⚠️ 初始化智谱AI provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_QWEN and config.get('qwen', {}).get('api_key'):
            try:
                providers['qwen'] = QwenProvider(config.get('qwen', {}))
            except Exception as e:
                print(f"⚠️ 初始化通义千问 provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_MISTRAL and config.get('mistral', {}).get('api_key'):
            try:
                providers['mistral'] = MistralProvider(config.get('mistral', {}))
            except Exception as e:
                print(f"⚠️ 初始化 Mistral provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_OLLAMA and config.get('ollama', {}).get('api_url'):
            try:
                providers['ollama'] = OllamaProvider(config.get('ollama', {}))
            except Exception as e:
                print(f"⚠️ 初始化 Ollama provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_LMSTUDIO and config.get('lmstudio', {}).get('api_url'):
            try:
                providers['lmstudio'] = LMStudioProvider(config.get('lmstudio', {}))
            except Exception as e:
                print(f"⚠️ 初始化 LM Studio provider 失败: {e}", file=sys.stderr, flush=True)
        
        if HAS_LLAMACPP and config.get('llamacpp', {}).get('api_url'):
            try:
                providers['llamacpp'] = LlamaCppProvider(config.get('llamacpp', {}))
            except Exception as e:
                print(f"⚠️ 初始化 llama.cpp provider 失败: {e}", file=sys.stderr, flush=True)
        
        # 加载自定义提供商（custom_providers 配置项）
        try:
            from providers.custom_provider import CustomProvider
            custom_providers = config.get('custom_providers', {})
            for provider_id, provider_config in custom_providers.items():
                if provider_config.get('enabled', True):
                    try:
                        # 添加 provider_id 到配置中
                        provider_config['name'] = provider_id
                        providers[provider_id] = CustomProvider(provider_config)
                    except Exception as e:
                        print(f"⚠️ 初始化自定义提供商 {provider_id} 失败: {e}", file=sys.stderr, flush=True)
        except ImportError:
            pass  # 如果导入失败，忽略自定义提供商
        
        _global_providers = providers
        _last_config_mtime = current_mtime
        
        return providers


class AIRequestHandler(BaseHTTPRequestHandler):
    """处理 AI API 请求"""
    
    def __init__(self, *args, **kwargs):
        try:
            super().__init__(*args, **kwargs)
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError) as e:
            # 静默处理连接中断错误（客户端提前关闭连接是正常的）
            pass
        except Exception as e:
            print(f"❌ AIRequestHandler 初始化失败: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            raise
    
    @property
    def providers(self):
        """获取 providers（延迟加载）"""
        return _get_providers()
    
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
        try:
            self.send_response(status_code)
            self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type')
            self.end_headers()
            
            response = json.dumps(data, ensure_ascii=False).encode('utf-8')
            self.wfile.write(response)
            self.wfile.flush()
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError):
            # 客户端提前关闭连接，静默处理
            pass
        except Exception as e:
            print(f"⚠️ 发送响应失败: {e}", file=sys.stderr, flush=True)
    
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
        try:
            parsed_path = urlparse(self.path)
            
            if parsed_path.path == '/health':
                providers = self.providers
                self._send_json_response({
                    'success': True,
                    'status': 'healthy',
                    'providers': list(providers.keys())
                })
            elif parsed_path.path == '/providers':
                providers = self.providers
                self._send_json_response({
                    'success': True,
                    'providers': list(providers.keys()),
                    'available': {
                        'openai': HAS_OPENAI,
                        'deepseek': HAS_DEEPSEEK,
                        'claude': HAS_CLAUDE,
                        'gemini': HAS_GEMINI,
                        'zhipu': HAS_ZHIPU,
                        'qwen': HAS_QWEN,
                        'mistral': HAS_MISTRAL,
                        'ollama': HAS_OLLAMA,
                        'lmstudio': HAS_LMSTUDIO,
                        'llamacpp': HAS_LLAMACPP
                    }
                })
            else:
                self._send_error('Not found', 404)
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError):
            # 客户端提前关闭连接，静默处理
            pass
        except Exception as e:
            print(f"⚠️ 处理 GET 请求失败: {e}", file=sys.stderr, flush=True)
            try:
                self._send_error(f'Internal server error: {str(e)}', 500)
            except:
                pass
    
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
                wiki_context = data.get('wiki_context', None)  # Wiki 上下文
                timeout = data.get('timeout', None)  # 自定义超时
                
                if provider_name not in self.providers:
                    self._send_error(f'Provider "{provider_name}" not available', 400)
                    return
                
                # 如果有 Wiki 上下文，添加到系统消息中
                if wiki_context:
                    system_message = {
                        'role': 'system',
                        'content': f'以下是相关的 Wiki 文档内容，请参考这些信息回答问题：\n\n{wiki_context}'
                    }
                    messages = [system_message] + messages
                
                provider = self.providers[provider_name]
                
                # 调用模型生成回复（带超时处理，跨平台兼容）
                if timeout:
                    # 使用 ThreadPoolExecutor 实现跨平台超时
                    with ThreadPoolExecutor(max_workers=1) as executor:
                        future = executor.submit(provider.chat, messages)
                        try:
                            response_text = future.result(timeout=timeout)
                        except FutureTimeoutError:
                            raise TimeoutError(f'AI 请求超时（{timeout}秒）')
                else:
                    response_text = provider.chat(messages)
                
                self._send_json_response({
                    'success': True,
                    'response': response_text
                })
                
            except TimeoutError as e:
                self._send_error(str(e), 408)  # Request Timeout
            except urllib.error.URLError as e:
                if 'timed out' in str(e).lower() or 'timeout' in str(e).lower():
                    self._send_error(f'请求超时: {str(e)}', 408)
                else:
                    self._send_error(f'网络错误: {str(e)}', 500)
            except json.JSONDecodeError:
                self._send_error('Invalid JSON', 400)
            except Exception as e:
                error_msg = str(e)
                traceback.print_exc()
                self._send_error(f'Internal error: {error_msg}', 500)
        elif parsed_path.path == '/wiki':
            # Wiki 内容读取接口
            try:
                content_length = int(self.headers.get('Content-Length', 0))
                body = self.rfile.read(content_length)
                data = json.loads(body.decode('utf-8'))
                
                file_path = data.get('file_path')
                if not file_path:
                    self._send_error('file_path is required', 400)
                    return
                
                # 通过环境变量获取配置目录，然后读取 Wiki 文件
                wiki_content = self._read_wiki_file(file_path)
                
                self._send_json_response({
                    'success': True,
                    'content': wiki_content
                })
            except json.JSONDecodeError:
                self._send_error('Invalid JSON', 400)
            except Exception as e:
                error_msg = str(e)
                self._send_error(f'Failed to read wiki: {error_msg}', 500)
        else:
            self._send_error('Not found', 404)
    
    def _read_wiki_file(self, file_path: str) -> str:
        """读取 Wiki 文件内容（通过环境变量获取 Wiki 目录）"""
        # 获取 Wiki 目录（由 Tauri 后端通过环境变量传递）
        wiki_dir = os.environ.get('NETSEC_TOOLBOX_WIKI_DIR')
        if not wiki_dir:
            # 尝试从配置目录推导
            config_dir = os.environ.get('NETSEC_TOOLBOX_CONFIG_DIR')
            if config_dir:
                # 假设 wiki 目录在项目根目录下
                wiki_dir = os.path.join(os.path.dirname(config_dir), 'wiki')
        
        if not wiki_dir or not os.path.exists(wiki_dir):
            raise FileNotFoundError(f'Wiki 目录不存在: {wiki_dir}')
        
        # 规范化路径（移除前导斜杠和反斜杠）
        normalized_path = file_path.lstrip('/\\')
        full_path = os.path.join(wiki_dir, normalized_path)
        
        # 安全检查：确保路径在 wiki_dir 内（防止路径遍历攻击）
        full_path = os.path.normpath(full_path)
        wiki_dir = os.path.normpath(wiki_dir)
        if not full_path.startswith(wiki_dir):
            raise ValueError(f'非法路径: {file_path}')
        
        if not os.path.exists(full_path):
            raise FileNotFoundError(f'Wiki 文件不存在: {file_path}')
        
        if not os.path.isfile(full_path):
            raise ValueError(f'路径不是文件: {file_path}')
        
        with open(full_path, 'r', encoding='utf-8') as f:
            return f.read()
    
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

