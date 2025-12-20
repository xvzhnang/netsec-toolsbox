# -*- coding: utf-8 -*-
"""
Process 适配器
用于本地命令行工具（如 llama.cpp, ollama CLI 等）
通过 stdin/stdout 与进程通信
对应 One API 中可能的本地工具集成
"""
import os
import json
import subprocess
import time
from typing import Dict, Any, Optional

from .base_adapter import ChatAdapter, OpenAIChatRequest, OpenAIChatResponse


class ProcessAdapter(ChatAdapter):
    """
    Process 适配器
    用于执行本地命令行工具，通过 stdin/stdout 通信
    """
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        
        # 进程配置
        self.command = config.get('command', '')  # 可执行文件路径
        self.args = config.get('args', [])  # 命令行参数列表
        self.working_dir = config.get('working_dir', None)  # 工作目录
        self.env = config.get('env', None)  # 环境变量（可选）
        
        # 请求格式配置
        self.input_format = config.get('input_format', 'json')  # 输入格式：json, prompt, openai
        self.output_format = config.get('output_format', 'json')  # 输出格式：json, text
        
        # 支持环境变量解析
        if isinstance(self.command, str) and self.command.startswith('ENV:'):
            env_var = self.command[4:]
            self.command = os.environ.get(env_var, '')
        
        # 解析 args 中的环境变量
        if self.args:
            parsed_args = []
            for arg in self.args:
                if isinstance(arg, str) and arg.startswith('ENV:'):
                    env_var = arg[4:]
                    parsed_args.append(os.environ.get(env_var, arg))
                else:
                    parsed_args.append(arg)
            self.args = parsed_args
        
        # 默认超时
        self.default_timeout = config.get('timeout', 120)
    
    @property
    def adapter_type(self) -> str:
        return "process"
    
    def is_available(self) -> bool:
        """检查适配器是否可用"""
        if not self.command:
            return False
        
        # 检查命令是否存在
        if os.path.isfile(self.command):
            return True
        
        # 检查是否在 PATH 中
        import shutil
        return shutil.which(self.command) is not None
    
    def _format_request_as_input(self, request: OpenAIChatRequest) -> str:
        """
        将 OpenAI 请求格式转换为 CLI 工具的输入格式
        
        Args:
            request: OpenAI 格式的请求
        
        Returns:
            格式化后的输入字符串
        """
        if self.input_format == 'json':
            # JSON 格式：直接序列化整个请求
            request_dict = {
                "model": request.model,
                "messages": request.messages,
                "temperature": request.temperature,
                "max_tokens": request.max_tokens,
                "stream": request.stream,
                "top_p": request.top_p,
                "frequency_penalty": request.frequency_penalty,
                "presence_penalty": request.presence_penalty,
                "stop": request.stop,
                "user": request.user
            }
            # 移除 None 值
            request_dict = {k: v for k, v in request_dict.items() if v is not None}
            return json.dumps(request_dict, ensure_ascii=False)
        
        elif self.input_format == 'prompt':
            # Prompt 格式：提取所有消息并拼接成单一 prompt
            prompt_parts = []
            for msg in request.messages:
                role = msg.get("role", "")
                content = msg.get("content", "")
                
                if isinstance(content, str):
                    text = content
                elif isinstance(content, list):
                    text_parts = []
                    for part in content:
                        if isinstance(part, dict) and part.get("type") == "text":
                            text_parts.append(part.get("text", ""))
                    text = " ".join(text_parts)
                else:
                    text = str(content)
                
                if role == "system":
                    prompt_parts.append(f"System: {text}")
                elif role == "user":
                    prompt_parts.append(f"User: {text}")
                elif role == "assistant":
                    prompt_parts.append(f"Assistant: {text}")
            
            return "\n".join(prompt_parts)
        
        elif self.input_format == 'openai':
            # OpenAI 格式：只发送 messages
            return json.dumps({"messages": request.messages}, ensure_ascii=False)
        
        else:
            # 默认：只发送最后一个 user 消息
            last_user_msg = None
            for msg in reversed(request.messages):
                if msg.get("role") == "user":
                    content = msg.get("content", "")
                    if isinstance(content, str):
                        last_user_msg = content
                    elif isinstance(content, list):
                        text_parts = []
                        for part in content:
                            if isinstance(part, dict) and part.get("type") == "text":
                                text_parts.append(part.get("text", ""))
                        last_user_msg = " ".join(text_parts)
                    break
            
            return last_user_msg or ""
    
    def _parse_output_as_response(self, output: str, request: OpenAIChatRequest) -> OpenAIChatResponse:
        """
        将 CLI 工具的输出解析为 OpenAI 格式的响应
        
        Args:
            output: CLI 工具的标准输出
            request: 原始请求（用于获取 model 等字段）
        
        Returns:
            OpenAI 格式的响应
        """
        output = output.strip()
        
        if self.output_format == 'json':
            try:
                output_data = json.loads(output)
                
                # 如果已经是 OpenAI 格式，直接使用
                if isinstance(output_data, dict) and "choices" in output_data:
                    return OpenAIChatResponse(
                        id=output_data.get("id", f"process-{int(time.time())}"),
                        object=output_data.get("object", "chat.completion"),
                        created=output_data.get("created", int(time.time())),
                        model=output_data.get("model", request.model),
                        choices=output_data.get("choices", []),
                        usage=output_data.get("usage")
                    )
                
                # 如果是简单的 { "content": "..." } 格式
                if "content" in output_data:
                    return OpenAIChatResponse(
                        id=f"process-{int(time.time())}",
                        object="chat.completion",
                        created=int(time.time()),
                        model=request.model,
                        choices=[{
                            "index": 0,
                            "message": {
                                "role": "assistant",
                                "content": output_data["content"]
                            },
                            "finish_reason": "stop"
                        }],
                        usage=None
                    )
                
                # 尝试其他格式
                return OpenAIChatResponse(
                    id=f"process-{int(time.time())}",
                    object="chat.completion",
                    created=int(time.time()),
                    model=request.model,
                    choices=[{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": str(output_data)
                        },
                        "finish_reason": "stop"
                    }],
                    usage=None
                )
            except json.JSONDecodeError:
                # 如果不是 JSON，当作纯文本处理
                pass
        
        # 纯文本格式：直接作为响应内容
        return OpenAIChatResponse(
            id=f"process-{int(time.time())}",
            object="chat.completion",
            created=int(time.time()),
            model=request.model,
            choices=[{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": output
                },
                "finish_reason": "stop"
            }],
            usage=None
        )
    
    def _run_process_with_timeout(
        self,
        input_data: str,
        timeout: Optional[int] = None
    ) -> tuple[str, str, int]:
        """
        运行进程并返回 stdout, stderr, return_code
        使用 ThreadPoolExecutor 实现跨平台超时
        
        Args:
            input_data: 输入数据（发送到 stdin）
            timeout: 超时时间（秒）
        
        Returns:
            (stdout, stderr, return_code)
        """
        if not timeout:
            timeout = self.default_timeout
        
        # 构建完整的命令
        full_command = [self.command] + self.args
        
        def run_process():
            """在单独线程中运行进程"""
            try:
                process = subprocess.Popen(
                    full_command,
                    stdin=subprocess.PIPE,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    encoding='utf-8',
                    errors='replace',
                    cwd=self.working_dir,
                    env=self.env
                )
                
                stdout, stderr = process.communicate(input=input_data, timeout=timeout)
                return stdout, stderr, process.returncode
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                raise FutureTimeoutError(f"Process timeout after {timeout} seconds")
            except Exception as e:
                raise Exception(f"Process execution failed: {str(e)}")
        
        # 使用 ThreadPoolExecutor 实现超时
        with ThreadPoolExecutor(max_workers=1) as executor:
            future = executor.submit(run_process)
            try:
                stdout, stderr, return_code = future.result(timeout=timeout)
                return stdout, stderr, return_code
            except FutureTimeoutError:
                raise Exception(f"Process timeout after {timeout} seconds")
            except Exception as e:
                raise Exception(f"Process execution failed: {str(e)}")
    
    async def chat(
        self,
        request: OpenAIChatRequest,
        timeout: Optional[int] = None
    ) -> OpenAIChatResponse:
        """
        执行本地命令行工具并获取回复
        
        Args:
            request: OpenAI 格式的请求
            timeout: 超时时间（秒），None 表示使用默认值
        
        Returns:
            OpenAI 格式的响应
        """
        if not self.is_available():
            raise ValueError(f"模型 {self.model_id} 的命令不存在或不可用: {self.command}")
        
        # 格式化请求为输入
        input_data = self._format_request_as_input(request)
        
        # 运行进程（同步调用，因为在 ThreadPoolExecutor 中已经是异步执行）
        # 注意：虽然方法是 async，但 subprocess 调用是同步的
        # 我们可以使用 asyncio.to_thread 在 Python 3.9+ 中真正异步化
        try:
            import asyncio
            stdout, stderr, return_code = await asyncio.to_thread(
                self._run_process_with_timeout,
                input_data,
                timeout or self.default_timeout
            )
        except AttributeError:
            # Python < 3.9，使用 run_in_executor
            import asyncio
            loop = asyncio.get_event_loop()
            stdout, stderr, return_code = await loop.run_in_executor(
                None,
                self._run_process_with_timeout,
                input_data,
                timeout or self.default_timeout
            )
        
        # 检查返回码
        if return_code != 0:
            error_msg = stderr or f"Process exited with code {return_code}"
            raise Exception(f"Process execution failed (code {return_code}): {error_msg}")
        
        # 解析输出为响应
        response = self._parse_output_as_response(stdout, request)
        
        return response

