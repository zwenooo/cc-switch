import { useState, useEffect } from "react";
import { X, Info, RefreshCw, FolderOpen } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import "../lib/tauri-api";
import type { Settings } from "../types";

interface SettingsModalProps {
  onClose: () => void;
}

export default function SettingsModal({ onClose }: SettingsModalProps) {
  const [settings, setSettings] = useState<Settings>({
    showInDock: true,
  });
  const [configPath, setConfigPath] = useState<string>("");
  const [version, setVersion] = useState<string>("");
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);

  useEffect(() => {
    loadSettings();
    loadConfigPath();
    loadVersion();
  }, []);

  const loadVersion = async () => {
    try {
      const appVersion = await getVersion();
      setVersion(appVersion);
    } catch (error) {
      console.error("获取版本信息失败:", error);
      setVersion("3.1.1"); // 降级使用默认版本
    }
  };

  const loadSettings = async () => {
    try {
      const loadedSettings = await window.api.getSettings();
      if (loadedSettings?.showInDock !== undefined) {
        setSettings({ showInDock: loadedSettings.showInDock });
      }
    } catch (error) {
      console.error("加载设置失败:", error);
    }
  };

  const loadConfigPath = async () => {
    try {
      const path = await window.api.getAppConfigPath();
      if (path) {
        setConfigPath(path);
      }
    } catch (error) {
      console.error("获取配置路径失败:", error);
    }
  };

  const saveSettings = async () => {
    try {
      await window.api.saveSettings(settings);
      onClose();
    } catch (error) {
      console.error("保存设置失败:", error);
    }
  };

  const handleCheckUpdate = async () => {
    setIsCheckingUpdate(true);
    try {
      await window.api.checkForUpdates();
    } catch (error) {
      console.error("检查更新失败:", error);
    } finally {
      setTimeout(() => setIsCheckingUpdate(false), 2000);
    }
  };

  const handleOpenConfigFolder = async () => {
    try {
      await window.api.openAppConfigFolder();
    } catch (error) {
      console.error("打开配置文件夹失败:", error);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 dark:bg-black/70 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-[500px] overflow-hidden">
        {/* 标题栏 */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-800">
          <h2 className="text-lg font-semibold text-blue-500 dark:text-blue-400">
            设置
          </h2>
          <button
            onClick={onClose}
            className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-md transition-colors"
          >
            <X size={20} className="text-gray-500 dark:text-gray-400" />
          </button>
        </div>

        {/* 设置内容 */}
        <div className="px-6 py-4 space-y-6">
          {/* 显示设置 - 功能还未实现 */}
          {/* <div>
            <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
              显示设置
            </h3>
            <label className="flex items-center justify-between">
              <span className="text-sm text-gray-500">
                在 Dock 中显示（macOS）
              </span>
              <input
                type="checkbox"
                checked={settings.showInDock}
                onChange={(e) =>
                  setSettings({ ...settings, showInDock: e.target.checked })
                }
                className="w-4 h-4 text-blue-500 rounded focus:ring-blue-500/20"
              />
            </label>
          </div> */}

          {/* 配置文件位置 */}
          <div>
            <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
              配置文件位置
            </h3>
            <div className="flex items-center gap-2">
              <div className="flex-1 px-3 py-2 bg-gray-100 dark:bg-gray-800 rounded-lg">
                <span className="text-xs font-mono text-gray-500 dark:text-gray-400">
                  {configPath || "加载中..."}
                </span>
              </div>
              <button
                onClick={handleOpenConfigFolder}
                className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                title="打开文件夹"
              >
                <FolderOpen
                  size={18}
                  className="text-gray-500 dark:text-gray-400"
                />
              </button>
            </div>
          </div>

          {/* 关于 */}
          <div>
            <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
              关于
            </h3>
            <div className="p-4 bg-gray-100 dark:bg-gray-800 rounded-lg">
              <div className="flex items-start justify-between">
                <div className="flex items-start gap-3">
                  <Info
                    size={18}
                    className="text-gray-500 mt-0.5"
                  />
                  <div className="text-sm">
                    <p className="font-medium text-gray-900 dark:text-gray-100">
                      CC Switch
                    </p>
                    <p className="mt-1 text-gray-500 dark:text-gray-400">
                      版本 {version}
                    </p>
                  </div>
                </div>
                <button
                  onClick={handleCheckUpdate}
                  disabled={isCheckingUpdate}
                  className={`px-3 py-1.5 text-xs font-medium rounded-lg transition-all ${
                    isCheckingUpdate
                      ? "bg-white dark:bg-gray-700 text-gray-400 dark:text-gray-500"
                      : "bg-white dark:bg-gray-700 hover:bg-gray-50 dark:hover:bg-gray-600 text-blue-500 dark:text-blue-400"
                  }`}
                >
                  {isCheckingUpdate ? (
                    <span className="flex items-center gap-1">
                      <RefreshCw size={12} className="animate-spin" />
                      检查中...
                    </span>
                  ) : (
                    "检查更新"
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* 底部按钮 */}
        <div className="flex justify-end gap-3 px-6 py-4 border-t border-gray-200 dark:border-gray-800">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
          >
            取消
          </button>
          <button
            onClick={saveSettings}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700 rounded-lg transition-colors"
          >
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
