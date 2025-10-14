// 供应商配置处理工具函数

import type { TemplateValueConfig } from "../config/providerPresets";

const isPlainObject = (value: unknown): value is Record<string, any> => {
  return Object.prototype.toString.call(value) === "[object Object]";
};

const deepMerge = (
  target: Record<string, any>,
  source: Record<string, any>,
): Record<string, any> => {
  Object.entries(source).forEach(([key, value]) => {
    if (isPlainObject(value)) {
      if (!isPlainObject(target[key])) {
        target[key] = {};
      }
      deepMerge(target[key], value);
    } else {
      // 直接覆盖非对象字段（数组/基础类型）
      target[key] = value;
    }
  });
  return target;
};

const deepRemove = (
  target: Record<string, any>,
  source: Record<string, any>,
) => {
  Object.entries(source).forEach(([key, value]) => {
    if (!(key in target)) return;

    if (isPlainObject(value) && isPlainObject(target[key])) {
      // 只移除完全匹配的嵌套属性
      deepRemove(target[key], value);
      if (Object.keys(target[key]).length === 0) {
        delete target[key];
      }
    } else if (isSubset(target[key], value)) {
      // 只有当值完全匹配时才删除
      delete target[key];
    }
  });
};

const isSubset = (target: any, source: any): boolean => {
  if (isPlainObject(source)) {
    if (!isPlainObject(target)) return false;
    return Object.entries(source).every(([key, value]) =>
      isSubset(target[key], value),
    );
  }

  if (Array.isArray(source)) {
    if (!Array.isArray(target) || target.length !== source.length) return false;
    return source.every((item, index) => isSubset(target[index], item));
  }

  return target === source;
};

// 深拷贝函数
const deepClone = <T>(obj: T): T => {
  if (obj === null || typeof obj !== "object") return obj;
  if (obj instanceof Date) return new Date(obj.getTime()) as T;
  if (obj instanceof Array) return obj.map((item) => deepClone(item)) as T;
  if (obj instanceof Object) {
    const clonedObj = {} as T;
    for (const key in obj) {
      if (obj.hasOwnProperty(key)) {
        clonedObj[key] = deepClone(obj[key]);
      }
    }
    return clonedObj;
  }
  return obj;
};

export interface UpdateCommonConfigResult {
  updatedConfig: string;
  error?: string;
}

// 验证JSON配置格式
export const validateJsonConfig = (
  value: string,
  fieldName: string = "配置",
): string => {
  if (!value.trim()) {
    return "";
  }
  try {
    const parsed = JSON.parse(value);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return `${fieldName}必须是 JSON 对象`;
    }
    return "";
  } catch {
    return `${fieldName}JSON格式错误，请检查语法`;
  }
};

// 将通用配置片段写入/移除 settingsConfig
export const updateCommonConfigSnippet = (
  jsonString: string,
  snippetString: string,
  enabled: boolean,
): UpdateCommonConfigResult => {
  let config: Record<string, any>;
  try {
    config = jsonString ? JSON.parse(jsonString) : {};
  } catch (err) {
    return {
      updatedConfig: jsonString,
      error: "配置 JSON 解析失败，无法写入通用配置",
    };
  }

  if (!snippetString.trim()) {
    return {
      updatedConfig: JSON.stringify(config, null, 2),
    };
  }

  // 使用统一的验证函数
  const snippetError = validateJsonConfig(snippetString, "通用配置片段");
  if (snippetError) {
    return {
      updatedConfig: JSON.stringify(config, null, 2),
      error: snippetError,
    };
  }

  const snippet = JSON.parse(snippetString) as Record<string, any>;

  if (enabled) {
    const merged = deepMerge(deepClone(config), snippet);
    return {
      updatedConfig: JSON.stringify(merged, null, 2),
    };
  }

  const cloned = deepClone(config);
  deepRemove(cloned, snippet);
  return {
    updatedConfig: JSON.stringify(cloned, null, 2),
  };
};

// 检查当前配置是否已包含通用配置片段
export const hasCommonConfigSnippet = (
  jsonString: string,
  snippetString: string,
): boolean => {
  try {
    if (!snippetString.trim()) return false;
    const config = jsonString ? JSON.parse(jsonString) : {};
    const snippet = JSON.parse(snippetString);
    if (!isPlainObject(snippet)) return false;
    return isSubset(config, snippet);
  } catch (err) {
    return false;
  }
};

// 读取配置中的 API Key（env.ANTHROPIC_AUTH_TOKEN）
export const getApiKeyFromConfig = (jsonString: string): string => {
  try {
    const config = JSON.parse(jsonString);
    const key = config?.env?.ANTHROPIC_AUTH_TOKEN;
    return typeof key === "string" ? key : "";
  } catch (err) {
    return "";
  }
};

// 模板变量替换
export const applyTemplateValues = (
  config: any,
  templateValues: Record<string, TemplateValueConfig> | undefined,
): any => {
  const resolvedValues = Object.fromEntries(
    Object.entries(templateValues ?? {}).map(([key, value]) => {
      const resolvedValue =
        value.editorValue !== undefined
          ? value.editorValue
          : (value.defaultValue ?? "");
      return [key, resolvedValue];
    }),
  );

  const replaceInString = (str: string): string => {
    return Object.entries(resolvedValues).reduce((acc, [key, value]) => {
      const placeholder = `\${${key}}`;
      if (!acc.includes(placeholder)) {
        return acc;
      }
      return acc.split(placeholder).join(value ?? "");
    }, str);
  };

  const traverse = (obj: any): any => {
    if (typeof obj === "string") {
      return replaceInString(obj);
    }
    if (Array.isArray(obj)) {
      return obj.map(traverse);
    }
    if (obj && typeof obj === "object") {
      const result: any = {};
      for (const [key, value] of Object.entries(obj)) {
        result[key] = traverse(value);
      }
      return result;
    }
    return obj;
  };

  return traverse(config);
};

// 判断配置中是否存在 API Key 字段
export const hasApiKeyField = (jsonString: string): boolean => {
  try {
    const config = JSON.parse(jsonString);
    return Object.prototype.hasOwnProperty.call(
      config?.env ?? {},
      "ANTHROPIC_AUTH_TOKEN",
    );
  } catch (err) {
    return false;
  }
};

// 写入/更新配置中的 API Key，默认不新增缺失字段
export const setApiKeyInConfig = (
  jsonString: string,
  apiKey: string,
  options: { createIfMissing?: boolean } = {},
): string => {
  const { createIfMissing = false } = options;
  try {
    const config = JSON.parse(jsonString);
    if (!config.env) {
      if (!createIfMissing) return jsonString;
      config.env = {};
    }
    if (!("ANTHROPIC_AUTH_TOKEN" in config.env) && !createIfMissing) {
      return jsonString;
    }
    config.env.ANTHROPIC_AUTH_TOKEN = apiKey;
    return JSON.stringify(config, null, 2);
  } catch (err) {
    return jsonString;
  }
};

// ========== TOML Config Utilities ==========

export interface UpdateTomlCommonConfigResult {
  updatedConfig: string;
  error?: string;
}

// 保存之前的通用配置片段，用于替换操作
let previousCommonSnippet = "";

// 将通用配置片段写入/移除 TOML 配置
export const updateTomlCommonConfigSnippet = (
  tomlString: string,
  snippetString: string,
  enabled: boolean,
): UpdateTomlCommonConfigResult => {
  if (!snippetString.trim()) {
    // 如果片段为空，直接返回原始配置
    return {
      updatedConfig: tomlString,
    };
  }

  if (enabled) {
    // 添加通用配置
    // 先移除旧的通用配置（如果有）
    let updatedConfig = tomlString;
    if (previousCommonSnippet && tomlString.includes(previousCommonSnippet)) {
      updatedConfig = tomlString.replace(previousCommonSnippet, "");
    }

    // 在文件末尾添加新的通用配置
    // 确保有适当的换行
    const needsNewline = updatedConfig && !updatedConfig.endsWith("\n");
    updatedConfig =
      updatedConfig + (needsNewline ? "\n\n" : "\n") + snippetString;

    // 保存当前通用配置片段
    previousCommonSnippet = snippetString;

    return {
      updatedConfig: updatedConfig.trim() + "\n",
    };
  } else {
    // 移除通用配置
    if (tomlString.includes(snippetString)) {
      const updatedConfig = tomlString.replace(snippetString, "");
      // 清理多余的空行
      const cleaned = updatedConfig.replace(/\n{3,}/g, "\n\n").trim();

      // 清空保存的状态
      previousCommonSnippet = "";

      return {
        updatedConfig: cleaned ? cleaned + "\n" : "",
      };
    }
    return {
      updatedConfig: tomlString,
    };
  }
};

// 检查 TOML 配置是否已包含通用配置片段
export const hasTomlCommonConfigSnippet = (
  tomlString: string,
  snippetString: string,
): boolean => {
  if (!snippetString.trim()) return false;

  // 简单检查配置是否包含片段内容
  // 去除空白字符后比较，避免格式差异影响
  const normalizeWhitespace = (str: string) => str.replace(/\s+/g, " ").trim();

  return normalizeWhitespace(tomlString).includes(
    normalizeWhitespace(snippetString),
  );
};

// ========== Codex base_url utils ==========

// 从 Codex 的 TOML 配置文本中提取 base_url（支持单/双引号）
export const extractCodexBaseUrl = (
  configText: string | undefined | null,
): string | undefined => {
  try {
    const text = typeof configText === "string" ? configText : "";
    if (!text) return undefined;
    const m = text.match(/base_url\s*=\s*(['"])([^'\"]+)\1/);
    return m && m[2] ? m[2] : undefined;
  } catch {
    return undefined;
  }
};

// 从 Provider 对象中提取 Codex base_url（当 settingsConfig.config 为 TOML 字符串时）
export const getCodexBaseUrl = (
  provider: { settingsConfig?: Record<string, any> } | undefined | null,
): string | undefined => {
  try {
    const text =
      typeof provider?.settingsConfig?.config === "string"
        ? (provider as any).settingsConfig.config
        : "";
    return extractCodexBaseUrl(text);
  } catch {
    return undefined;
  }
};

// 在 Codex 的 TOML 配置文本中写入或更新 base_url 字段
export const setCodexBaseUrl = (
  configText: string,
  baseUrl: string,
): string => {
  const trimmed = baseUrl.trim();
  if (!trimmed) {
    return configText;
  }

  const normalizedUrl = trimmed.replace(/\s+/g, "").replace(/\/+$/, "");
  const replacementLine = `base_url = "${normalizedUrl}"`;
  const pattern = /base_url\s*=\s*(["'])([^"']+)\1/;

  if (pattern.test(configText)) {
    return configText.replace(pattern, replacementLine);
  }

  const prefix =
    configText && !configText.endsWith("\n") ? `${configText}\n` : configText;
  return `${prefix}${replacementLine}\n`;
};
