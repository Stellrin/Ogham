# Ogham - EPUB 小说管理器

## 项目概述

基于 Tauri 的 EPUB 格式小说管理器，提供直观 EPUB 文件管理功能。

## 技术栈

### 前端
- **框架**: React + TypeScript
- **构建工具**: Vite
- **状态管理**: Zustand
### 后端
- **语言**: Rust
- **框架**: Tauri 2.x

## 开发规范
- 不需要向后兼容，请放心删除不必要的代码
- 注意这是一个Tauri 桌面应用，调试时不要使用浏览器相关的 API（比如浏览器的开发者工具，console 等），输出调试信息到终端或使用 Tauri 提供的日志功能
- 一定要注意epub文件中的资源路径问题，调整资源后要确保路径正确，避免出现资源无法加载的情况

## 美术风格
- 简洁现代，注重用户体验
- 采用柔和的配色方案，减少视觉疲劳
- 扁平化设计，突出内容编辑区域
- 不要使用渐变色设计

## 核心功能

### 1. EPUB 文件管理
- 导入 EPUB 文件
- 拆解 EPUB 结构（解析 mimetype、META-INF、OEBPS 等标准目录）
- 重新加载和解析 EPUB 内容

### 2. 导出功能
- 导出为标准 EPUB 格式
- 保持原有文件结构和元数据

## 项目结构

```
src/                    # React 前端源码
src-tauri/              # Tauri Rust 后端
  src/                  # Rust 源码
  tauri.conf.json       # Tauri 配置
books/                  # EPUB 示例文件
```

## 开发命令

```bash
npm install            # 安装依赖
npm run dev            # 启动开发服务器
npm run build          # 构建应用
npm run tauri dev      # 启动 Tauri 开发模式
npm run tauri build    # 构建桌面应用
```

### 标准化结构
经过 Ogham 处理的所有 EPUB 将遵循统一结构：
```
mimetype                 # 标准 mimetype 文件
META-INF/
└── container.xml        # 标准容器文件
OEBPS/
├── content.opf          # 标准 OPF 文件
├── toc.ncx              # 标准 NCX 目录（EPUB 2.0）
├── nav.xhtml            # 导航文件（EPUB 3.0，首选）
├── Text/                # 章节目录
│   ├── cover.xhtml      # 封面文件（保持原始命名）
│   ├── episode1.xhtml   # 保持原始命名
│   ├── episode2.xhtml
│   └── ...
├── Styles/              # 样式表（整理到此处）
│   └── style.css
└── Images/              # 图片资源（整理到此处）
    └── ...
```

**标准化原则**：
- **保持原始命名**：章节文件保持原始名称（episode1.xhtml, chapter1.xhtml 等）
- **基于 spine 顺序**：使用 OPF spine 中的 `<itemref>` 顺序作为阅读顺序
- **识别特殊文件**：nav.xhtml、cover.xhtml 等通过 manifest properties 识别
- **整理资源文件**：样式和图片整理到标准目录，但保持文件名不变
