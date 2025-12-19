# -*- coding: utf-8 -*-
"""
AI Gateway 测试脚本
"""
import requests
import json

BASE_URL = "http://127.0.0.1:8765"

def test_health():
    """测试健康检查"""
    print("测试 /health...")
    try:
        response = requests.get(f"{BASE_URL}/health", timeout=5)
        print(f"✅ 健康检查成功: {response.json()}")
        return True
    except Exception as e:
        print(f"❌ 健康检查失败: {e}")
        return False

def test_list_models():
    """测试获取模型列表"""
    print("\n测试 /v1/models...")
    try:
        response = requests.get(f"{BASE_URL}/v1/models", timeout=5)
        data = response.json()
        print(f"✅ 获取模型列表成功:")
        print(f"   模型数量: {len(data.get('data', []))}")
        for model in data.get('data', []):
            print(f"   - {model.get('id')} ({model.get('owned_by')})")
        return True
    except Exception as e:
        print(f"❌ 获取模型列表失败: {e}")
        return False

def test_chat_completions():
    """测试聊天接口"""
    print("\n测试 /v1/chat/completions...")
    try:
        # 先获取可用模型
        models_response = requests.get(f"{BASE_URL}/v1/models", timeout=5)
        models_data = models_response.json()
        available_models = [m['id'] for m in models_data.get('data', [])]
        
        if not available_models:
            print("⚠️ 没有可用的模型，跳过聊天测试")
            return True
        
        model_id = available_models[0]
        print(f"   使用模型: {model_id}")
        
        request_data = {
            "model": model_id,
            "messages": [
                {"role": "user", "content": "Hello! Please respond with 'Gateway test successful'."}
            ],
            "temperature": 0.7,
            "max_tokens": 50
        }
        
        response = requests.post(
            f"{BASE_URL}/v1/chat/completions",
            json=request_data,
            timeout=30
        )
        
        if response.status_code == 200:
            data = response.json()
            print(f"✅ 聊天请求成功:")
            if data.get('choices'):
                content = data['choices'][0].get('message', {}).get('content', '')
                print(f"   响应: {content[:100]}...")
            return True
        else:
            error_data = response.json()
            print(f"❌ 聊天请求失败: {error_data.get('error', {}).get('message', 'Unknown error')}")
            return False
    
    except requests.exceptions.ConnectionError:
        print(f"❌ 连接失败: 请确保 AI Gateway 服务正在运行 (http://127.0.0.1:8765)")
        return False
    except Exception as e:
        print(f"❌ 聊天请求失败: {e}")
        return False

if __name__ == '__main__':
    print("=" * 50)
    print("AI Gateway 测试")
    print("=" * 50)
    
    results = []
    results.append(("健康检查", test_health()))
    results.append(("模型列表", test_list_models()))
    results.append(("聊天接口", test_chat_completions()))
    
    print("\n" + "=" * 50)
    print("测试结果汇总:")
    print("=" * 50)
    for name, result in results:
        status = "✅ 通过" if result else "❌ 失败"
        print(f"{name}: {status}")
    
    all_passed = all(result for _, result in results)
    if all_passed:
        print("\n🎉 所有测试通过！")
    else:
        print("\n⚠️ 部分测试失败，请检查配置和服务状态")

