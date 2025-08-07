export interface Provider {
  id: string
  name: string
  settingsConfig: object  // 完整的Claude Code settings.json配置
  websiteUrl?: string
  createdAt?: number
  updatedAt?: number
}

export interface AppConfig {
  providers: Record<string, Provider>
  current: string
}

declare global {
  interface Window {
    electronAPI: {
      getProviders: () => Promise<Record<string, Provider>>
      getCurrentProvider: () => Promise<string>
      addProvider: (provider: Provider) => Promise<boolean>
      deleteProvider: (id: string) => Promise<boolean>
      updateProvider: (provider: Provider) => Promise<boolean>
      switchProvider: (providerId: string) => Promise<boolean>
      importCurrentConfig: (name: string) => Promise<{ success: boolean; providerId?: string }>
      getClaudeCodeConfigPath: () => Promise<string>
      selectConfigFile: () => Promise<string | null>
      openExternal: (url: string) => Promise<boolean>
    }
  }
}