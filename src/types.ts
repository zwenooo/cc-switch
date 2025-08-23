export interface Provider {
  id: string
  name: string
  settingsConfig: Record<string, any>  // 完整的Claude Code settings.json配置
  websiteUrl?: string
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
      importCurrentConfigAsDefault: () => Promise<{ success: boolean; providerId?: string }>
      getClaudeCodeConfigPath: () => Promise<string>
      selectConfigFile: () => Promise<string | null>
      openConfigFolder: () => Promise<boolean>
      openExternal: (url: string) => Promise<boolean>
    }
  }
}