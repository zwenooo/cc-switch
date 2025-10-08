import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { X, Plus, Save, Trash2, Wrench } from "lucide-react";
import { McpServer, McpStatus } from "../../types";

interface McpPanelProps {
  onClose: () => void;
  onNotify?: (message: string, type: "success" | "error", duration?: number) => void;
}

const emptyServer: McpServer & { id?: string } = {
  type: "stdio",
  command: "",
  args: [],
  env: {},
};

const parseEnvText = (text: string): Record<string, string> => {
  const lines = text
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
  const env: Record<string, string> = {};
  for (const l of lines) {
    const idx = l.indexOf("=");
    if (idx > 0) {
      const k = l.slice(0, idx).trim();
      const v = l.slice(idx + 1).trim();
      if (k) env[k] = v;
    }
  }
  return env;
};

const formatEnvText = (env?: Record<string, string>): string => {
  if (!env) return "";
  return Object.entries(env)
    .map(([k, v]) => `${k}=${v}`)
    .join("\n");
};

const McpPanel: React.FC<McpPanelProps> = ({ onClose, onNotify }) => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<McpStatus | null>(null);
  const [servers, setServers] = useState<Record<string, McpServer>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<McpServer & { id?: string }>(emptyServer);
  const [formEnvText, setFormEnvText] = useState<string>("");
  const [formArgsText, setFormArgsText] = useState<string>("");

  const reload = async () => {
    setLoading(true);
    try {
      const s = await window.api.getClaudeMcpStatus();
      setStatus(s);
      const text = await window.api.readClaudeMcpConfig();
      if (text) {
        try {
          const obj = JSON.parse(text);
          const list = (obj?.mcpServers || {}) as Record<string, McpServer>;
          setServers(list);
        } catch (e) {
          console.error("Failed to parse mcp.json", e);
          setServers({});
        }
      } else {
        setServers({});
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    reload();
  }, []);

  const handleToggleEnable = async (enable: boolean) => {
    try {
      const changed = await window.api.setClaudeMcpEnableAllProjects(enable);
      if (changed) {
        await reload();
        onNotify?.(t("mcp.notice.restartClaude"), "success", 2000);
      }
    } catch (e: any) {
      onNotify?.(e?.message || t("mcp.error.toggleFailed"), "error", 5000);
    }
  };

  const resetForm = () => {
    setEditingId(null);
    setForm(emptyServer);
    setFormArgsText("");
    setFormEnvText("");
  };

  const beginEdit = (id?: string) => {
    if (!id) {
      resetForm();
      return;
    }
    const spec = servers[id];
    setEditingId(id);
    setForm({ id, ...spec });
    setFormArgsText((spec.args || []).join(" "));
    setFormEnvText(formatEnvText(spec.env));
  };

  const submitForm = async () => {
    if (!form.id || !form.id.trim()) {
      onNotify?.(t("mcp.error.idRequired"), "error", 3000);
      return;
    }
    if (!form.command || !form.command.trim()) {
      onNotify?.(t("mcp.error.commandRequired"), "error", 3000);
      return;
    }
    setSaving(true);
    try {
      const spec: McpServer = {
        type: form.type,
        command: form.command.trim(),
        args: formArgsText
          .split(/\s+/)
          .map((s) => s.trim())
          .filter((s) => s.length > 0),
        env: parseEnvText(formEnvText),
        ...(form.cwd ? { cwd: form.cwd } : {}),
      };
      await window.api.upsertClaudeMcpServer(form.id.trim(), spec);
      await reload();
      resetForm();
      onNotify?.(t("mcp.msg.saved"), "success", 1500);
    } catch (e: any) {
      onNotify?.(e?.message || t("mcp.error.saveFailed"), "error", 6000);
    } finally {
      setSaving(false);
    }
  };

  const removeServer = async (id: string) => {
    try {
      await window.api.deleteClaudeMcpServer(id);
      await reload();
      if (editingId === id) resetForm();
      onNotify?.(t("mcp.msg.deleted"), "success", 1500);
    } catch (e: any) {
      onNotify?.(e?.message || t("mcp.error.deleteFailed"), "error", 5000);
    }
  };

  const addTemplateFetch = async () => {
    try {
      await window.api.upsertClaudeMcpServer("mcp-fetch", {
        type: "stdio",
        command: "uvx",
        args: ["mcp-server-fetch"],
      });
      await reload();
      onNotify?.(t("mcp.msg.templateAdded"), "success", 1500);
    } catch (e: any) {
      onNotify?.(e?.message || t("mcp.error.saveFailed"), "error", 5000);
    }
  };

  const validateCommand = async () => {
    if (!form.command) return;
    const ok = await window.api.validateMcpCommand(form.command.trim());
    onNotify?.(
      ok ? t("mcp.validation.ok") : t("mcp.validation.fail"),
      ok ? "success" : "error",
      1500,
    );
  };

  const serverEntries = useMemo(() => Object.entries(servers), [servers]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />

      <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-lg max-w-4xl w-full mx-4 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-800">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            {t("mcp.title")}
          </h3>
          <button
            onClick={onClose}
            className="p-1 text-gray-500 hover:text-gray-900 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-gray-100 dark:hover:bg-gray-800 rounded-md transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Left: status & list */}
          <div>
            <div className="flex items-center justify-between mb-4">
              <div>
                <div className="text-sm text-gray-500 dark:text-gray-400">
                  {t("mcp.enableProject")}
                </div>
                <div className="text-xs text-gray-400 dark:text-gray-500">
                  {status?.settingsLocalPath}
                </div>
              </div>
              <label className="inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  className="sr-only peer"
                  checked={!!status?.enableAllProjectMcpServers}
                  onChange={(e) => handleToggleEnable(e.target.checked)}
                />
                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-emerald-500 relative" />
              </label>
            </div>

            <div className="flex items-center gap-3 mb-3">
              <button
                onClick={() => beginEdit(undefined)}
                className="inline-flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-md bg-blue-500 text-white hover:bg-blue-600"
              >
                <Plus size={16} /> {t("mcp.add")}
              </button>
              <button
                onClick={addTemplateFetch}
                className="inline-flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-md bg-emerald-500 text-white hover:bg-emerald-600"
              >
                <Wrench size={16} /> {t("mcp.template.fetch")}
              </button>
              <button
                onClick={() => window.api.openConfigFolder("claude")}
                className="inline-flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-md bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-200 dark:hover:bg-gray-700"
              >
                {t("mcp.openFolder")}
              </button>
            </div>

            <div className="border border-gray-200 dark:border-gray-800 rounded-lg overflow-hidden">
              <div className="px-3 py-2 text-xs text-gray-500 dark:text-gray-400 border-b border-gray-200 dark:border-gray-800 flex items-center justify-between">
                <span>
                  {t("mcp.serverList")} ({status?.serverCount || 0})
                </span>
                {status?.mcpJsonExists ? (
                  <span className="text-gray-400">{status?.mcpJsonPath}</span>
                ) : (
                  <span className="text-gray-400">mcp.json</span>
                )}
              </div>
              <div className="max-h-64 overflow-auto divide-y divide-gray-200 dark:divide-gray-800">
                {loading && (
                  <div className="p-4 text-sm text-gray-500">{t("mcp.loading")}</div>
                )}
                {!loading && serverEntries.length === 0 && (
                  <div className="p-4 text-sm text-gray-500">{t("mcp.empty")}</div>
                )}
                {!loading &&
                  serverEntries.map(([id, spec]) => (
                    <div key={id} className="p-3 flex items-center justify-between">
                      <div>
                        <div className="text-sm font-medium text-gray-900 dark:text-gray-100">
                          {id}
                        </div>
                        <div className="text-xs text-gray-500 dark:text-gray-400">
                          {spec.type} · {spec.command} {spec.args?.join(" ")}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => beginEdit(id)}
                          className="px-2 py-1 text-xs rounded-md bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-200 dark:hover:bg-gray-700"
                        >
                          {t("common.edit")}
                        </button>
                        <button
                          onClick={() => removeServer(id)}
                          className="px-2 py-1 text-xs rounded-md bg-red-500 text-white hover:bg-red-600 flex items-center gap-1"
                        >
                          <Trash2 size={14} /> {t("common.delete")}
                        </button>
                      </div>
                    </div>
                  ))}
              </div>
            </div>
          </div>

          {/* Right: form */}
          <div>
            <div className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
              {editingId ? t("mcp.editServer") : t("mcp.addServer")}
            </div>

            <div className="space-y-3">
              <div>
                <label className="block text-xs text-gray-500 mb-1">
                  {t("mcp.id")}
                </label>
                <input
                  className="w-full px-3 py-2 rounded-md bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="my-mcp"
                  value={form.id || ""}
                  onChange={(e) => setForm((s) => ({ ...s, id: e.target.value }))}
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs text-gray-500 mb-1">
                    {t("mcp.type")}
                  </label>
                  <select
                    className="w-full px-3 py-2 rounded-md bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800"
                    value={form.type}
                    onChange={(e) =>
                      setForm((s) => ({ ...s, type: e.target.value as any }))
                    }
                  >
                    <option value="stdio">stdio</option>
                    <option value="sse">sse</option>
                  </select>
                </div>
                <div>
                  <label className="block text-xs text-gray-500 mb-1">
                    {t("mcp.cwd")}
                  </label>
                  <input
                    className="w-full px-3 py-2 rounded-md bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800"
                    placeholder="/path/to/project"
                    value={form.cwd || ""}
                    onChange={(e) => setForm((s) => ({ ...s, cwd: e.target.value }))}
                  />
                </div>
              </div>

              <div>
                <label className="block text-xs text-gray-500 mb-1">
                  {t("mcp.command")}
                </label>
                <div className="flex gap-2">
                  <input
                    className="flex-1 px-3 py-2 rounded-md bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800"
                    placeholder="uvx"
                    value={form.command}
                    onChange={(e) => setForm((s) => ({ ...s, command: e.target.value }))}
                  />
                  <button
                    type="button"
                    onClick={validateCommand}
                    className="px-3 py-2 rounded-md bg-emerald-500 text-white hover:bg-emerald-600 text-sm inline-flex items-center gap-1"
                  >
                    <Wrench size={16} /> {t("mcp.validateCommand")}
                  </button>
                </div>
              </div>

              <div>
                <label className="block text-xs text-gray-500 mb-1">{t("mcp.args")}</label>
                <input
                  className="w-full px-3 py-2 rounded-md bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800"
                  placeholder={t("mcp.argsPlaceholder")}
                  value={formArgsText}
                  onChange={(e) => setFormArgsText(e.target.value)}
                />
              </div>

              <div>
                <label className="block text-xs text-gray-500 mb-1">{t("mcp.env")}</label>
                <textarea
                  className="w-full px-3 py-2 rounded-md bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 h-24"
                  placeholder={t("mcp.envPlaceholder")}
                  value={formEnvText}
                  onChange={(e) => setFormEnvText(e.target.value)}
                />
              </div>

              <div className="flex items-center gap-3">
                <button
                  onClick={submitForm}
                  disabled={saving}
                  className="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-md bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-60"
                >
                  <Save size={16} /> {editingId ? t("common.save") : t("common.add")}
                </button>
                <button
                  onClick={resetForm}
                  className="px-4 py-2 text-sm rounded-md bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-200 dark:hover:bg-gray-700"
                >
                  {t("mcp.reset")}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default McpPanel;

