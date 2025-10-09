import { McpServer } from "../types";

export type McpPreset = {
  id: string;
  name: string;
  description: string;
  tags?: string[];
  server: McpServer;
  homepage?: string;
  docs?: string;
  requiresEnv?: string[];
};

// 预设库（数据文件，当前未接入 UI，便于后续“一键启用”）
export const mcpPresets: McpPreset[] = [
  {
    id: "github-issues",
    name: "GitHub Issues",
    description: "查询与管理 GitHub Issues（示例）",
    tags: ["productivity", "dev"],
    server: { type: "http", url: "https://mcp.example.com/github-issues" },
    docs: "https://example.com/mcp/github-issues",
    requiresEnv: ["GITHUB_TOKEN"],
  },
  {
    id: "local-notes",
    name: "本地笔记",
    description: "访问本地笔记数据库（示例）",
    tags: ["local"],
    server: {
      type: "stdio",
      command: "/usr/local/bin/notes-mcp",
      args: ["--db", "~/.notes/notes.db"],
    },
  },
];

export default mcpPresets;
