import axios from 'axios'
import fs from 'fs/promises'
import path from 'path'
import os from 'os'
import { Provider, ProviderStatus } from '../shared/types'

export async function checkProviderStatus(provider: Provider): Promise<ProviderStatus> {
  const startTime = Date.now()
  
  try {
    // 简单的健康检查请求
    const response = await axios.post(
      `${provider.apiUrl}/v1/messages`,
      {
        model: provider.model || 'claude-3-opus-20240229',
        messages: [{ role: 'user', content: 'Hi' }],
        max_tokens: 1
      },
      {
        headers: {
          'x-api-key': provider.apiKey,
          'anthropic-version': '2023-06-01',
          'content-type': 'application/json'
        },
        timeout: 5000
      }
    )

    const responseTime = Date.now() - startTime
    
    return {
      isOnline: true,
      responseTime,
      lastChecked: new Date()
    }
  } catch (error) {
    return {
      isOnline: false,
      responseTime: -1,
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