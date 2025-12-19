"""
Process Adapter
适用于本地 CLI 或二进制模型，通过子进程调用
"""

import subprocess
import time
import uuid
import json
from typing import Dict, Any, Optional, List
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeoutError

from .trait import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse


class ProcessAdapter(ChatAdapter):
    """
    Process Adapter
    
    适用于本地 CLI 或二进制模型，通过子进程调用
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.command = config.get('command', [])  # 命令列表，如 ['llama.cpp', '--model', 'model.bin']
        self.working_dir = config.get('working_dir', None)
        self.env = config.get('env', None)  # 环境变量字典
        self.input_format = config.get('input_format', 'prompt')  # 'prompt' 或 'json'
        self.output_format = config.get('output_format', 'text')  # 'text' 或 'json'
        self.prompt_template = config.get('prompt_template', '{prompt}')  # prompt 模板
    
    def is_available(self) -> bool:
        """检查 Adapter 是否可用"""
        if not self.command:
            return False
        
        # 检查命令是否存在（简单检查第一个命令）
        import shutil
        return shutil.which(self.command[0]) is not None
    
    def _format_prompt(self, messages: List[Dict[str, str]]) -> str:
        """格式化消息为 prompt"""
        if self.input_format == 'json':
            return json.dumps(messages)
        
        # 默认拼接为 prompt
        prompt_parts = []
        for msg in messages:
            role = msg.get('role', 'user')
            content = msg.get('content', msg.get('text', ''))
            if role == 'system':
                prompt_parts.append(f"System: {content}")
            elif role == 'user':
                prompt_parts.append(f"User: {content}")
            elif role == 'assistant':
                prompt_parts.append(f"Assistant: {content}")
        
        prompt = "\n\n".join(prompt_parts)
        return self.prompt_template.format(prompt=prompt)
    
    def chat(self, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """发送聊天请求"""
        if not self.is_available():
            raise Exception(f"Adapter {self.model_id} 不可用，请检查配置")
        
        # 格式化 prompt
        prompt = self._format_prompt(request.messages)
        
        # 构建环境变量
        env = None
        if self.env:
            import os
            env = {**os.environ, **self.env}
        
        # 执行命令（使用跨平台超时）
        request_timeout = request.timeout or self.config.get('timeout', 120)
        
        def _call_process():
            try:
                result = subprocess.run(
                    self.command,
                    input=prompt.encode('utf-8'),
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=request_timeout,
                    cwd=self.working_dir,
                    env=env
                )
                
                if result.returncode != 0:
                    error_msg = result.stderr.decode('utf-8', errors='replace')
                    raise Exception(f'进程执行失败 (exit code {result.returncode}): {error_msg}')
                
                output = result.stdout.decode('utf-8', errors='replace').strip()
                return output
            except subprocess.TimeoutExpired:
                raise TimeoutError(f'进程执行超时（{request_timeout}秒）')
            except Exception as e:
                raise Exception(f'进程执行失败: {str(e)}')
        
        with ThreadPoolExecutor(max_workers=1) as executor:
            future = executor.submit(_call_process)
            try:
                output = future.result(timeout=request_timeout + 5)
            except FutureTimeoutError:
                raise TimeoutError(f'AI 请求超时（{request_timeout}秒）')
        
        # 解析输出
        if self.output_format == 'json':
            try:
                output_data = json.loads(output)
                content = output_data.get('text', output_data.get('content', output))
            except json.JSONDecodeError:
                content = output
        else:
            content = output
        
        # 转换为 OpenAI 响应格式
        return OpenAIChatResponse(
            id=f"chatcmpl-{uuid.uuid4().hex[:8]}",
            object="chat.completion",
            created=int(time.time()),
            model=request.model,
            choices=[{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            usage=None
        )

