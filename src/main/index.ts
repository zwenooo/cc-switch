import { app, BrowserWindow, ipcMain, dialog, shell } from "electron";
import path from "path";
import fs from "fs/promises";
import { Provider } from "../shared/types";
import {
  switchProvider,
  getClaudeCodeConfig,
  saveProviderConfig,
  deleteProviderConfig,
  sanitizeProviderName,
  importCurrentConfigAsDefault,
  getProviderConfigPath,
  fileExists,
} from "./services";
import { store } from "./store";

let mainWindow: BrowserWindow | null = null;

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    webPreferences: {
      preload: path.join(__dirname, "../main/preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
    },
    titleBarStyle: "hiddenInset",
    autoHideMenuBar: true,
  });

  if (app.isPackaged) {
    mainWindow.loadFile(path.join(__dirname, "../renderer/index.html"));
  } else {
    mainWindow.loadURL("http://localhost:3000");
    mainWindow.webContents.openDevTools();
  }

  mainWindow.on("closed", () => {
    mainWindow = null;
  });
}

app.whenReady().then(() => {
  createWindow();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

// IPC handlers
ipcMain.handle("getProviders", () => {
  return store.get("providers", {} as Record<string, Provider>);
});

ipcMain.handle("getCurrentProvider", () => {
  return store.get("current", "");
});

ipcMain.handle("addProvider", async (_, provider: Provider) => {
  try {
    // 1. 保存供应商配置到独立文件
    const saveSuccess = await saveProviderConfig(provider);
    if (!saveSuccess) {
      return false;
    }

    // 2. 更新应用配置
    const providers = store.get("providers", {} as Record<string, Provider>);
    providers[provider.id] = {
      ...provider,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };
    await store.set("providers", providers);

    return true;
  } catch (error) {
    console.error("添加供应商失败:", error);
    return false;
  }
});

ipcMain.handle("deleteProvider", async (_, id: string) => {
  try {
    const providers = store.get("providers", {} as Record<string, Provider>);
    const provider = providers[id];

    // 1. 删除供应商配置文件
    const deleteSuccess = await deleteProviderConfig(id, provider?.name);
    if (!deleteSuccess) {
      console.error("删除供应商配置文件失败");
      // 仍然继续删除应用配置，避免配置不同步
    }

    // 2. 更新应用配置
    delete providers[id];
    await store.set("providers", providers);

    // 3. 如果删除的是当前供应商，清空当前选择
    const currentProviderId = store.get("current", "");
    if (currentProviderId === id) {
      await store.set("current", "");
    }

    return true;
  } catch (error) {
    console.error("删除供应商失败:", error);
    return false;
  }
});

ipcMain.handle("updateProvider", async (_, provider: Provider) => {
  try {
    const providers = store.get("providers", {} as Record<string, Provider>);
    const currentProviderId = store.get("current", "");
    const oldProvider = providers[provider.id];

    // 1. 如果名字发生变化，需要重命名配置文件
    if (oldProvider && oldProvider.name !== provider.name) {
      const oldConfigPath = getProviderConfigPath(
        provider.id,
        oldProvider.name
      );
      const newConfigPath = getProviderConfigPath(provider.id, provider.name);

      // 如果旧配置文件存在且路径不同，需要重命名
      if (
        (await fileExists(oldConfigPath)) &&
        oldConfigPath !== newConfigPath
      ) {
        // 如果新路径已存在文件，先删除避免冲突
        if (await fileExists(newConfigPath)) {
          await fs.unlink(newConfigPath);
        }
        await fs.rename(oldConfigPath, newConfigPath);
        console.log(
          `已重命名配置文件: ${oldProvider.name} -> ${provider.name}`
        );
      }
    }

    // 2. 保存更新后的配置到文件
    const saveSuccess = await saveProviderConfig({
      ...provider,
      updatedAt: Date.now(),
    });
    if (!saveSuccess) {
      return false;
    }

    // 3. 更新应用配置
    providers[provider.id] = {
      ...provider,
      updatedAt: Date.now(),
    };
    await store.set("providers", providers);

    // 4. 如果编辑的是当前激活的供应商，需要重新切换以应用更改
    if (provider.id === currentProviderId) {
      const switchSuccess = await switchProvider(
        provider,
        currentProviderId,
        providers
      );
      if (!switchSuccess) {
        console.error("更新当前供应商的Claude Code配置失败");
        return false;
      }
    }

    return true;
  } catch (error) {
    console.error("更新供应商失败:", error);
    return false;
  }
});

ipcMain.handle("switchProvider", async (_, providerId: string) => {
  try {
    const providers = store.get("providers", {} as Record<string, Provider>);
    const provider = providers[providerId];
    const currentProviderId = store.get("current", "");

    if (!provider) {
      console.error(`供应商不存在: ${providerId}`);
      return false;
    }

    // 执行切换
    const success = await switchProvider(
      provider,
      currentProviderId,
      providers
    );
    if (success) {
      await store.set("current", providerId);
      console.log(`成功切换到供应商: ${provider.name}`);
    }

    return success;
  } catch (error) {
    console.error("切换供应商失败:", error);
    return false;
  }
});

ipcMain.handle("importCurrentConfigAsDefault", async () => {
  try {
    const result = await importCurrentConfigAsDefault();

    if (result.success && result.provider) {
      // 将默认供应商添加到store中
      const providers = store.get("providers", {} as Record<string, Provider>);
      providers[result.provider.id] = result.provider;
      await store.set("providers", providers);
      
      // 设置为当前选中的供应商
      await store.set("current", result.provider.id);

      return { success: true, providerId: result.provider.id };
    }

    return result;
  } catch (error: any) {
    console.error("导入默认配置失败:", error);
    return { success: false };
  }
});

ipcMain.handle("getClaudeCodeConfigPath", () => {
  return getClaudeCodeConfig().path;
});

ipcMain.handle("selectConfigFile", async () => {
  if (!mainWindow) return null;

  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ["openFile"],
    title: "选择 Claude Code 配置文件",
    filters: [
      { name: "JSON 文件", extensions: ["json"] },
      { name: "所有文件", extensions: ["*"] },
    ],
    defaultPath: "settings.json",
  });

  if (result.canceled || result.filePaths.length === 0) {
    return null;
  }

  return result.filePaths[0];
});

ipcMain.handle("openExternal", async (_, url: string) => {
  try {
    await shell.openExternal(url);
    return true;
  } catch (error) {
    console.error("打开外部链接失败:", error);
    return false;
  }
});
