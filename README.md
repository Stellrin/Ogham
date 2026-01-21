# Ogham - EPUB 小说管理器

<div align="center">

**基于 Tauri 的 EPUB 管理与阅读工具**

[![Tauri](https://img.shields.io/badge/Tauri-2.x-orange)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-18.3-blue)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue)](https://www.typescriptlang.org/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

</div>

## ✨ 简介

Ogham 是一款专为管理和阅读 EPUB 格式小说而设计桌面应用程序。它提供直观的 EPUB 文件管理功能，能够深度解析、重构和导出 EPUB 文件，同时保持标准化的内部结构。

### 核心特性

- 📚 **智能 EPUB 导入** - 支持标准 EPUB 文件的导入与验证
- 🔍 **深度解析** - 完整解析 EPUB 结构（元数据、目录、章节、资源）
- 🌳 **可视化结构树** - 交互式展示 EPUB 内部结构
- 📖 **内置阅读器** - 支持 HTML 内容渲染、字体大小调整
- 🖼️ **图片查看** - 独立图片查看器，支持嵌入式图片
- 🔄 **结构重构** - 将不同来源的 EPUB 标准化为统一格式
- 📤 **导出功能** - 导出重构后的 EPUB 文件

## 🎯 技术栈

### 前端
- **框架**: React 18.3.1 + TypeScript 5.6.2
- **构建工具**: Vite 6.0.3
- **状态管理**: Zustand 5.0.10
- **桌面集成**: Tauri 2.x

### 后端
- **语言**: Rust
- **框架**: Tauri 2.x
- **核心依赖**:
  - `zip` - ZIP 归档处理
  - `epub` - EPUB 格式解析
  - `quick-xml` - XML 解析
  - `serde` - 序列化框架


## 🔧 核心功能

### EPUB 解析

- 完整解析 EPUB 结构
- 提取元数据（标题、作者、语言、标识符）
- 解析 manifest 和 spine
- 提取目录结构（TOC）

### 结构可视化

- 交互式树状展示
- 实时显示章节、样式、图片资源
- 支持目录折叠/展开

### 阅读功能

- iframe 渲染 HTML 内容
- 字体大小调整（小/中/大/特大）
- 滚动位置持久化
- 支持嵌入式图片

### 重构与导出

- 完整的 EPUB 结构重构
- 生成标准的 OPF、NCX、NAV 文件
- 导出为标准 EPUB 格式
- 保持原有元数据



## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件


<div align="center">

**Made with ❤️ by Stellrin**

</div>
