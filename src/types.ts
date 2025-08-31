export interface Provider {
  id: string;
  name: string;
  settingsConfig: Record<string, any>; // 应用配置对象：Claude 为 settings.json；Codex 为 { auth, config }
  websiteUrl?: string;
}

export interface AppConfig {
  providers: Record<string, Provider>;
  current: string;
}
