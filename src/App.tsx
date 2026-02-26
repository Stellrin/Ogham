import { useEffect } from 'react';
import { ImportButton } from './components/ImportButton';
import { EpubList } from './components/EpubList';
import { EpubStructureTree } from './components/EpubStructureTree';
import { EpubReader } from './components/EpubReader';
import { Toolbar, ConversionMode } from './components/Toolbar';
import { useEpubStore } from './store/epubStore';
import './App.css';
import './components/ImportButton.css';
import './components/EpubList.css';
import './components/EpubStructureTree.css';
import './components/EpubReader.css';
import './components/Toolbar.css';

function App() {
  const {
    selectedEpubId,
    loadEpubStructure,
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
  } = useEpubStore();

  useEffect(() => {
    if (selectedEpubId) {
      loadEpubStructure(selectedEpubId);
    }
  }, [selectedEpubId]);

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
    </div>
  );
}

export default App;
