/**
 * Codex 预设供应商配置模板
 */
export interface CodexProviderPreset {
  name: string;
  websiteUrl: string;
  auth: Record<string, any>; // 将写入 ~/.codex/auth.json
  config: string; // 将写入 ~/.codex/config.toml（TOML 字符串）
  isOfficial?: boolean; // 标识是否为官方预设
}

export const codexProviderPresets: CodexProviderPreset[] = [
  {
    name: "Codex官方",
    websiteUrl: "https://codex",
    isOfficial: true,
    // 官方一般不需要在 auth.json 里预置 key，由用户根据实际环境填写
    auth: {},
    config: `# Codex 默认配置模板\n# 根据你的 Codex 安装或文档进行调整\nmodel = "default"\ntemperature = 0.7`,
  },
  {
    name: "PackyCode",
    websiteUrl: "https://www.packycode.com",
    // PackyCode 一般通过 API Key；请将占位符替换为你的实际 key
    auth: {
      api_key: "sk-your-api-key-here",
    },
    config: `# Codex 配置模板 - PackyCode\n# 如有需要可添加 base_url: \n# base_url = "https://api.packycode.com"\nmodel = "default"\ntemperature = 0.7`,
  },
];

