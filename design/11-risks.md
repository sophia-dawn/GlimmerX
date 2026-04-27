# 11 - 风险与应对

> 返回 [DESIGN.md](../DESIGN.md)

| 风险 | 影响 | 应对 |
| --- | --- | --- |
| SQLCipher 三平台编译兼容性 | macOS(ARM/x86)、Windows、Linux 编译失败 | CI 中三平台分别编译并测试 |
| Windows WebView2 运行时缺失 | 部分 Win10 用户缺少运行时 | 安装包自动检测并安装 |
| macOS 平台行为差异 | WebKit 与 EdgeHTML 渲染差异 | 避免平台特定 API；三平台分别测试 |
| 大数据量交易列表性能 | 万条交易时卡顿 | TanStack Table 虚拟滚动 + 分页 |
| 数据丢失风险 | 误操作或文件损坏 | 定期自动备份 + 手动备份双保险 |
