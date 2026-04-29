import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ImportButton } from './components/ImportButton';
import { EpubList } from './components/EpubList';
import { EpubStructureTree } from './components/EpubStructureTree';
import { EpubReader } from './components/EpubReader';
import { Toolbar, type ConversionMode } from './components/Toolbar';
import { NotificationCenter } from './components/NotificationCenter';
import { useEpubStore } from './store/epubStore';
import './App.css';
import './components/ImportButton.css';
import './components/EpubList.css';
import './components/EpubStructureTree.css';
import './components/EpubReader.css';
import './components/Toolbar.css';
import './components/NotificationCenter.css';

const EPUB_OPEN_REQUESTED_EVENT = 'epub-open-requested';

function normalizeOpenPaths(paths: string[]): string[] {
  const seen = new Set<string>();
  const normalizedPaths: string[] = [];

  for (const path of paths) {
    if (!path.toLowerCase().endsWith('.epub')) continue;

    const key = path.replace(/\\/g, '/').toLowerCase();
    if (!seen.has(key)) {
      seen.add(key);
      normalizedPaths.push(path);
    }
  }

  return normalizedPaths;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function App() {
  const startupHandledRef = useRef(false);
  const openQueueRef = useRef<Promise<void>>(Promise.resolve());

  const {
    epubs,
    selectedEpubId,
    isConverting,
    conversionProgress,
    convertSimplifiedTraditional,
    processAllImages,
    isProcessingImages,
    imageProcessingProgress,
    imageProcessingTotal,
    imageProcessingCurrentChapter,
    imageProcessingCurrentImageUrl,
    imageProcessingSuccess,
    imageProcessingFailed,
    imageProcessingSkipped,
    imageProcessingProcessedUnique,
    importEpubFromPath,
    addNotification,
  } = useEpubStore();

  useEffect(() => {
    let removeListener: (() => void) | undefined;
    let shouldRemoveListener = false;

    const enqueueOpenPaths = (paths: string[]) => {
      const epubPaths = normalizeOpenPaths(paths);
      if (epubPaths.length === 0) return;

      openQueueRef.current = openQueueRef.current.then(async () => {
        for (const filePath of epubPaths) {
          try {
            await importEpubFromPath(filePath);
          } catch {
            // importEpubFromPath 已经把错误展示到通知中心
          }
        }
      });
    };

    listen<string[]>(EPUB_OPEN_REQUESTED_EVENT, (event) => {
      enqueueOpenPaths(event.payload);
    })
      .then((unlisten) => {
        if (shouldRemoveListener) {
          unlisten();
        } else {
          removeListener = unlisten;
        }
      })
      .catch((error) => {
        addNotification({
          kind: 'error',
          title: '系统打开事件监听失败',
          details: getErrorMessage(error),
          timeoutMs: 0,
        });
      });

    if (!startupHandledRef.current) {
      startupHandledRef.current = true;
      void invoke<string[]>('get_startup_epub_paths_command')
        .then(enqueueOpenPaths)
        .catch((error) => {
          addNotification({
            kind: 'error',
            title: '读取启动 EPUB 失败',
            details: getErrorMessage(error),
            timeoutMs: 0,
          });
        });
    }

    return () => {
      shouldRemoveListener = true;
      if (removeListener) {
        removeListener();
      }
    };
  }, [addNotification, importEpubFromPath]);

  const handleConversion = async (mode: ConversionMode) => {
    try {
      await convertSimplifiedTraditional(mode);
    } catch (error) {
      // Error is already handled in the store
    }
  };

  const handleProcessAllImages = async () => {
    try {
      await processAllImages();
    } catch (error) {
      // Error is already handled in the store
    }
  };

  const selectedEpub = epubs.find((epub) => epub.id === selectedEpubId);
  const canRunTools = Boolean(selectedEpub?.refactoredStructure);

  return (
    <div className="app">
      <header className="app-header">
        <h1>Ogham</h1>
        <span className="app-subtitle">EPUB 小说管理器</span>
      </header>

      <main className="app-main">
        <aside className="app-sidebar">
          <EpubList />
          <EpubStructureTree />
          <div className="sidebar-footer">
            <ImportButton />
          </div>
        </aside>

        <section className="app-content">
          <Toolbar
            onConvert={handleConversion}
            isConverting={isConverting}
            conversionProgress={conversionProgress}
            onProcessAllImages={handleProcessAllImages}
            isProcessingImages={isProcessingImages}
            canRunTools={canRunTools}
            imageProcessingProgress={imageProcessingProgress}
            imageProcessingTotal={imageProcessingTotal}
            imageProcessingCurrentChapter={imageProcessingCurrentChapter}
            imageProcessingCurrentImageUrl={imageProcessingCurrentImageUrl}
            imageProcessingSuccess={imageProcessingSuccess}
            imageProcessingFailed={imageProcessingFailed}
            imageProcessingSkipped={imageProcessingSkipped}
            imageProcessingProcessedUnique={imageProcessingProcessedUnique}
          />
          <EpubReader />
        </section>
      </main>
      <NotificationCenter />
    </div>
  );
}

export default App;
