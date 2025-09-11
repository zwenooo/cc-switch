export interface Provider {
  id: string;
  name: string;
  settingsConfig: Record<string, any>; // 应用配置对象：Claude 为 settings.json；Codex 为 { auth, config }
  websiteUrl?: string;
  createdAt?: number; // 添加时间戳（毫秒）
}

export interface AppConfig {
  providers: Record<string, Provider>;
  current: string;
}

// 应用设置类型（用于 SettingsModal 与 Tauri API）
export interface Settings {
  // 是否在系统托盘（macOS 菜单栏）显示图标
  showInTray: boolean;
}
