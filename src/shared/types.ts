export interface Provider {
  id: string
  name: string
  apiUrl: string
  apiKey: string
  model?: string
}

export interface ProviderStatus {
  isOnline: boolean
  responseTime: number
  lastChecked: Date
  error?: string
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
      checkStatus: (provider: Provider) => Promise<ProviderStatus>
      switchProvider: (providerId: string) => Promise<boolean>
      getClaudeCodeConfigPath: () => Promise<string>
      selectConfigFile: () => Promise<string | null>
    }
  }
}