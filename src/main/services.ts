import fs from "fs/promises";
import path from "path";
import os from "os";
import { Provider } from "../shared/types";

/**
 * 清理供应商名称，确保文件名安全
 */
export function sanitizeProviderName(name: string): string {
  return name.replace(/[<>:"/\\|?*]/g, "-").toLowerCase();
}

export function getClaudeCodeConfig() {
  // Claude Code 配置文件路径
  const configDir = path.join(os.homedir(), ".claude");
  const configPath = path.join(configDir, "settings.json");

  return { path: configPath, dir: configDir };
}

/**
 * 获取供应商配置文件路径（基于供应商名称）
 */
export function getProviderConfigPath(
  providerId: string,
  providerName?: string
): string {
  const { dir } = getClaudeCodeConfig();

  // 如果提供了名称，使用名称；否则使用ID（向后兼容）
  const baseName = providerName
    ? sanitizeProviderName(providerName)
    : sanitizeProviderName(providerId);
  return path.join(dir, `settings-${baseName}.json`);
}

/**
 * 保存供应商配置到独立文件
 */
export async function saveProviderConfig(provider: Provider): Promise<boolean> {
  try {
    const { dir } = getClaudeCodeConfig();
    const providerConfigPath = getProviderConfigPath(
      provider.id,
      provider.name
    );

    // 确保目录存在
    await fs.mkdir(dir, { recursive: true });

    // 保存配置到供应商专用文件
    await fs.writeFile(
      providerConfigPath,
      JSON.stringify(provider.settingsConfig, null, 2),
      "utf-8"
    );

    return true;
  } catch (error) {
    console.error("保存供应商配置失败:", error);
    return false;
  }
}

/**
 * 检查文件是否存在
 */
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

/**
 * 切换供应商配置（基于文件重命名）
 */
export async function switchProvider(
  provider: Provider,
  currentProviderId?: string,
  providers?: Record<string, Provider>
): Promise<boolean> {
  try {
    const { path: settingsPath, dir: configDir } = getClaudeCodeConfig();
    const newSettingsPath = getProviderConfigPath(provider.id, provider.name);

    // 确保目录存在
    await fs.mkdir(configDir, { recursive: true });

    // 检查目标配置文件是否存在
    if (!(await fileExists(newSettingsPath))) {
      console.error(`供应商配置文件不存在: ${newSettingsPath}`);
      return false;
    }

    // 1. 如果当前存在settings.json，先备份到当前供应商的配置文件
    if (await fileExists(settingsPath)) {
      if (currentProviderId && providers && providers[currentProviderId]) {
        const currentProvider = providers[currentProviderId];
        const currentProviderPath = getProviderConfigPath(
          currentProviderId,
          currentProvider.name
        );
        await fs.rename(settingsPath, currentProviderPath);
      } else {
        // 如果没有当前供应商ID，创建临时备份
        const backupPath = path.join(
          configDir,
          `settings-backup-${Date.now()}.json`
        );
        await fs.rename(settingsPath, backupPath);
        console.log(`已备份当前配置到: ${backupPath}`);
      }
    }

    // 2. 将目标供应商配置重命名为settings.json
    await fs.rename(newSettingsPath, settingsPath);

    console.log(`成功切换到供应商: ${provider.name}`);
    return true;
  } catch (error) {
    console.error("切换供应商失败:", error);
    return false;
  }
}

/**
 * 导入当前 settings.json 配置为一个供应商
 */
export async function importCurrentConfig(
  name: string
): Promise<{ success: boolean; provider?: Provider }> {
  try {
    const { path: settingsPath } = getClaudeCodeConfig();

    // 检查当前配置是否存在
    if (!(await fileExists(settingsPath))) {
      return { success: false };
    }

    // 读取当前配置
    const configContent = await fs.readFile(settingsPath, "utf-8");
    const settingsConfig = JSON.parse(configContent);

    // 生成唯一的供应商ID
    let providerId = name.toLowerCase().replace(/[^a-z0-9]/g, "-");
    let counter = 1;

    // 检查ID是否已存在，如果存在则添加数字后缀
    while (await fileExists(getProviderConfigPath(providerId, name))) {
      providerId = `${name
        .toLowerCase()
        .replace(/[^a-z0-9]/g, "-")}-${counter}`;
      counter++;
    }

    // 创建供应商对象
    const provider: Provider = {
      id: providerId,
      name: name,
      settingsConfig: settingsConfig,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };

    // 保存为供应商配置
    const success = await saveProviderConfig(provider);

    if (success) {
      console.log(`已导入当前配置为供应商: ${name} (${providerId})`);
      return { success: true, provider };
    } else {
      return { success: false };
    }
  } catch (error: any) {
    console.error("导入当前配置失败:", error);
    return { success: false };
  }
}

/**
 * 删除供应商配置文件
 */
export async function deleteProviderConfig(
  providerId: string,
  providerName?: string
): Promise<boolean> {
  try {
    const providerConfigPath = getProviderConfigPath(providerId, providerName);

    if (await fileExists(providerConfigPath)) {
      await fs.unlink(providerConfigPath);
      console.log(`已删除供应商配置文件: ${providerConfigPath}`);
    }

    return true;
  } catch (error) {
    console.error("删除供应商配置失败:", error);
    return false;
  }
}
