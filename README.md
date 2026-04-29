# Ogham - EPUB 小说管理器

Ogham 是一个基于 Tauri 2 的桌面 EPUB 小说管理器，用于导入、标准化、阅读、整理并导出 EPUB 文件。它会把不同来源的 EPUB 拆解到统一的管理目录中，再围绕标准化后的结构进行阅读、目录维护、资源处理和导出。

[![Tauri](https://img.shields.io/badge/Tauri-2.x-orange)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18.3-blue)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue)](https://www.typescriptlang.org/)

## 核心功能

- **导入与标准化**：验证 EPUB 文件，解析元数据、manifest、spine、目录和资源，并整理为统一的 `OEBPS` 结构。
- **结构浏览**：以文件树查看章节、样式、图片等资源，也可以切换到目录视图查看 TOC。
- **内置阅读器**：渲染章节 HTML，支持字号调整、滚动位置恢复、章节跳转和图片查看。
- **目录维护**：支持加载目录、更新目录项标题、调整目录项关联文件和顺序。
- **文本转换**：支持简体转繁体、繁体转简体，并显示转换进度。
- **图片链接处理**：扫描章节中的图片链接，下载并写入 `OEBPS/Images`，同步更新 OPF manifest。
- **资源索引**：维护资源引用关系，辅助检查章节、样式、图片、字体等资源的路径关系。
- **导出 EPUB**：将管理目录重新打包为标准 EPUB 文件，保留元数据和整理后的结构。

## 快速开始

### 环境要求

- Node.js 与 npm
- Rust 工具链
- Tauri 2 所需的系统依赖

### 安装依赖

```bash
npm install
```

### 开发运行

```bash
npm run tauri dev
```

这是主要的桌面调试入口，会同时启动 Vite 前端服务和 Tauri 应用。`npm run dev` 只会启动 Vite 前端服务，不能代表完整的桌面端行为。

### 构建

```bash
npm run build
npm run tauri build
```

`npm run build` 会执行 TypeScript 检查并构建前端资源。`npm run tauri build` 会构建桌面应用安装包。

## 技术栈

### 前端

- React 18 + TypeScript
- Vite 6
- Zustand
- Tauri JavaScript API 与官方插件

### 后端

- Rust
- Tauri 2
- `zip`：EPUB/ZIP 归档处理
- `quick-xml`：OPF、NCX、NAV 等 XML/XHTML 处理
- `epub`：EPUB 解析辅助
- `zhconv`：简繁转换
- `reqwest`：图片下载

## 项目结构

```text
src/
  components/        React 组件
  store/             Zustand 状态与 Tauri command 调用
  utils/             EPUB 路径处理工具
src-tauri/
  src/epub/          EPUB 解析、标准化、目录、资源、导出逻辑
  capabilities/      Tauri 权限配置
  tauri.conf.json    Tauri 应用配置
```

## 开发注意事项

- 这是 Tauri 桌面应用，涉及文件系统、对话框和 Rust command 的功能应优先在 `npm run tauri dev` 中验证。
- 前端主要操作标准化后的 `epubId` 和 `refactoredStructure`，不要混用原始 EPUB 路径与管理目录路径。
- 修改章节、目录、图片、OPF 或资源后，需要重新加载 EPUB 结构，保证前端显示的是管理目录中的最新状态。
- EPUB 内部资源路径必须保持相对关系正确，尤其是 `Text/`、`Styles/`、`Images/`、`Fonts/` 之间的引用。

## 许可证

本项目采用 MIT License，详见 [LICENSE](LICENSE)。
