import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface Chapter {
  id: string;
  path: string;
  name: string;
  order: number;
  content?: ChapterContent;
}

export interface ChapterContent {
  html: string;
  resources: Record<string, ResourceData>;
}

export interface ResourceData {
  mimeType: string;
  data: string;
}

export interface EpubMetadata {
  title: string;
  author?: string;
  language?: string;
  identifier: string;
}

export interface EpubStructure {
  oebpsPath: string;
  contentOpf: string;
  tocNcx?: string;
  navXhtml?: string;
  chapters: Chapter[];
  styles: string[];
  images: string[];
  metadata?: EpubMetadata;
}

export interface EpubFile {
  id: string;
  name: string;
  path: string;
  structure?: EpubStructure;
  structureError?: string;
  loadedAt: number;
  loaded_at?: number;
  refactoredStructure?: RefactoredEpubStructure;
  epubId?: string;
}

export interface ReaderState {
  currentChapterIndex: number;
  currentChapterPath: string | null;
  scrollPosition: number;
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  viewingImagePath: string | null;
  viewingImageData: string | null;
  pendingAnchor: string | null;
}

// 导出 CombinedChapter 类型供组件使用
export type CombinedChapter = Chapter | StandardChapter;

// 后端返回的数据结构（snake_case）
interface BackendEpubStructure {
  oebps_path: string;
  content_opf: string;
  toc_ncx?: string;
  nav_xhtml?: string;
  chapters: Chapter[];
  styles: string[];
  images: string[];
  metadata?: EpubMetadata;
}

// 重构系统相关接口
export interface RefactoredEpubStructure {
  epubId: string;
  metadata: EpubMetadata;
  structure: StandardEpubStructure;
  storagePath: string;
}

export interface StandardEpubStructure {
  chapters: StandardChapter[];
  styles: string[];
  images: string[];
  fonts: string[];
  navigation: NavigationEntry[];
}

export interface StandardChapter {
  id: string;
  original_filename: string;
  standard_path: string;
  title?: string;
  order: number;
  content?: ChapterContent;
}

export interface NavigationEntry {
  id: string;
  label: string;
  content_src: string;
  level: number;
  children: NavigationEntry[];
}

interface BackendRefactoredEpubResult {
  epub_id: string;
  metadata: EpubMetadata;
  structure: StandardEpubStructure;
  storage_path: string;
}

interface EpubStore {
  epubs: EpubFile[];
  selectedEpubId: string | null;
  readerState: ReaderState;

  addEpub: (file: EpubFile) => void;
  removeEpub: (id: string) => void;
  selectEpub: (id: string) => void;
  updateEpubStructure: (id: string, structure: EpubStructure) => void;
  setStructureError: (id: string, error: string) => void;
  updateRefactoredStructure: (id: string, refactoredStructure: RefactoredEpubStructure) => void;
  clearRefactoredStructure: (id: string) => void;

  loadEpubStructure: (id: string) => Promise<void>;
  loadChapterContent: (chapterPath: string) => Promise<void>;
  loadImageContent: (imagePath: string) => Promise<void>;
  refactorEpub: (id: string) => Promise<void>;
  loadRefactoredChapter: (epubId: string, chapterPath: string) => Promise<void>;
  exportEpub: (epubId: string) => Promise<string>;
  setReaderState: (state: Partial<ReaderState>) => void;
  navigateChapter: (direction: 'prev' | 'next') => void;
}

const defaultReaderState: ReaderState = {
  currentChapterIndex: 0,
  currentChapterPath: null,
  scrollPosition: 0,
  fontSize: 16,
  fontFamily: '"Georgia", serif',
  lineHeight: 1.6,
  viewingImagePath: null,
  viewingImageData: null,
  pendingAnchor: null,
};

export const useEpubStore = create<EpubStore>((set, get) => ({
  epubs: [],
  selectedEpubId: null,
  readerState: defaultReaderState,

  addEpub: (file) =>
    set((state) => {
      const normalizedFile = {
        ...file,
        loadedAt: file.loadedAt ?? file.loaded_at ?? Date.now(),
      };
      return {
        epubs: [...state.epubs, normalizedFile],
      };
    }),

  removeEpub: (id) =>
    set((state) => ({
      epubs: state.epubs.filter((epub) => epub.id !== id),
      selectedEpubId: state.selectedEpubId === id ? null : state.selectedEpubId,
    })),

  selectEpub: (id) =>
    set(() => ({
      selectedEpubId: id,
      readerState: defaultReaderState,
    })),

  updateEpubStructure: (id, structure) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id ? { ...epub, structure, structureError: undefined } : epub
      ),
    })),

  setStructureError: (id, error) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id ? { ...epub, structureError: error } : epub
      ),
    })),

  updateRefactoredStructure: (id, refactoredStructure) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id
          ? {
              ...epub,
              refactoredStructure,
              epubId: refactoredStructure.epubId,
              structureError: undefined,
            }
          : epub
      ),
    })),

  clearRefactoredStructure: (id) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id
          ? {
              ...epub,
              refactoredStructure: undefined,
              epubId: undefined,
            }
          : epub
      ),
    })),

  loadEpubStructure: async (id: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === id);
    if (!epub) return;

    // 清除之前的错误和结构
    get().setStructureError(id, '');

    try {
      const backendStructure = await invoke<BackendEpubStructure>('parse_epub_structure_command', {
        epubPath: epub.path,
      });

      // 转换后端返回的数据结构为前端格式
      const normalizedStructure: EpubStructure = {
        oebpsPath: backendStructure.oebps_path,
        contentOpf: backendStructure.content_opf,
        tocNcx: backendStructure.toc_ncx,
        navXhtml: backendStructure.nav_xhtml,
        chapters: backendStructure.chapters,
        styles: backendStructure.styles,
        images: backendStructure.images,
        metadata: backendStructure.metadata,
      };

      get().updateEpubStructure(id, normalizedStructure);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      get().setStructureError(id, errorMessage);
    }
  },

  loadChapterContent: async (chapterPath: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub) return;

    try {
      const content = await invoke<ChapterContent>('get_chapter_content_command', {
        epubPath: epub.path,
        chapterPath,
      });

      set((state) => ({
        epubs: state.epubs.map((e) => {
          if (e.id === epub.id) {
            return {
              ...e,
              structure: e.structure
                ? {
                    ...e.structure,
                    chapters: e.structure.chapters.map((ch) =>
                      ch.path === chapterPath ? { ...ch, content } : ch
                    ),
                  }
                : undefined,
            };
          }
          return e;
        }),
      }));
    } catch (error) {
      console.error('Failed to load chapter content:', error);
    }
  },

  loadImageContent: async (imagePath: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub) return;

    try {
      // Determine whether to use refactored or original structure
      if (epub.refactoredStructure?.epubId) {
        const imageData = await invoke<string>('get_image_from_refactored_command', {
          epubId: epub.refactoredStructure.epubId,
          imagePath,
        });
        get().setReaderState({
          viewingImagePath: imagePath,
          viewingImageData: imageData,
        });
      } else {
        const imageData = await invoke<string>('get_image_content_command', {
          epubPath: epub.path,
          imagePath,
        });
        get().setReaderState({
          viewingImagePath: imagePath,
          viewingImageData: imageData,
        });
      }
    } catch (error) {
      console.error('Failed to load image content:', error);
    }
  },

  setReaderState: (newState: Partial<ReaderState>) =>
    set((state) => ({
      readerState: { ...state.readerState, ...newState },
    })),

  navigateChapter: (direction: 'prev' | 'next') => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.structure?.chapters) return;

    const chapters = epub.structure.chapters;
    const currentIndex = state.readerState.currentChapterIndex;

    let targetIndex: number;
    if (direction === 'prev') {
      targetIndex = Math.max(0, currentIndex - 1);
    } else {
      targetIndex = Math.min(chapters.length - 1, currentIndex + 1);
    }

    if (targetIndex !== currentIndex) {
      const targetChapter = chapters[targetIndex];
      get().setReaderState({
        currentChapterIndex: targetIndex,
        currentChapterPath: targetChapter.path,
        scrollPosition: 0,
      });
    }
  },

  refactorEpub: async (id: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === id);
    if (!epub) return;

    try {
      const result = await invoke<BackendRefactoredEpubResult>('refactor_epub_command', {
        epubPath: epub.path,
      });

      const refactoredStructure: RefactoredEpubStructure = {
        epubId: result.epub_id,
        metadata: result.metadata,
        structure: result.structure,
        storagePath: result.storage_path,
      };

      get().updateRefactoredStructure(id, refactoredStructure);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      get().setStructureError(id, errorMessage);
    }
  },

  loadRefactoredChapter: async (epubId: string, chapterPath: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub) return;

    try {
      const content = await invoke<ChapterContent>('get_chapter_from_refactored_command', {
        epubId,
        chapterPath,
      });

      set((state) => ({
        epubs: state.epubs.map((e) => {
          if (e.id === epub.id) {
            return {
              ...e,
              refactoredStructure: e.refactoredStructure
                ? {
                    ...e.refactoredStructure,
                    structure: {
                      ...e.refactoredStructure.structure,
                      chapters: e.refactoredStructure.structure.chapters.map((ch) =>
                        ch.standard_path === chapterPath ? { ...ch, content } : ch
                      ),
                    },
                  }
                : undefined,
            };
          }
          return e;
        }),
      }));
    } catch (error) {
      console.error('Failed to load refactored chapter content:', error);
    }
  },

  exportEpub: async (epubId: string) => {
    const state = get();
    // 首先通过 epubId 查找，如果找不到则通过 id 查找
    let epub = state.epubs.find((e) => e.epubId === epubId);
    if (!epub) {
      epub = state.epubs.find((e) => e.id === epubId);
    }
    if (!epub) {
      throw new Error('EPUB not found');
    }

    // 如果 EPUB 还没有重构，先重构它
    if (!epub.refactoredStructure) {
      await get().refactorEpub(epub.id);
    }

    // 获取更新后的 EPUB 信息
    const updatedEpub = get().epubs.find((e) => e.id === epub.id);
    if (!updatedEpub?.refactoredStructure) {
      throw new Error('EPUB refactoring failed');
    }

    return updatedEpub.refactoredStructure.epubId;
  },
}));
