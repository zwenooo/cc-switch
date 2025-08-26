/**
 * 预设供应商配置模板
 */
export interface ProviderPreset {
  name: string;
  websiteUrl: string;
  settingsConfig: object;
}

export const providerPresets: ProviderPreset[] = [
  {
    name: "DeepSeek v3.1",
    websiteUrl: "https://platform.deepseek.com",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.deepseek.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "sk-your-api-key-here",
        ANTHROPIC_MODEL: "deepseek-chat",
        ANTHROPIC_SMALL_FAST_MODEL: "deepseek-chat",
      },
    },
  },
  {
    name: "智谱GLM",
    websiteUrl: "https://open.bigmodel.cn",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://open.bigmodel.cn/api/anthropic",
        ANTHROPIC_AUTH_TOKEN: "sk-your-api-key-here",
      },
    },
  },
  {
    name: "千问Qwen-Coder",
    websiteUrl: "https://bailian.console.aliyun.com",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL:
          "https://dashscope.aliyuncs.com/api/v2/apps/claude-code-proxy",
        ANTHROPIC_AUTH_TOKEN: "sk-your-api-key-here",
      },
    },
  },
  {
    name: "Kimi k2",
    websiteUrl: "https://platform.moonshot.cn/console",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.moonshot.cn/anthropic",
        ANTHROPIC_AUTH_TOKEN: "sk-your-api-key-here",
        ANTHROPIC_MODEL: "kimi-k2-turbo-preview",
        ANTHROPIC_SMALL_FAST_MODEL: "kimi-k2-turbo-preview",
      },
    },
  },
  {
    name: "PackyCode",
    websiteUrl: "https://www.packycode.com",
    settingsConfig: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.packycode.com",
        ANTHROPIC_AUTH_TOKEN: "sk-your-api-key-here",
      },
    },
  },
  
];
