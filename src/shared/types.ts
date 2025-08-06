export interface Provider {
  id: string
  name: string
  apiUrl: string
  apiKey: string
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
      getClaudeCodeConfigPath: () => Promise<string>
      selectConfigFile: () => Promise<string | null>
      openExternal: (url: string) => Promise<boolean>
    }
  }
}