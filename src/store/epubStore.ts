import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

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

// 目录章节 - 支持嵌套层级
export interface TocChapter {
  id: string;
  label: string;
  contentSrc: string;
  filePath?: string;
  level: number;
  order: number;
  children: TocChapter[];
  isExpanded?: boolean;
}

// 视图模式
export type ViewMode = 'file' | 'toc';

interface BackendRefactoredEpubResult {
  epub_id: string;
  metadata: EpubMetadata;
  structure: StandardEpubStructure;
  storage_path: string;
}

interface ImageProcessFailure {
  chapter_path: string;
  image_url: string;
  error: string;
}

interface ImageProcessProgressEvent {
  task_id: string;
  chapter_path: string;
  current_chapter_index: number;
  total_chapters: number;
  image_url?: string | null;
  processed_unique_images: number;
  total_unique_images: number;
  successful_images: number;
  failed_images: number;
  skipped_duplicates: number;
  stage: string;
  message: string;
}

interface ImageProcessResult {
  task_id: string;
  total_chapters: number;
  processed_chapters: number;
  detected_raw_matches: number;
  detected_unique_urls: number;
  successful_images: number;
  failed_images: number;
  skipped_duplicates: number;
  inserted_images: number;
  failures: ImageProcessFailure[];
}

interface EpubStore {
  epubs: EpubFile[];
  selectedEpubId: string | null;
  readerState: ReaderState;

  // 目录管理相关状态
  viewMode: ViewMode;
  tocEntries: TocChapter[];
  expandedTocIds: Set<string>;

  // 简繁转换相关状态
  isConverting: boolean;
  conversionProgress: number;

  // 图片处理相关状态
  isProcessingImages: boolean;
  imageProcessingProgress: number;
  imageProcessingTotal: number;
  imageProcessingCurrentChapter: string;
  imageProcessingCurrentImageUrl: string;
  imageProcessingSuccess: number;
  imageProcessingFailed: number;
  imageProcessingSkipped: number;
  imageProcessingProcessedUnique: number;
  imageProcessingTaskId: string | null;
  imageProcessingFailures: ImageProcessFailure[];

  addEpub: (file: EpubFile) => void;
  removeEpub: (id: string) => void;
  selectEpub: (id: string) => void;
  updateEpubStructure: (id: string, structure: EpubStructure) => void;
  setStructureError: (id: string, error: string) => void;
  updateRefactoredStructure: (id: string, refactoredStructure: RefactoredEpubStructure) => void;
  clearRefactoredStructure: (id: string) => void;

  // 目录管理方法
  setViewMode: (mode: ViewMode) => void;
  loadTocEntries: (epubId: string) => Promise<void>;
  updateTocEntryLabel: (entryId: string, newLabel: string) => Promise<void>;
  updateTocEntryFile: (entryId: string, newContentSrc: string) => Promise<void>;
  reorderTocEntries: (newOrder: TocChapter[]) => Promise<void>;
  toggleTocExpanded: (entryId: string) => void;

  // 简繁转换方法
  convertSimplifiedTraditional: (mode: 's2t' | 't2s') => Promise<void>;
  setConverting: (isConverting: boolean) => void;
  clearChapterCache: (id: string) => void;

  // 图片链接处理方法
  processAllImages: () => Promise<void>;

  loadEpubStructure: (id: string) => Promise<void>;
  loadChapterContent: (chapterPath: string) => Promise<void>;
  loadImageContent: (imagePath: string) => Promise<void>;
  /** 重构 EPUB（解析原始文件，会生成新的 epub_id） */
  refactorEpub: (id: string) => Promise<void>;
  /** 从缓存重新加载（读取现有缓存，保持相同的 epub_id） */
  reloadEpubStructure: (id: string) => Promise<void>;
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
  viewMode: 'file',
  tocEntries: [],
  expandedTocIds: new Set<string>(),
  isConverting: false,
  conversionProgress: 0,
  isProcessingImages: false,
  imageProcessingProgress: 0,
  imageProcessingTotal: 0,
  imageProcessingCurrentChapter: '',
  imageProcessingCurrentImageUrl: '',
  imageProcessingSuccess: 0,
  imageProcessingFailed: 0,
  imageProcessingSkipped: 0,
  imageProcessingProcessedUnique: 0,
  imageProcessingTaskId: null,
  imageProcessingFailures: [],

  addEpub: (file) =>
    set((state) => {
      return {
        epubs: [...state.epubs, file],
      };
    }),

  removeEpub: (id) =>
    set((state) => ({
      epubs: state.epubs.filter((epub) => epub.id !== id),
      selectedEpubId: state.selectedEpubId === id ? null : state.selectedEpubId,
    })),

  selectEpub: async (id: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === id);

    set(() => ({
      selectedEpubId: id,
      readerState: defaultReaderState,
      viewMode: 'file',
    }));

    // 选择 EPUB 后自动重构
    if (epub && !epub.refactoredStructure) {
      await get().refactorEpub(id);
    }
  },

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

    // 优先使用重构后的结构，回退到原始结构
    const chapters = epub?.refactoredStructure?.structure.chapters || epub?.structure?.chapters;
    if (!chapters || chapters.length === 0) return;

    const currentPath = state.readerState.currentChapterPath;

    // 通过当前路径查找序号，避免 index 与 TOC order 不一致
    const foundIndex = chapters.findIndex((c) =>
      'standard_path' in c ? c.standard_path === currentPath : (c as Chapter).path === currentPath
    );
    const baseIndex = foundIndex >= 0 ? foundIndex : state.readerState.currentChapterIndex;

    let targetIndex: number;
    if (direction === 'prev') {
      targetIndex = Math.max(0, baseIndex - 1);
    } else {
      targetIndex = Math.min(chapters.length - 1, baseIndex + 1);
    }

    if (targetIndex !== baseIndex) {
      const targetChapter = chapters[targetIndex];
      const targetPath =
        'standard_path' in targetChapter ? targetChapter.standard_path : (targetChapter as Chapter).path;
      get().setReaderState({
        currentChapterIndex: targetIndex,
        currentChapterPath: targetPath,
        scrollPosition: 0,
      });
    }
  },

  /**
   * 重构 EPUB 文件
   *
   * ⚠️ 重要说明：
   * 此方法会重新解析**原始 EPUB 文件**，而不是读取缓存目录。
   * 适用于：首次加载 EPUB 或需要从原始文件重新生成标准结构的情况。
   *
   * 与 reloadEpubStructure 的区别：
   * - refactorEpub：解析原始 EPUB 文件，生成新的缓存和 epub_id
   * - reloadEpubStructure：读取现有缓存，使用相同的 epub_id
   */
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

      // 加载目录数据
      await get().loadTocEntries(result.epub_id);
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      get().setStructureError(id, errorMessage);
    }
  },

  /**
   * 从缓存目录重新加载 EPUB 结构
   *
   * ⚠️ 重要说明：
   * 此方法从**缓存目录**读取当前的文件结构，而不是重新解析原始 EPUB 文件。
   * 适用于：简繁转换、图片处理等操作后，需要刷新前端视图的情况。
   *
   * 与 refactorEpub 的区别：
   * - refactorEpub：解析原始 EPUB 文件，生成新的缓存和 epub_id
   * - reloadEpubStructure：读取现有缓存，使用相同的 epub_id
   */
  reloadEpubStructure: async (id: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === id);
    if (!epub?.refactoredStructure) return;

    try {
      const result = await invoke<BackendRefactoredEpubResult>('reload_epub_structure_command', {
        epubId: epub.refactoredStructure.epubId,
      });

      const refactoredStructure: RefactoredEpubStructure = {
        epubId: result.epub_id,
        metadata: result.metadata,
        structure: result.structure,
        storagePath: result.storage_path,
      };

      get().updateRefactoredStructure(id, refactoredStructure);

      // 重新加载目录数据
      await get().loadTocEntries(result.epub_id);
    } catch (error) {
      console.error('Failed to reload EPUB structure:', error);
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

  // 目录管理方法实现
  setViewMode: async (mode: ViewMode) => {
    // 切换到目录视图时，如果 EPUB 尚未重构则自动重构
    if (mode === 'toc') {
      const state = get();
      const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
      if (epub && !epub.refactoredStructure) {
        await get().refactorEpub(epub.id);
      }
    }
    set({ viewMode: mode });
  },

  loadTocEntries: async (epubId: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.epubId === epubId || e.id === epubId);
    if (!epub?.refactoredStructure) return;

    try {
      const entries = await invoke<TocChapter[]>('load_toc_entries_command', {
        epubId,
      });
      set({ tocEntries: entries });
    } catch (error) {
      console.error('Failed to load TOC entries:', error);
    }
  },

  updateTocEntryLabel: async (entryId: string, newLabel: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) return;

    try {
      await invoke('update_toc_entry_command', {
        epubId: epub.refactoredStructure.epubId,
        entryId,
        newLabel,
        newContentSrc: null,
      });

      // 更新本地状态
      const updateLabel = (entries: TocChapter[]): TocChapter[] => {
        return entries.map((entry) => {
          if (entry.id === entryId) {
            return { ...entry, label: newLabel };
          }
          if (entry.children.length > 0) {
            return { ...entry, children: updateLabel(entry.children) };
          }
          return entry;
        });
      };

      set((state) => ({
        tocEntries: updateLabel(state.tocEntries),
      }));
    } catch (error) {
      console.error('Failed to update TOC entry label:', error);
    }
  },

  updateTocEntryFile: async (entryId: string, newContentSrc: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) return;

    try {
      await invoke('update_toc_entry_command', {
        epubId: epub.refactoredStructure.epubId,
        entryId,
        newLabel: null,
        newContentSrc,
      });

      // 更新本地状态
      const updateFile = (entries: TocChapter[]): TocChapter[] => {
        return entries.map((entry) => {
          if (entry.id === entryId) {
            return { ...entry, contentSrc: newContentSrc };
          }
          if (entry.children.length > 0) {
            return { ...entry, children: updateFile(entry.children) };
          }
          return entry;
        });
      };

      set((state) => ({
        tocEntries: updateFile(state.tocEntries),
      }));
    } catch (error) {
      console.error('Failed to update TOC entry file:', error);
    }
  },

  reorderTocEntries: async (newOrder: TocChapter[]) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) return;

    try {
      await invoke('update_toc_order_command', {
        epubId: epub.refactoredStructure.epubId,
        newOrder,
      });

      set({ tocEntries: newOrder });
    } catch (error) {
      console.error('Failed to reorder TOC entries:', error);
    }
  },

  toggleTocExpanded: (entryId: string) =>
    set((state) => {
      const newExpanded = new Set(state.expandedTocIds);
      if (newExpanded.has(entryId)) {
        newExpanded.delete(entryId);
      } else {
        newExpanded.add(entryId);
      }
      return { expandedTocIds: newExpanded };
    }),

  setConverting: (isConverting: boolean) =>
    set({ isConverting }),

  clearChapterCache: (id: string) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id && epub.refactoredStructure
          ? {
              ...epub,
              refactoredStructure: {
                ...epub.refactoredStructure,
                structure: {
                  ...epub.refactoredStructure.structure,
                  chapters: epub.refactoredStructure.structure.chapters.map((ch) => ({
                    ...ch,
                    content: undefined,
                  })),
                },
              },
            }
          : epub
      ),
    })),

  convertSimplifiedTraditional: async (mode: 's2t' | 't2s') => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) {
      throw new Error('EPUB not found or not refactored');
    }

    const epubId = epub.refactoredStructure.epubId;
    let progressInterval: ReturnType<typeof setInterval> | null = null;

    try {
      set({ isConverting: true, conversionProgress: 0 });

      // 模拟进度更新
      progressInterval = setInterval(() => {
        set((state) => ({
          conversionProgress: Math.min(state.conversionProgress + 10, 90),
        }));
      }, 100);

      const result = await invoke<{ success: boolean; files_converted: number; error: string | null }>(
        'convert_simplified_traditional_command',
        {
          epubId,
          mode,
        }
      );

      if (progressInterval) {
        clearInterval(progressInterval);
        progressInterval = null;
      }

      if (!result.success) {
        throw new Error(result.error || 'Conversion failed');
      }

      // 显示进度 100%
      set({ conversionProgress: 100 });

      // 清除章节缓存，确保下次加载获取转换后的最新内容
      get().clearChapterCache(epub.id);

      // 先清理状态，避免阻塞 UI 更新
      set({ isConverting: false, conversionProgress: 0 });

      // 重新加载目录数据以确保前端显示最新改动（在状态清理后调用）
      await get().loadTocEntries(epubId);

      // 重新加载 EPUB 结构以更新文件视图（直接从缓存读取）
      await get().reloadEpubStructure(epub.id);

      // 使用 setTimeout 确保状态更新后再显示 alert
      setTimeout(() => {
        alert(`转换完成！共转换 ${result.files_converted} 个文件`);
      }, 0);
    } catch (error) {
      // 清理状态
      if (progressInterval) {
        clearInterval(progressInterval);
      }
      set({ isConverting: false, conversionProgress: 0 });

      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error('Conversion failed:', errorMessage);
      alert(`转换失败: ${errorMessage}`);
      throw error;
    }
  },

  // 处理整本 EPUB 中所有章节的图片链接
  processAllImages: async () => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) {
      throw new Error('EPUB not found or not refactored');
    }

    const epubId = epub.refactoredStructure.epubId;
    let unlisten: (() => void) | null = null;

    try {
      set({
        isProcessingImages: true,
        imageProcessingProgress: 0,
        imageProcessingTotal: 0,
        imageProcessingCurrentChapter: '',
        imageProcessingCurrentImageUrl: '',
        imageProcessingSuccess: 0,
        imageProcessingFailed: 0,
        imageProcessingSkipped: 0,
        imageProcessingProcessedUnique: 0,
        imageProcessingTaskId: null,
        imageProcessingFailures: [],
      });

      unlisten = await listen<ImageProcessProgressEvent>('image_process_progress', (event) => {
        const payload = event.payload;

        set((currentState) => {
          const existingTaskId = currentState.imageProcessingTaskId;
          if (existingTaskId && existingTaskId !== payload.task_id) {
            return currentState;
          }

          const total = payload.total_unique_images || 0;
          const processed = payload.processed_unique_images || 0;
          const progress = total > 0 ? Math.round((processed / total) * 100) : 0;

          return {
            imageProcessingTaskId: payload.task_id,
            imageProcessingTotal: total,
            imageProcessingProcessedUnique: processed,
            imageProcessingProgress: Math.min(progress, 100),
            imageProcessingCurrentChapter: payload.chapter_path || '',
            imageProcessingCurrentImageUrl: payload.image_url || '',
            imageProcessingSuccess: payload.successful_images || 0,
            imageProcessingFailed: payload.failed_images || 0,
            imageProcessingSkipped: payload.skipped_duplicates || 0,
          };
        });
      });

      const result = await invoke<ImageProcessResult>('process_all_images_command', {
        epubId,
      });

      if (unlisten) {
        unlisten();
        unlisten = null;
      }

      set({
        imageProcessingTaskId: result.task_id,
        imageProcessingTotal: result.detected_unique_urls,
        imageProcessingProcessedUnique: result.detected_unique_urls,
        imageProcessingSuccess: result.successful_images,
        imageProcessingFailed: result.failed_images,
        imageProcessingSkipped: result.skipped_duplicates,
        imageProcessingFailures: result.failures,
        imageProcessingProgress: 100,
      });

      // 清除章节缓存，确保下次加载获取最新内容
      get().clearChapterCache(epub.id);

      // 重新加载目录数据以确保前端显示最新改动
      await get().loadTocEntries(epubId);

      // 重新加载 EPUB 结构以更新文件视图（包括新增图片，直接从缓存读取）
      await get().reloadEpubStructure(epub.id);

      // 先清理状态，避免阻塞 UI 更新
      set({
        isProcessingImages: false,
        imageProcessingProgress: 0,
        imageProcessingCurrentChapter: '',
        imageProcessingCurrentImageUrl: '',
      });

      // 使用 setTimeout 确保状态更新后再显示 alert
      setTimeout(() => {
        const summary = `处理完成！章节 ${result.processed_chapters}/${result.total_chapters}，` +
          `匹配 ${result.detected_raw_matches} 条，去重后 ${result.detected_unique_urls} 张，` +
          `成功 ${result.successful_images}，失败 ${result.failed_images}，复用 ${result.skipped_duplicates}`;

        if (result.failed_images > 0 && result.failures.length > 0) {
          const details = result.failures
            .slice(0, 5)
            .map((failure, index) => `${index + 1}. ${failure.chapter_path} | ${failure.image_url} | ${failure.error}`)
            .join('\n');
          alert(`${summary}\n\n失败明细（最多显示5条）:\n${details}`);
          return;
        }

        alert(summary);
      }, 0);
    } catch (error) {
      // 清理状态
      if (unlisten) {
        unlisten();
      }
      set({
        isProcessingImages: false,
        imageProcessingProgress: 0,
        imageProcessingCurrentChapter: '',
        imageProcessingCurrentImageUrl: '',
      });

      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error('Failed to process all images:', errorMessage);
      alert(`处理图片失败: ${errorMessage}`);
      throw error;
    }
  },
}));
