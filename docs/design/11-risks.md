# 11 - 风险与应对

> 返回 [DESIGN.md](../DESIGN.md)

| 风险 | 影响 | 应对 |
| --- | --- | --- |
| SQLCipher 三平台编译兼容性 | macOS(ARM/x86)、Windows、Linux 编译失败 | CI 中三平台分别编译并测试 |
| Windows WebView2 运行时缺失 | 精简镜像 / LTSC / 企业受管设备可能缺少运行时 | NSIS/MSI 安装包使用 `downloadBootstrapper` 按需下载引导程序（不捆绑运行时）；便携版单 exe 无安装器兜底，启动前用 Rust 侧检测并弹出可读提示 |
| macOS 平台行为差异 | WebKit 与 EdgeHTML 渲染差异 | 避免平台特定 API；三平台分别测试 |
| 大数据量交易列表性能 | 万条交易时卡顿 | `@tanstack/react-virtual` 虚拟滚动 + 分页 |
| 数据丢失风险 | 误操作或文件损坏 | 定期自动备份 + 手动备份双保险 |
