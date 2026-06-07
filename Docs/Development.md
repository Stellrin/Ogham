# Ogham 开发说明

这份文档放置项目开发相关内容。根目录 `README.md` 主要面向开源项目介绍，这里记录运行命令、架构约定、EPUB 管理目录和验证建议。

## 项目定位

Ogham 是基于 Tauri 2 的 EPUB 小说管理器。应用导入原始 EPUB 后，会拆解并标准化为统一的管理目录，再在标准化结构上进行阅读、目录维护、简繁转换、图片处理、资源索引和导出。

当前项目不需要向后兼容旧实现。遇到过时或绕远的代码，可以直接删除和重构，但 EPUB 资源路径属于高风险区域，调整后需要重点验证。

## 开发命令

```bash
npm install
npm run tauri dev
npm run dev
npm run build
npm run tauri build
```

- `npm run tauri dev` 是主要调试入口，会启动 Vite 和 Tauri 桌面应用。
- `npm run dev` 只启动 Vite 前端服务，不能完整代表桌面端行为。
- `npm run build` 执行 TypeScript 检查并构建前端资源。
- `npm run tauri build` 构建桌面应用。

涉及文件系统、弹窗、Rust command、导入导出、图片下载等行为时，优先用 `npm run tauri dev` 调试。

## 技术栈

前端：

- React + TypeScript
- Vite
- Zustand
- Tauri JavaScript API

后端：

- Rust
- Tauri 2
- `zip`
- `quick-xml`
- `epub`
- `zhconv`
- `reqwest`

## 目录概览

```text
src/
  components/        React components
  store/             Zustand state and Tauri command calls
  utils/             EPUB path helpers

src-tauri/
  src/epub/          EPUB parsing, normalization, TOC, resources, export
  capabilities/      Tauri permission configuration
  tauri.conf.json    Tauri app configuration
```

## 关键数据流

- `import_epub_command`：导入原始 EPUB，并立即标准化到 Ogham 管理目录。
- `refactor_epub_command`：重新解析原始 EPUB，会生成新的 `epub_id` 和缓存目录。
- `reload_epub_structure_command`：从已有管理目录重新读取结构，保持相同的 `epub_id`。
- `get_chapter_from_refactored_command`：从标准化管理目录读取章节内容。
- `get_image_from_refactored_command`：从标准化管理目录读取图片内容。
- `export_epub_command`：将管理目录重新打包为 EPUB。

前端应主要使用 `epubId`、`refactoredStructure` 和标准化路径，不要把原始 EPUB 内部路径与管理目录路径混用。

修改章节、目录、图片、样式、OPF、NCX、NAV 或资源索引后，优先调用 `reload_epub_structure_command` 重新加载 EPUB 结构，让前端显示最新落盘状态。

## 标准化 EPUB 结构

标准化后的 EPUB 根目录包含：

```text
mimetype
META-INF/
OEBPS/
  content.opf
  nav.xhtml
  toc.ncx
  Text/
  Styles/
  Images/
  Fonts/
```

约定：

- `OEBPS/content.opf` 是 manifest、spine 和资源声明的核心来源。
- `OEBPS/nav.xhtml` 与 `OEBPS/toc.ncx` 都需要和目录操作保持同步。
- 正文文件放在 `OEBPS/Text/`。
- 样式放在 `OEBPS/Styles/`。
- 图片放在 `OEBPS/Images/`。
- 字体放在 `OEBPS/Fonts/`。
- 写入 OPF manifest 时使用相对 `OEBPS` 的 href，例如 `Images/cover.jpg`。
- 前端展示和读取时通常使用带 `OEBPS/` 前缀的路径，例如 `OEBPS/Images/cover.jpg`。
- 修改资源后检查 `.ogham/resource_index.json` 是否需要刷新。

## 路径和资源注意事项

EPUB 资源路径是高风险区域。调整资源后必须确认 `Text/`、`Styles/`、`Images/`、`Fonts/` 等目录之间的相对引用正确，避免导出后资源丢失或无法显示。

常见检查点：

- 章节内图片 `src` 是否相对当前章节文件正确。
- CSS 中引用的图片或字体路径是否相对 CSS 文件正确。
- OPF manifest 中的 href 是否相对 `OEBPS`。
- `nav.xhtml` 和 `toc.ncx` 是否指向实际存在的章节。
- 新增、删除或移动资源后，资源索引是否刷新。

## 验证建议

- 文档调整：检查格式、链接和明显拼写问题。
- 前端状态或 UI 调整：运行 `npm run build`，必要时用 `npm run tauri dev` 试关键流程。
- Rust EPUB 逻辑调整：运行相关 Rust 测试或 `npm run tauri build`。
- 会改写 EPUB 管理目录的功能：用真实 EPUB 验证导入、刷新、目录视图、阅读器和导出结果。

不要依赖浏览器开发者工具或浏览器专用 API 调试桌面行为。后端信息输出到终端，前端状态通过应用 UI 或 Tauri 事件验证。
