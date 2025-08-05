import axios from 'axios'
import fs from 'fs/promises'
import path from 'path'
import os from 'os'
import { Provider, ProviderStatus } from '../shared/types'

export async function checkProviderStatus(provider: Provider): Promise<ProviderStatus> {
  const startTime = Date.now()
  
  try {
    // 方法1: 先检查 Anthropic 官方状态 (适用于 Anthropic API)
    if (provider.apiUrl.includes('anthropic.com')) {
      try {
        const statusResponse = await axios.get('https://status.anthropic.com/api/v2/summary.json', {
          timeout: 3000
        })
        
        if (statusResponse.data.status?.indicator !== 'none') {
          return {
            isOnline: false,
            responseTime: -1,
            lastChecked: new Date(),
            error: 'Anthropic 服务当前不可用'
          }
        }
      } catch {
        // 状态检查失败，继续尝试直接 API 调用
      }
    }

    // 方法2: 轻量级 API 测试请求 - 使用最新的 API 格式
    const testPayload = {
      model: provider.model || 'claude-sonnet-4-20250514',
      max_tokens: 1,
      messages: [
        {
          role: 'user',
          content: 'test'
        }
      ]
    }

    const response = await axios.post(
      `${provider.apiUrl}/v1/messages`,
      testPayload,
      {
        headers: {
          'x-api-key': provider.apiKey,
          'anthropic-version': '2023-06-01',
          'content-type': 'application/json'
        },
        timeout: 20000, // 增加超时时间到20秒
        validateStatus: (status) => {
          // 200-299 为成功，400-499 通常表示 API 可用但请求有问题（如 key 无效）
          return status < 500
        }
      }
    )

    const responseTime = Date.now() - startTime
    
    // 检查响应状态
    if (response.status >= 200 && response.status < 300) {
      return {
        isOnline: true,
        responseTime,
        lastChecked: new Date()
      }
    } else if (response.status >= 400 && response.status < 500) {
      // 客户端错误，API 可用但可能是 key 无效或其他认证问题
      return {
        isOnline: true, // API 本身是可用的
        responseTime,
        lastChecked: new Date(),
        error: `API 可用但认证失败 (${response.status}): ${response.data?.error?.message || '请检查 API Key'}`
      }
    } else {
      return {
        isOnline: false,
        responseTime,
        lastChecked: new Date(),
        error: `服务器错误 (${response.status})`
      }
    }
  } catch (error) {
    const responseTime = Date.now() - startTime
    
    if (axios.isAxiosError(error)) {
      if (error.code === 'ECONNABORTED' || error.code === 'ETIMEDOUT') {
        return {
          isOnline: false,
          responseTime,
          lastChecked: new Date(),
          error: '请求超时 - 服务可能不可用'
        }
      } else if (error.response) {
        // 服务器响应了错误状态码
        return {
          isOnline: false,
          responseTime,
          lastChecked: new Date(),
          error: `HTTP ${error.response.status}: ${error.response.data?.error?.message || error.message}`
        }
      } else if (error.request) {
        // 请求发出但没有收到响应
        return {
          isOnline: false,
          responseTime,
          lastChecked: new Date(),
          error: '网络连接失败 - 无法访问服务'
        }
      }
    }
    
    return {
      isOnline: false,
      responseTime,
      lastChecked: new Date(),
      error: error instanceof Error ? error.message : '未知错误'
    }
  }
}

export function getClaudeCodeConfig() {
  // Claude Code 配置文件路径
  const configDir = path.join(os.homedir(), '.claude')
  const configPath = path.join(configDir, 'settings.json')
  
  return { path: configPath, dir: configDir }
}

export async function switchProvider(provider: Provider): Promise<boolean> {
  try {
    const { path: configPath, dir: configDir } = getClaudeCodeConfig()
    
    // 确保目录存在
    await fs.mkdir(configDir, { recursive: true })
    
    // 读取现有配置
    let config: any = {}
    try {
      const content = await fs.readFile(configPath, 'utf-8')
      config = JSON.parse(content)
    } catch {
      // 文件不存在或解析失败，使用空配置
    }
    
    // 更新配置
    config.api = {
      ...config.api,
      baseURL: provider.apiUrl,
      apiKey: provider.apiKey,
      model: provider.model
    }
    
    // 写回配置文件
    await fs.writeFile(configPath, JSON.stringify(config, null, 2))
    
    return true
  } catch (error) {
    console.error('切换供应商失败:', error)
    return false
  }
}