/// <reference types="vite/client" />

import { Provider } from './shared/types';

interface ImportResult {
  success: boolean;
  message?: string;
}

interface ConfigStatus {
  exists: boolean;
  path: string;
  error?: string;
}

declare global {
  interface Window {
    electronAPI: {
      getProviders: () => Promise<Record<string, Provider>>;
      getCurrentProvider: () => Promise<string>;
      addProvider: (provider: Provider) => Promise<boolean>;
      deleteProvider: (id: string) => Promise<boolean>;
      updateProvider: (provider: Provider) => Promise<boolean>;
      switchProvider: (providerId: string) => Promise<boolean>;
      importCurrentConfigAsDefault: () => Promise<ImportResult>;
      getClaudeCodeConfigPath: () => Promise<string>;
      getClaudeConfigStatus: () => Promise<ConfigStatus>;
      selectConfigFile: () => Promise<string | null>;
      openConfigFolder: () => Promise<void>;
      openExternal: (url: string) => Promise<void>;
    };
    platform: {
      isMac: boolean;
    };
    __TAURI__?: any;
  }
}

export {};