import fs from 'fs/promises'
import path from 'path'
import os from 'os'
import { Provider } from '../shared/types'

interface ClaudeCodeConfig {
  env?: {
    ANTHROPIC_AUTH_TOKEN?: string
    ANTHROPIC_BASE_URL?: string
    [key: string]: string | undefined
  }
  [key: string]: any
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
    let config: ClaudeCodeConfig = {}
    try {
      const content = await fs.readFile(configPath, 'utf-8')
      config = JSON.parse(content)
    } catch {
      // 文件不存在或解析失败，使用空配置
    }

    // 确保 env 对象存在
    if (!config.env) {
      config.env = {}
    }

    // 更新环境变量配置
    config.env.ANTHROPIC_AUTH_TOKEN = provider.apiKey
    config.env.ANTHROPIC_BASE_URL = provider.apiUrl

    // 写回配置文件
    await fs.writeFile(configPath, JSON.stringify(config, null, 2))

    return true
  } catch (error) {
    console.error('切换供应商失败:', error)
    return false
  }
}