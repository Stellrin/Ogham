import React, { useRef, useEffect, useState } from 'react';
import { useEpubStore, Chapter, StandardChapter } from '../store/epubStore';
import { openUrl } from '@tauri-apps/plugin-opener';
import './EpubReader.css';

type CombinedChapter = Chapter | StandardChapter;

// 获取章节路径的辅助函数
const getChapterPath = (chapter: CombinedChapter): string => {
  if ('path' in chapter) {
    return chapter.path;
  } else {
    return chapter.standard_path;
  }
};

// 获取章节名称的辅助函数
const getChapterName = (chapter: CombinedChapter): string | undefined => {
  if ('name' in chapter) {
    return chapter.name;
  } else {
    return chapter.title || chapter.original_filename;
  }
};

export const EpubReader: React.FC = () => {
  const { epubs, selectedEpubId, readerState, loadChapterContent, loadRefactoredChapter, setReaderState } =
    useEpubStore();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedEpub = epubs.find((e) => e.id === selectedEpubId);

  // 优先使用重构后的结构，回退到原始结构
  const chapters = selectedEpub?.refactoredStructure?.structure.chapters ||
                   selectedEpub?.structure?.chapters ||
                   [];

  // 查找当前章节
  const currentChapter = chapters.find(
    (c) => getChapterPath(c) === readerState.currentChapterPath
  );

  // 检查是否使用重构后的 EPUB
  const useRefactored = !!selectedEpub?.refactoredStructure?.epubId;

  // 当章节路径改变时加载章节内容
  useEffect(() => {
    if (readerState.viewingImagePath && readerState.viewingImageData) {
      renderImage(readerState.viewingImageData, readerState.viewingImagePath);
    } else if (readerState.currentChapterPath && !currentChapter?.content) {
      loadChapter(readerState.currentChapterPath);
    } else if (currentChapter?.content) {
      renderContent(currentChapter.content.html);
    }
  }, [readerState.currentChapterPath, currentChapter?.content, readerState.viewingImagePath, readerState.viewingImageData]);

  // 当阅读器设置改变时重新渲染
  useEffect(() => {
    if (readerState.viewingImagePath && readerState.viewingImageData) {
      renderImage(readerState.viewingImageData, readerState.viewingImagePath);
    } else if (currentChapter?.content) {
      renderContent(currentChapter.content.html);
    }
  }, [readerState.fontSize, readerState.fontFamily, readerState.lineHeight]);

  const loadChapter = async (chapterPath: string) => {
    setLoading(true);
    setError(null);
    try {
      if (useRefactored && selectedEpub?.epubId) {
        await loadRefactoredChapter(selectedEpub.epubId, chapterPath);
      } else {
        await loadChapterContent(chapterPath);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const renderImage = (imageData: string, imagePath: string) => {
    const iframe = iframeRef.current;
    if (!iframe) return;

    const doc = iframe.contentDocument;
    if (!doc) return;

    // Extract filename from path for display
    const filename = imagePath.split('/').pop() || imagePath;

    const imageHtml = `
      <!DOCTYPE html>
      <html>
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <style>
            * {
              box-sizing: border-box;
            }
            body {
              margin: 0;
              padding: 20px;
              font-family: ${readerState.fontFamily};
              font-size: ${readerState.fontSize}px;
              line-height: ${readerState.lineHeight};
              color: #333;
              background-color: #ffffff;
              display: flex;
              flex-direction: column;
              align-items: center;
              min-height: 100vh;
            }
            .image-header {
              text-align: center;
              margin-bottom: 20px;
              padding-bottom: 10px;
              border-bottom: 1px solid #e0e0e0;
              width: 100%;
              max-width: 800px;
            }
            .image-filename {
              font-size: 14px;
              color: #666;
              font-family: 'Consolas', 'Monaco', monospace;
            }
            .image-container {
              display: flex;
              justify-content: center;
              align-items: center;
              width: 100%;
              flex: 1;
            }
            img {
              max-width: 100%;
              max-height: calc(100vh - 150px);
              height: auto;
              display: block;
              object-fit: contain;
            }
          </style>
        </head>
        <body>
          <div class="image-header">
            <span class="image-filename">${filename}</span>
          </div>
          <div class="image-container">
            <img src="data:image;base64,${imageData}" alt="${filename}" />
          </div>
        </body>
      </html>
    `;

    doc.open();
    doc.write(imageHtml);
    doc.close();

    // 恢复滚动位置
    if (readerState.scrollPosition > 0) {
      setTimeout(() => {
        if (doc.documentElement) {
          doc.documentElement.scrollTop = readerState.scrollPosition;
        }
      }, 0);
    }
  };

  const renderContent = (html: string) => {
    const iframe = iframeRef.current;
    if (!iframe) return;

    const doc = iframe.contentDocument;
    if (!doc) return;

    const enhancedHtml = `
      <!DOCTYPE html>
      <html>
        <head>
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <style>
            * {
              box-sizing: border-box;
            }
            body {
              margin: 0;
              padding: 20px;
              font-family: ${readerState.fontFamily};
              font-size: ${readerState.fontSize}px;
              line-height: ${readerState.lineHeight};
              color: #333;
              background-color: #ffffff;
            }
            img {
              max-width: 100%;
              height: auto;
              display: block;
              margin: 10px auto;
            }
            p {
              margin: 0.5em 0;
            }
            h1, h2, h3, h4, h5, h6 {
              margin: 0.8em 0 0.4em;
              line-height: 1.3;
            }
            a {
              color: #4a90d9;
              text-decoration: none;
            }
            a:hover {
              text-decoration: underline;
              cursor: pointer;
            }
          </style>
        </head>
        <body>
          ${html}
        </body>
      </html>
    `;

    doc.open();
    doc.write(enhancedHtml);
    doc.close();

    // 恢复滚动位置
    if (readerState.scrollPosition > 0) {
      setTimeout(() => {
        if (doc.documentElement) {
          doc.documentElement.scrollTop = readerState.scrollPosition;
        }
      }, 0);
    }

    // 添加事件监听
    attachEventListeners();
  };

  const attachEventListeners = () => {
    const iframe = iframeRef.current;
    if (!iframe || !iframe.contentDocument) return;

    // 监听点击事件处理内部链接
    iframe.contentDocument.addEventListener('click', handleLinkClick);

    // 监听滚动事件
    iframe.contentDocument.addEventListener('scroll', handleScroll);
  };

  const handleLinkClick = (event: MouseEvent) => {
    const target = event.target as HTMLElement;
    if (target.tagName === 'A') {
      const anchor = target as HTMLAnchorElement;
      const href = anchor.getAttribute('href');

      if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
        // 外部链接：在默认浏览器中打开
        event.preventDefault();
        openUrl(href);
      } else if (href && (href.startsWith('./') || href.startsWith('../') || href.endsWith('.html') || href.endsWith('.xhtml'))) {
        // 内部章节链接
        event.preventDefault();
        navigateToChapter(href);
      } else if (href && href.startsWith('#')) {
        // 处理页面内锚点跳转
        event.preventDefault();
        const iframe = iframeRef.current;
        if (iframe?.contentDocument) {
          const targetElement = iframe.contentDocument.querySelector(href);
          if (targetElement) {
            targetElement.scrollIntoView({ behavior: 'smooth' });
          }
        }
      }
    }
  };

  const navigateToChapter = (href: string) => {
    if (!selectedEpub) return;

    // 解析相对路径
    const basePath = readerState.currentChapterPath?.split('/').slice(0, -1).join('/') || 'OEBPS/Text';
    const resolvedPath = resolvePath(basePath, href);

    // 优先在重构后的结构中查找
    const targetChapter = chapters.find((c) => {
      const path = getChapterPath(c);
      return path === resolvedPath || path.endsWith(href) || path.endsWith(href.replace('../', ''));
    });

    if (targetChapter) {
      const chapterPath = getChapterPath(targetChapter);
      setReaderState({
        currentChapterIndex: targetChapter.order,
        currentChapterPath: chapterPath,
        scrollPosition: 0,
      });
    }
  };

  const resolvePath = (basePath: string, href: string): string => {
    const baseParts = basePath.split('/').filter((p) => p);
    const hrefParts = href.split('/').filter((p) => p && p !== '.');

    for (const part of hrefParts) {
      if (part === '..') {
        baseParts.pop();
      } else {
        baseParts.push(part);
      }
    }

    return baseParts.join('/');
  };

  const handleScroll = () => {
    const iframe = iframeRef.current;
    if (!iframe) return;

    const scrollTop = iframe.contentDocument?.documentElement.scrollTop || 0;
    setReaderState({ scrollPosition: scrollTop });
  };

  const handleFontSizeChange = (size: number) => {
    setReaderState({ fontSize: size });
  };

  return (
    <div className="epub-reader">
      {loading && (
        <div className="reader-loading">
          <span>加载中...</span>
        </div>
      )}
      {error && (
        <div className="reader-error">
          <span>加载失败: {error}</span>
        </div>
      )}

      {!selectedEpub && (
        <div className="reader-empty">
          <p>请选择一个 EPUB 文件</p>
          <p className="hint">在左侧列表中选择以开始阅读</p>
        </div>
      )}

      {selectedEpub && !readerState.currentChapterPath && !readerState.viewingImagePath && (
        <div className="reader-empty">
          <p>请选择一个章节</p>
          <p className="hint">在左侧结构树中点击章节开始阅读</p>
        </div>
      )}

      {selectedEpub && (readerState.currentChapterPath || readerState.viewingImagePath) && (
        <>
          <iframe
            ref={iframeRef}
            className="reader-iframe"
            sandbox="allow-same-origin allow-scripts"
            title="EPUB Reader"
          />
          <ReaderControls
            fontSize={readerState.fontSize}
            onFontSizeChange={handleFontSizeChange}
            chapterName={readerState.viewingImagePath || (currentChapter ? getChapterName(currentChapter) : undefined)}
            isImage={!!readerState.viewingImagePath}
          />
        </>
      )}
    </div>
  );
};

interface ReaderControlsProps {
  fontSize: number;
  onFontSizeChange: (size: number) => void;
  chapterName?: string;
  isImage?: boolean;
}

const ReaderControls: React.FC<ReaderControlsProps> = ({
  fontSize,
  onFontSizeChange,
  chapterName,
  isImage = false,
}) => {
  const displayName = isImage && chapterName
    ? chapterName.split('/').pop() || chapterName
    : chapterName || '';

  return (
    <div className="reader-controls">
      <div className="reader-info">
        <span className="reader-chapter-name">{displayName}</span>
      </div>

      <div className="reader-settings">
        <select
          value={fontSize}
          onChange={(e) => onFontSizeChange(Number(e.target.value))}
          className="font-size-select"
          title="字体大小"
        >
          <option value="14">小</option>
          <option value="16">中</option>
          <option value="18">大</option>
          <option value="20">特大</option>
        </select>
      </div>
    </div>
  );
};
