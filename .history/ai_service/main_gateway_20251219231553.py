#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AI Gateway 服务
提供 OpenAI-Compatible API，统一接入在线/本地/非兼容协议的大模型
"""

import json
import os
import sys
import io
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse
import traceback

# 修复 Windows 下的编码问题
if sys.platform == 'win32':
    if sys.stdout.encoding != 'utf-8':
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace', line_buffering=True)
    if sys.stderr.encoding != 'utf-8':
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace', line_buffering=True)

# 导入 Gateway 核心模块
from core.registry import ModelRegistry
from api.openai_handler import OpenAIHandler


class GatewayRequestHandler(BaseHTTPRequestHandler):
    """AI Gateway 请求处理器"""
    
    def __init__(self, *args, **kwargs):
        # 从服务器获取 registry 和 handler
        self.registry = kwargs.pop('registry', None)
        self.openai_handler = kwargs.pop('openai_handler', None)
        try:
            super().__init__(*args, **kwargs)
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError) as e:
            # 静默处理连接中断错误
            pass
        except Exception as e:
            print(f"❌ GatewayRequestHandler 初始化失败: {e}", file=sys.stderr, flush=True)
            traceback.print_exc(file=sys.stderr)
            raise
    
    def do_OPTIONS(self):
        """处理 CORS 预检请求"""
        try:
            self.send_response(200)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
            self.end_headers()
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError, OSError):
            pass
        except Exception as e:
            print(f"⚠️ OPTIONS 请求处理失败: {e}", file=sys.stderr, flush=True)
    
    def do_GET(self):
        """处理 GET 请求"""
        try:
            parsed_path = urlparse(self.path)
            
            if parsed_path.path == '/health':
                self._send_json_response({'status': 'ok'}, 200)
            elif parsed_path.path == '/v1/models':
                # OpenAI-Compatible: /v1/models
                self.openai_handler.handle_models(self)
            else:
                self._send_error('Not found', 404)
        
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError):
            pass
        except Exception as e:
            print(f"⚠️ 处理 GET 请求失败: {e}", file=sys.stderr, flush=True)
            try:
                self._send_error(f'Internal server error: {str(e)}', 500)
            except:
                pass
    
    def do_POST(self):
        """处理 POST 请求"""
        try:
            parsed_path = urlparse(self.path)
            
            if parsed_path.path == '/v1/chat/completions':
                # OpenAI-Compatible: /v1/chat/completions
                try:
                    content_length = int(self.headers.get('Content-Length', 0))
                    body = self.rfile.read(content_length)
                    data = json.loads(body.decode('utf-8'))
                    
                    self.openai_handler.handle_chat_completions(self, data)
                except json.JSONDecodeError:
                    self._send_error('Invalid JSON', 400)
                except Exception as e:
                    error_msg = str(e)
                    print(f"❌ [Gateway] 处理 /v1/chat/completions 失败: {error_msg}", file=sys.stderr, flush=True)
                    traceback.print_exc(file=sys.stderr)
                    self._send_error(error_msg, 500)
            else:
                self._send_error('Not found', 404)
        
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError):
            pass
        except Exception as e:
            print(f"⚠️ 处理 POST 请求失败: {e}", file=sys.stderr, flush=True)
            try:
                self._send_error(f'Internal server error: {str(e)}', 500)
            except:
                pass
    
    def _send_json_response(self, data: dict, status_code: int = 200):
        """发送 JSON 响应"""
        try:
            self.send_response(status_code)
            self.send_header('Content-Type', 'application/json; charset=utf-8')
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
            self.end_headers()
            
            response = json.dumps(data, ensure_ascii=False).encode('utf-8')
            self.wfile.write(response)
            self.wfile.flush()
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError, OSError):
            pass
        except Exception as e:
            error_msg = str(e)
            if 'signal' not in error_msg.lower() and 'SIGALRM' not in error_msg:
                print(f"⚠️ 发送响应失败: {error_msg}", file=sys.stderr, flush=True)
    
    def _send_error(self, message: str, status_code: int = 400):
        """发送错误响应"""
        error_response = {
            "error": {
                "message": message,
                "type": "invalid_request_error",
                "code": status_code
            }
        }
        self._send_json_response(error_response, status_code)
    
    def log_message(self, format, *args):
        """自定义日志格式（静默）"""
        pass


class GatewayHTTPServer(HTTPServer):
    """自定义 HTTP 服务器，传递 registry 和 handler"""
    
    def __init__(self, server_address, RequestHandlerClass, registry, openai_handler):
        self.registry = registry
        self.openai_handler = openai_handler
        super().__init__(server_address, RequestHandlerClass)
    
    def finish_request(self, request, client_address):
        """重写 finish_request 以传递 registry 和 handler"""
        self.RequestHandlerClass(
            request,
            client_address,
            self,
            registry=self.registry,
            openai_handler=self.openai_handler
        )


def run_server(port: int = 8765, host: str = '127.0.0.1'):
    """启动 AI Gateway 服务器"""
    try:
        # 初始化模型注册表
        print("🔄 正在加载模型配置...", flush=True)
        registry = ModelRegistry()
        
        if not registry.adapters:
            print("⚠️ 警告: 没有可用的模型，请检查配置文件", flush=True)
        
        # 初始化 OpenAI Handler
        openai_handler = OpenAIHandler(registry)
        
        # 创建服务器
        server_address = (host, port)
        httpd = GatewayHTTPServer(server_address, GatewayRequestHandler, registry, openai_handler)
        
        print(f"\n🤖 AI Gateway 服务已启动", flush=True)
        print(f"📍 地址: http://{host}:{port}", flush=True)
        print(f"🔗 健康检查: http://{host}:{port}/health", flush=True)
        print(f"📋 模型列表: http://{host}:{port}/v1/models", flush=True)
        print(f"💬 聊天接口: http://{host}:{port}/v1/chat/completions", flush=True)
        print(f"✅ 已加载 {len(registry.adapters)} 个模型", flush=True)
        print("\n按 Ctrl+C 停止服务\n", flush=True)
        
        httpd.serve_forever()
    
    except OSError as e:
        if hasattr(e, 'errno') and e.errno in [10048, 98]:  # Address already in use
            print(f"❌ 错误: 端口 {port} 已被占用", file=sys.stderr, flush=True)
        else:
            print(f"❌ 启动服务器失败: {e}", file=sys.stderr, flush=True)
        sys.exit(1)
    except KeyboardInterrupt:
        print("\n\n🛑 正在停止服务...", flush=True)
        print("✅ 服务已停止", flush=True)
    except Exception as e:
        print(f"❌ 服务器运行错误: {e}", file=sys.stderr, flush=True)
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    import argparse
    
    parser = argparse.ArgumentParser(description='AI Gateway 服务')
    parser.add_argument('--port', type=int, default=8765, help='服务端口 (默认: 8765)')
    parser.add_argument('--host', type=str, default='127.0.0.1', help='服务地址 (默认: 127.0.0.1)')
    
    args = parser.parse_args()
    
    run_server(port=args.port, host=args.host)

