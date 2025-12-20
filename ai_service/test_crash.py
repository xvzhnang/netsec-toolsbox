# -*- coding: utf-8 -*-
"""
最小崩溃示例 - 用于测试和对比
演示可能导致进程崩溃的代码模式
"""
from http.server import HTTPServer, BaseHTTPRequestHandler
import asyncio
import sys

class CrashHandler(BaseHTTPRequestHandler):
    """会崩溃的 Handler - 用于测试"""
    
    def do_POST(self):
        # ❌ 危险模式 1：事件循环关闭时仍有未完成的协程
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            async def crash_task():
                await asyncio.sleep(0.1)
                raise Exception("模拟崩溃")
            
            loop.run_until_complete(crash_task())
        finally:
            loop.close()  # ⚠️ 这里可能崩溃

if __name__ == '__main__':
    server = HTTPServer(('127.0.0.1', 8766), CrashHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        server.shutdown()

