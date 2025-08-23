export interface Provider {
  id: string;
  name: string;
  settingsConfig: Record<string, any>; // 完整的 Claude Code settings.json 配置
  websiteUrl?: string;
}

export interface AppConfig {
  providers: Record<string, Provider>;
  current: string;
}
