// 供应商配置处理工具函数

// 处理includeCoAuthoredBy字段的添加/删除
export const updateCoAuthoredSetting = (jsonString: string, disable: boolean): string => {
  try {
    const config = JSON.parse(jsonString)
    
    if (disable) {
      // 添加或更新includeCoAuthoredBy字段
      config.includeCoAuthoredBy = false
    } else {
      // 删除includeCoAuthoredBy字段
      delete config.includeCoAuthoredBy
    }
    
    return JSON.stringify(config, null, 2)
  } catch (err) {
    // 如果JSON解析失败，返回原始字符串
    return jsonString
  }
}

// 从JSON配置中检查是否包含includeCoAuthoredBy设置
export const checkCoAuthoredSetting = (jsonString: string): boolean => {
  try {
    const config = JSON.parse(jsonString)
    return config.includeCoAuthoredBy === false
  } catch (err) {
    return false
  }
}

// 从JSON配置中提取并处理官网地址
export const extractWebsiteUrl = (jsonString: string): string => {
  try {
    const config = JSON.parse(jsonString)
    const baseUrl = config?.env?.ANTHROPIC_BASE_URL
    
    if (baseUrl && typeof baseUrl === 'string') {
      // 去掉 "api." 前缀
      return baseUrl.replace(/^https?:\/\/api\./, 'https://')
    }
  } catch (err) {
    // 忽略JSON解析错误
  }
  return ''
}