import React from 'react';
import { ImportButton } from './components/ImportButton';
import { EpubList } from './components/EpubList';
import { EpubStructureTree } from './components/EpubStructureTree';
import './App.css';
import './components/ImportButton.css';
import './components/EpubList.css';
import './components/EpubStructureTree.css';

function App() {
  return (
    <div className="app">
      <header className="app-header">
        <h1>Ogham</h1>
        <span className="app-subtitle">EPUB 小说管理器</span>
      </header>

      <main className="app-main">
        <aside className="app-sidebar">
          <EpubList />
          <div className="sidebar-footer">
            <ImportButton />
          </div>
        </aside>

        <section className="app-content">
          <EpubStructureTree />
        </section>
      </main>
    </div>
  );
}

export default App;
