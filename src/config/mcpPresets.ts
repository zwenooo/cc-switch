import { McpServer, McpServerSpec } from "../types";

export type McpPreset = Omit<McpServer, "enabled">;

// 预设 MCP（逻辑简化版）：
// - 仅包含最常用、可快速落地的 stdio 模式示例
// - 不涉及分类/模板/测速等复杂逻辑，默认以 disabled 形式"回种"到 config.json
// - 用户可在 MCP 面板中一键启用/编辑
export const mcpPresets: McpPreset[] = [
  {
    id: "fetch",
    name: "mcp-server-fetch",
    description:
      "通用 HTTP 请求工具，支持 GET/POST 等 HTTP 方法，适合快速请求接口/抓取网页数据",
    tags: ["stdio", "http", "web"],
    server: {
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-fetch"],
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
  },
  {
    id: "time",
    name: "@modelcontextprotocol/server-time",
    description:
      "时间查询工具，提供当前时间、时区转换、日期计算等功能，完全无需配置",
    tags: ["stdio", "time", "utility"],
    server: {
      type: "stdio",
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-time"],
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
  },
  {
    id: "memory",
    name: "@modelcontextprotocol/server-memory",
    description:
      "知识图谱记忆系统，支持存储实体、关系和观察，让 AI 记住对话中的重要信息",
    tags: ["stdio", "memory", "graph"],
    server: {
      type: "stdio",
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-memory"],
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
  },
  {
    id: "sequential-thinking",
    name: "@modelcontextprotocol/server-sequential-thinking",
    description: "顺序思考工具，帮助 AI 将复杂问题分解为多个步骤，逐步深入思考",
    tags: ["stdio", "thinking", "reasoning"],
    server: {
      type: "stdio",
      command: "npx",
      args: ["-y", "@modelcontextprotocol/server-sequential-thinking"],
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
  },
  {
    id: "context7",
    name: "@context7/mcp-server",
    description:
      "Context7 文档搜索工具，提供最新的库文档和代码示例，完全无需配置",
    tags: ["stdio", "docs", "search"],
    server: {
      type: "stdio",
      command: "npx",
      args: ["-y", "@context7/mcp-server"],
    } as McpServerSpec,
    homepage: "https://context7.com",
    docs: "https://github.com/context7/mcp-server",
  },
];

export default mcpPresets;
