import fs from 'fs/promises'
import path from 'path'
import os from 'os'
import { AppConfig } from '../shared/types'

export class SimpleStore {
  private configPath: string
  private configDir: string
  private data: AppConfig = { providers: {}, current: '' }
  private initPromise: Promise<void>

  constructor() {
    this.configDir = path.join(os.homedir(), '.cc-switch')
    this.configPath = path.join(this.configDir, 'config.json')
    // 立即开始加载，但不阻塞构造函数
    this.initPromise = this.loadData()
  }

  private async loadData(): Promise<void> {
    try {
      const content = await fs.readFile(this.configPath, 'utf-8')
      this.data = JSON.parse(content)
    } catch (error) {
      // 文件不存在或格式错误，使用默认数据
      this.data = { providers: {}, current: '' }
      await this.saveData()
    }
  }

  private async saveData(): Promise<void> {
    try {
      // 确保目录存在
      await fs.mkdir(this.configDir, { recursive: true })
      // 写入配置文件
      await fs.writeFile(this.configPath, JSON.stringify(this.data, null, 2), 'utf-8')
    } catch (error) {
      console.error('保存配置失败:', error)
    }
  }

  async get<T>(key: keyof AppConfig, defaultValue?: T): Promise<T> {
    await this.initPromise // 等待初始化完成
    const value = this.data[key] as T
    return value !== undefined ? value : (defaultValue as T)
  }

  async set<K extends keyof AppConfig>(key: K, value: AppConfig[K]): Promise<void> {
    await this.initPromise // 等待初始化完成
    this.data[key] = value
    await this.saveData()
  }

  // 获取配置文件路径，用于调试
  getConfigPath(): string {
    return this.configPath
  }
}

// 创建单例
export const store = new SimpleStore()