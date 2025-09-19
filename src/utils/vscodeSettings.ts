import { applyEdits, modify, parse } from "jsonc-parser";

const fmt = { insertSpaces: true, tabSize: 2, eol: "\n" } as const;

export interface AppliedCheck {
  hasApiBase: boolean;
  apiBase?: string;
  hasPreferredAuthMethod: boolean;
}

export function normalizeBaseUrl(url: string): string {
  return url.replace(/\/+$/, "");
}

const isDocEmpty = (s: string) => s.trim().length === 0;

// 检查 settings.json（JSONC 文本）中是否已经应用了我们的键
export function detectApplied(content: string): AppliedCheck {
  try {
    // 允许 JSONC 的宽松解析：jsonc-parser 的 parse 可以直接处理注释
    const data = parse(content) as any;
    const apiBase = data?.["chatgpt.apiBase"];
    const method = data?.["chatgpt.config"]?.preferred_auth_method;
    return {
      hasApiBase: typeof apiBase === "string",
      apiBase,
      hasPreferredAuthMethod: typeof method === "string",
    };
  } catch {
    return { hasApiBase: false, hasPreferredAuthMethod: false };
  }
}

// 生成“清理我们管理的键”后的文本（仅删除我们写入的两个键）
export function removeManagedKeys(content: string): string {
  if (isDocEmpty(content)) return content; // 空文档无需删除
  let out = content;
  // 删除 chatgpt.apiBase
  try {
    out = applyEdits(
      out,
      modify(out, ["chatgpt.apiBase"], undefined, { formattingOptions: fmt }),
    );
  } catch {
    // 忽略删除失败
  }
  // 删除 chatgpt.config.preferred_auth_method（注意 chatgpt.config 是顶层带点的键）
  try {
    out = applyEdits(
      out,
      modify(out, ["chatgpt.config", "preferred_auth_method"], undefined, {
        formattingOptions: fmt,
      }),
    );
  } catch {
    // 忽略删除失败
  }

  // 兼容早期错误写入：若曾写成嵌套 chatgpt.config.preferred_auth_method，也一并清理
  try {
    out = applyEdits(
      out,
      modify(out, ["chatgpt", "config", "preferred_auth_method"], undefined, {
        formattingOptions: fmt,
      }),
    );
  } catch {
    // 忽略删除失败
  }

  // 若 chatgpt.config 变为空对象，顺便移除（不影响其他 chatgpt* 键）
  try {
    const data = parse(out) as any;
    const cfg = data?.["chatgpt.config"];
    if (
      cfg &&
      typeof cfg === "object" &&
      !Array.isArray(cfg) &&
      Object.keys(cfg).length === 0
    ) {
      out = applyEdits(
        out,
        modify(out, ["chatgpt.config"], undefined, { formattingOptions: fmt }),
      );
    }
  } catch {
    // 忽略解析失败，保持已删除的键
  }

  return out;
}

// 生成“应用供应商到 VS Code”后的文本：
// - 先清理我们管理的键
// - 再根据是否官方决定写入（官方：不写入；非官方：写入两个键）
export function applyProviderToVSCode(
  content: string,
  opts: { baseUrl?: string | null; isOfficial?: boolean },
): string {
  let out = removeManagedKeys(content);
  if (!opts.isOfficial && opts.baseUrl) {
    const apiBase = normalizeBaseUrl(opts.baseUrl);
    if (isDocEmpty(out)) {
      // 简化：空文档直接写入新对象
      const obj: any = {
        "chatgpt.apiBase": apiBase,
        "chatgpt.config": { preferred_auth_method: "apikey" },
      };
      out = JSON.stringify(obj, null, 2) + "\n";
    } else {
      out = applyEdits(
        out,
        modify(out, ["chatgpt.apiBase"], apiBase, { formattingOptions: fmt }),
      );
      out = applyEdits(
        out,
        modify(out, ["chatgpt.config", "preferred_auth_method"], "apikey", {
          formattingOptions: fmt,
        }),
      );
    }
  }
  return out;
}
