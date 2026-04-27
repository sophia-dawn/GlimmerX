# 10 - 构建与发布

> 返回 [DESIGN.md](../DESIGN.md)

## 开发环境

```bash
npm install
npm run tauri dev        # React HMR + Tauri
npm run dev              # 仅前端（浏览器调试）
```

## 生产构建

```bash
npm run tauri build
# 输出: src-tauri/target/release/bundle/
#   macOS: .app + .dmg
#   Windows: .exe + .msi
#   Linux: .deb + .AppImage
```

## Makefile 工具链

```bash
make help      # 显示所有可用命令
make setup     # 安装所有依赖
make dev       # Tauri dev mode
make dev-web   # 仅前端
make lint      # ESLint + Clippy
make fmt       # Prettier + cargo fmt
make test      # Vitest + cargo test
make check     # 全量检查
make build     # 生产构建
make bundle    # 打包三平台安装包
make clean     # 清理构建产物
```

## CI/CD

- GitHub Actions 自动构建 **三平台**（macOS / Linux / Windows）
- 三平台共享源码、测试用例、覆盖率要求
- `make check` 替代分散的 lint/fmt/type-check
- `make test` 运行测试
- `make bundle` 生成发布产物
- 任一平台失败则整次 CI 失败
