import { useEffect } from 'react';
import { ImportButton } from './components/ImportButton';
import { EpubList } from './components/EpubList';
import { EpubStructureTree } from './components/EpubStructureTree';
import { EpubReader } from './components/EpubReader';
import { useEpubStore } from './store/epubStore';
import './App.css';
import './components/ImportButton.css';
import './components/EpubList.css';
import './components/EpubStructureTree.css';
import './components/EpubReader.css';

function App() {
  const { selectedEpubId, loadEpubStructure } = useEpubStore();

  useEffect(() => {
    if (selectedEpubId) {
      loadEpubStructure(selectedEpubId);
    }
  }, [selectedEpubId]);

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
          <EpubReader />
        </section>
      </main>
    </div>
  );
}

export default App;
