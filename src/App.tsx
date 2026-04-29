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

function App() {
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
  } = useEpubStore();

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
