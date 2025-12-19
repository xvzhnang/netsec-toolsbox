# -*- coding: utf-8 -*-
"""
OpenAI-compatible API Handler
对应 One API 的 router/relay.go + controller/relay.go
"""
import json
import asyncio
from http.server import BaseHTTPRequestHandler
from typing import Optional
from urllib.parse import urlparse, parse_qs

import sys
import os

# 添加 ai_service 目录到 Python 路径
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.router import Router
from core.adapter.base_adapter import OpenAIChatRequest


class AIRequestHandler(BaseHTTPRequestHandler):
    """OpenAI-compatible API Handler"""
    
    def __init__(self, *args, router: Router, **kwargs):
        """
        初始化 Handler
        
        Args:
            router: 路由器实例
        """
        self.router = router
        super().__init__(*args, **kwargs)
    
    def do_OPTIONS(self):
        """处理 CORS 预检请求"""
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
        self.end_headers()
    
    def do_GET(self):
        """处理 GET 请求"""
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/v1/models':
            self._handle_list_models()
        elif parsed_path.path == '/health':
            self._send_json_response({"status": "ok"})
        else:
            self._send_error(404, "Not Found")
    
    def do_POST(self):
        """处理 POST 请求"""
        parsed_path = urlparse(self.path)
        
        if parsed_path.path == '/v1/chat/completions':
            self._handle_chat_completions()
        else:
            self._send_error(404, "Not Found")
    
    def _handle_chat_completions(self):
        """处理 /v1/chat/completions 请求"""
        try:
            # 解析请求体（对应 One API 的 getAndValidateTextRequest）
            content_length = int(self.headers.get('Content-Length', 0))
            if content_length == 0:
                self._send_error(400, "Request body is required")
                return
            
            request_body = self.rfile.read(content_length)
            request_data = json.loads(request_body.decode('utf-8'))
            
            # 提取 model 字段（对应 One API 的 meta.OriginModelName）
            model_id = request_data.get('model')
            if not model_id:
                self._send_error(400, "Missing 'model' field")
                return
            
            # 构建请求对象
            chat_request = OpenAIChatRequest(
                model=model_id,
                messages=request_data.get('messages', []),
                temperature=request_data.get('temperature'),
                max_tokens=request_data.get('max_tokens'),
                stream=request_data.get('stream', False),
                top_p=request_data.get('top_p'),
                frequency_penalty=request_data.get('frequency_penalty'),
                presence_penalty=request_data.get('presence_penalty'),
                stop=request_data.get('stop'),
                user=request_data.get('user')
            )
            
            # 路由到适配器（对应 One API 的 RelayTextHelper）
            # 使用 asyncio 运行异步方法
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            try:
                response = loop.run_until_complete(
                    self.router.route(model_id, chat_request)
                )
                
                # 转换为字典格式
                response_dict = {
                    "id": response.id,
                    "object": response.object,
                    "created": response.created,
                    "model": response.model,
                    "choices": response.choices,
                }
                if response.usage:
                    response_dict["usage"] = response.usage
                
                # 返回响应（已经是 OpenAI 格式）
                self._send_json_response(response_dict)
            finally:
                loop.close()
        
        except ValueError as e:
            self._send_error(404, str(e))
        except Exception as e:
            error_msg = str(e)
            # 过滤掉敏感信息
            if 'api_key' in error_msg.lower() or 'key' in error_msg.lower():
                error_msg = "API 配置错误"
            self._send_error(500, error_msg)
    
    def _handle_list_models(self):
        """处理 /v1/models 请求"""
        models = self.router.registry.list_models()
        self._send_json_response(models)
    
    def _send_json_response(self, data: dict):
        """发送 JSON 响应"""
        response = json.dumps(data, ensure_ascii=False).encode('utf-8')
        self.send_response(200)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization')
        self.end_headers()
        self.wfile.write(response)
    
    def _send_error(self, status_code: int, message: str):
        """发送错误响应（OpenAI 格式）"""
        error_response = {
            "error": {
                "message": message,
                "type": "invalid_request_error" if status_code < 500 else "server_error",
                "code": str(status_code)
            }
        }
        response = json.dumps(error_response, ensure_ascii=False).encode('utf-8')
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json; charset=utf-8')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(response)
    
    def log_message(self, format, *args):
        """重写日志方法，避免输出到 stderr"""
        # 可以在这里添加自定义日志逻辑
        pass

