/**
 * 从API地址推测对应的网站地址
 * @param apiUrl API地址
 * @returns 推测的网站地址，如果无法推测则返回空字符串
 */
export function inferWebsiteUrl(apiUrl: string): string {
  if (!apiUrl || !apiUrl.trim()) {
    return ''
  }

  try {
    const url = new URL(apiUrl.trim())
    
    // 如果是localhost或IP地址，去掉路径部分
    if (url.hostname === 'localhost' || /^\d+\.\d+\.\d+\.\d+$/.test(url.hostname)) {
      return `${url.protocol}//${url.host}`
    }
    
    // 处理域名，去掉api前缀
    let hostname = url.hostname
    
    // 去掉 api. 前缀
    if (hostname.startsWith('api.')) {
      hostname = hostname.substring(4)
    }
    
    // 构建推测的网站地址
    const port = url.port ? `:${url.port}` : ''
    return `${url.protocol}//${hostname}${port}`
    
  } catch (error) {
    // URL解析失败，返回空字符串
    return ''
  }
}