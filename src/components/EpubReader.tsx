import React, { useRef, useEffect, useState } from 'react';
import { useEpubStore } from '../store/epubStore';
import { openUrl } from '@tauri-apps/plugin-opener';
import { getChapterName } from '../utils/epubPathUtils';
import './EpubReader.css';
import { Loader2 } from 'lucide-react';

export const EpubReader: React.FC = () => {
  const { epubs, selectedEpubId, readerState, loadRefactoredChapter, setReaderState, resolveChapterHref } =
    useEpubStore();
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedEpub = epubs.find((e) => e.id === selectedEpubId);

  const chapters = selectedEpub?.refactoredStructure?.structure.chapters || [];

  const currentChapterPath = readerState.currentChapterPath || '';
  const currentChapter = chapters.find(
    (c) => c.standard_path === currentChapterPath
  );

  // 当章节路径改变时加载章节内容
  useEffect(() => {
    if (readerState.viewingImagePath && readerState.viewingImageData) {
      renderImage(readerState.viewingImageData, readerState.viewingImagePath);
    } else if (currentChapterPath && !currentChapter?.content) {
      loadChapter(currentChapterPath);
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

  // 当章节内容加载完成且有待处理锚点时
  useEffect(() => {
    if (readerState.pendingAnchor && iframeRef.current?.contentDocument) {
      setTimeout(() => {
        scrollToAnchor(readerState.pendingAnchor || '');
        setReaderState({ pendingAnchor: null });
      }, 100);
    }
  }, [readerState.pendingAnchor, currentChapter?.content]);

  const loadChapter = async (chapterPath: string) => {
    setLoading(true);
    setError(null);
    try {
      if (!selectedEpub?.epubId) {
        throw new Error('EPUB 尚未完成标准化导入');
      }
      await loadRefactoredChapter(selectedEpub.epubId, chapterPath);
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
    const filename = escapeHtml(imagePath.split('/').pop() || imagePath);

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
              background-color: #fbfbf8;
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

    const sanitizedHtml = sanitizeEpubHtml(html);
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
              background-color: #fbfbf8;
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
          ${sanitizedHtml}
        </body>
      </html>
    `;

    doc.open();
    doc.write(enhancedHtml);
    doc.close();

    // 等待 DOM 完全解析后再附加事件监听器
    setTimeout(() => {
      attachEventListeners();
    }, 150);

    // 恢复滚动位置
    if (readerState.scrollPosition > 0) {
      setTimeout(() => {
        if (doc.documentElement) {
          doc.documentElement.scrollTop = readerState.scrollPosition;
        }
      }, 0);
    }
  };

  const attachEventListeners = () => {
    const iframe = iframeRef.current;
    if (!iframe || !iframe.contentDocument) return;

    // 移除旧的监听器（防止重复添加）
    try {
      iframe.contentDocument.removeEventListener('click', handleLinkClick, true);
      iframe.contentDocument.removeEventListener('scroll', handleScroll);
    } catch (e) {
      // 忽略移除不存在的监听器的错误
    }

    // 使用捕获阶段监听点击事件处理内部链接
    iframe.contentDocument.addEventListener('click', handleLinkClick, true);

    // 监听滚动事件
    iframe.contentDocument.addEventListener('scroll', handleScroll);
  };

  const scrollToAnchor = (anchorId: string) => {
    if (!anchorId) return;
    const iframe = iframeRef.current;
    if (!iframe?.contentDocument) return;

    const targetElement = iframe.contentDocument.getElementById(anchorId);
    const namedElement = !targetElement
      ? iframe.contentDocument.querySelector(`[name="${anchorId}"]`)
      : null;

    const element = targetElement || namedElement;
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
      setReaderState({ scrollPosition: iframe.contentDocument.documentElement.scrollTop });
    }
  };

  const handleLinkClick = (event: MouseEvent) => {
    const target = event.target as HTMLElement;
    // 使用 closest 查找最近的 <a> 标签（处理点击链接内子元素的情况）
    const anchor = target.closest('A');

    if (!anchor) return;

    const href = anchor.getAttribute('href');
    if (!href) return;

    // 外部链接：在默认浏览器中打开
    if (href.startsWith('http://') || href.startsWith('https://')) {
      event.preventDefault();
      openUrl(href);
      return;
    }

    // EPUB 内部链接交给后端按当前管理目录解析
    event.preventDefault();
    navigateToChapterWithAnchor(href).catch((error) => {
      setError(error instanceof Error ? error.message : String(error));
    });
  };

  const navigateToChapterWithAnchor = async (href: string) => {
    const resolved = await resolveChapterHref(href);
    const chapterPath = resolved.chapterPath || resolved.chapter_path;
    const anchor = resolved.anchor || null;
    const sameChapter = resolved.sameChapter ?? resolved.same_chapter ?? false;

    if (sameChapter) {
      scrollToAnchor(anchor || '');
      return;
    }

    if (chapterPath) {
      const targetChapter = chapters.find((chapter) => chapter.standard_path === chapterPath);
      setReaderState({
        currentChapterIndex: targetChapter?.order ?? readerState.currentChapterIndex,
        currentChapterPath: chapterPath,
        scrollPosition: 0,
        pendingAnchor: anchor,
      });
    }
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
          <Loader2 className="reader-state-icon is-spinning" size={18} aria-hidden="true" />
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
          <p className="hint">在左侧目录中点击章节开始阅读</p>
        </div>
      )}

      {selectedEpub && (readerState.currentChapterPath || readerState.viewingImagePath) && (
        <>
          <iframe
            ref={iframeRef}
            className="reader-iframe"
            title="EPUB Reader"
            sandbox="allow-same-origin"
          />
          <ReaderControls
            fontSize={readerState.fontSize}
            onFontSizeChange={handleFontSizeChange}
            chapterName={readerState.viewingImagePath || (currentChapter ? getChapterName(currentChapter) : undefined)}
          />
        </>
      )}
    </div>
  );
};

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function sanitizeEpubHtml(html: string): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(`<body>${html}</body>`, 'text/html');

  doc.querySelectorAll('script, iframe, object, embed, base, form').forEach((node) => node.remove());
  doc.querySelectorAll('meta[http-equiv]').forEach((node) => {
    const value = node.getAttribute('http-equiv')?.toLowerCase();
    if (value === 'refresh') {
      node.remove();
    }
  });

  doc.body.querySelectorAll('*').forEach((element) => {
    for (const attr of [...element.attributes]) {
      const name = attr.name.toLowerCase();
      const value = attr.value.trim();

      if (name.startsWith('on')) {
        element.removeAttribute(attr.name);
        continue;
      }

      if ((name === 'href' || name === 'src' || name === 'xlink:href') && /^javascript:/i.test(value)) {
        element.removeAttribute(attr.name);
        continue;
      }

      if (name === 'style' && /(expression\s*\(|url\s*\(\s*['"]?\s*javascript:)/i.test(value)) {
        element.removeAttribute(attr.name);
      }
    }
  });

  return doc.body.innerHTML;
}

interface ReaderControlsProps {
  fontSize: number;
  onFontSizeChange: (size: number) => void;
  chapterName?: string;
}

const ReaderControls: React.FC<ReaderControlsProps> = ({
  fontSize,
  onFontSizeChange,
  chapterName,
}) => {
  const displayName = chapterName ? chapterName.split('/').pop() || chapterName : '';

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
