import type { StandardChapter } from '../store/epubStore';

export type CombinedChapter = StandardChapter;

/**
 * 解析 EPUB 链接，提取章节路径和锚点
 * @param href - 链接地址（如 "chapter1.xhtml#section2"）
 * @returns 解析后的对象 { chapterPath, anchor }
 */
export function parseEpubHref(href: string): { chapterPath: string; anchor: string } {
  // 移除前导空格
  href = href.trim();

  // 处理空链接
  if (!href) {
    return { chapterPath: '', anchor: '' };
  }

  // 纯锚点链接（同页面跳转）
  if (href.startsWith('#')) {
    return { chapterPath: '', anchor: href.slice(1) };
  }

  // 分离锚点
  const hashIndex = href.indexOf('#');
  let chapterPath = href;
  let anchor = '';

  if (hashIndex !== -1) {
    chapterPath = href.substring(0, hashIndex);
    anchor = href.substring(hashIndex + 1);
  }

  // URI 解码
  try {
    chapterPath = decodeURIComponent(chapterPath);
  } catch {
    // 保持原样如果解码失败
  }

  return { chapterPath, anchor };
}

/**
 * 解析相对路径为绝对路径
 * @param basePath - 基础路径（如 "OEBPS/Text"）
 * @param relativePath - 相对路径（如 "../chapter2.xhtml"）
 * @returns 解析后的绝对路径
 */
export function resolveEpubPath(basePath: string, relativePath: string): string {
  // 处理根相对路径（以 / 开头）
  if (relativePath.startsWith('/')) {
    relativePath = relativePath.slice(1);
  }

  const baseParts = basePath.split('/').filter((p) => p);
  const hrefParts = relativePath.split('/').filter((p) => p && p !== '.');

  for (const part of hrefParts) {
    if (part === '..') {
      baseParts.pop();
    } else {
      baseParts.push(part);
    }
  }

  return baseParts.join('/');
}

/**
 * 获取标准化章节路径
 */
export function getChapterPath(chapter: CombinedChapter): string {
  return chapter.standard_path;
}

/**
 * 获取标准化章节名称
 */
export function getChapterName(chapter: CombinedChapter): string {
  return chapter.original_filename || chapter.title || '';
}

/**
 * 去除路径的扩展名，用于匹配
 */
function removeExtension(path: string): string {
  const lastDot = path.lastIndexOf('.');
  if (lastDot > path.lastIndexOf('/')) {
    return path.substring(0, lastDot);
  }
  return path;
}

/**
 * 标准化路径以便匹配（处理大小写、编码等）
 */
function normalizePath(path: string): string {
  return path.toLowerCase().replace(/%20/g, ' ');
}

/**
 * 检查两个路径是否匹配（使用多种策略）
 */
function isPathMatch(targetPath: string, chapterPath: string, currentBasePath?: string): boolean {
  // 策略1: 完全匹配
  if (targetPath === chapterPath) {
    return true;
  }

  // 策略2: 尝试相对路径解析
  if (currentBasePath) {
    const resolved = resolveEpubPath(currentBasePath, targetPath);
    if (resolved === chapterPath) {
      return true;
    }
  }

  // 策略3: 后缀匹配
  const normalizedTarget = normalizePath(targetPath);
  const normalizedChapter = normalizePath(chapterPath);
  if (normalizedChapter.endsWith(normalizedTarget) ||
      normalizedChapter.endsWith(normalizedTarget + '.xhtml') ||
      normalizedChapter.endsWith(normalizedTarget + '.html')) {
    return true;
  }

  // 策略4: 去扩展名后匹配
  const targetNoExt = removeExtension(normalizePath(targetPath));
  const chapterNoExt = removeExtension(normalizedChapter);
  if (targetNoExt === chapterNoExt) {
    return true;
  }

  // 策略5: 文件名匹配
  const targetFileName = targetPath.split('/').pop() || '';
  const chapterFileName = chapterPath.split('/').pop() || '';
  if (targetFileName && chapterFileName) {
    const normalizedTargetFile = normalizePath(targetFileName);
    const normalizedChapterFile = normalizePath(chapterFileName);
    if (normalizedTargetFile === normalizedChapterFile ||
        removeExtension(normalizedTargetFile) === removeExtension(normalizedChapterFile)) {
      return true;
    }
  }

  return false;
}

/**
 * 根据链接查找目标章节
 * @param targetHref - 目标链接（如 "chapter2.xhtml#section1"）
 * @param chapters - 章节数组
 * @param currentChapterPath - 当前章节路径（用于解析相对路径）
 * @returns 找到的章节，未找到返回 undefined
 */
export function findChapterByHref(
  targetHref: string,
  chapters: CombinedChapter[],
  currentChapterPath?: string
): CombinedChapter | undefined {
  const { chapterPath: targetPath } = parseEpubHref(targetHref);

  // 如果没有章节路径，只返回当前章节（锚点跳转）
  if (!targetPath) {
    if (currentChapterPath) {
      return chapters.find((c) => getChapterPath(c) === currentChapterPath);
    }
    return undefined;
  }

  // 获取当前章节的基础路径（用于解析相对路径）
  const currentBasePath = currentChapterPath
    ? currentChapterPath.split('/').slice(0, -1).join('/')
    : 'OEBPS/Text';

  // 尝试查找匹配的章节
  for (const chapter of chapters) {
    const chapterPath = getChapterPath(chapter);
    if (isPathMatch(targetPath, chapterPath, currentBasePath)) {
      return chapter;
    }
  }

  return undefined;
}
