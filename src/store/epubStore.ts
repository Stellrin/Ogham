import { create } from 'zustand';

export interface Chapter {
  id: string;
  path: string;
  name: string;
  order: number;
}

export interface EpubStructure {
  oebpsPath: string;
  contentOpf: string;
  tocNcx?: string;
  navXhtml?: string;
  chapters: Chapter[];
  styles: string[];
  images: string[];
}

export interface EpubFile {
  id: string;
  name: string;
  path: string;
  structure?: EpubStructure;
  loadedAt: number;
  loaded_at?: number;
}

interface EpubStore {
  epubs: EpubFile[];
  selectedEpubId: string | null;

  addEpub: (file: EpubFile) => void;
  removeEpub: (id: string) => void;
  selectEpub: (id: string) => void;
  updateEpubStructure: (id: string, structure: EpubStructure) => void;
}

export const useEpubStore = create<EpubStore>((set) => ({
  epubs: [],
  selectedEpubId: null,

  addEpub: (file) =>
    set((state) => {
      // 兼容后端的 snake_case 和前端的 camelCase
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
    })),

  updateEpubStructure: (id, structure) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id ? { ...epub, structure } : epub
      ),
    })),
}));
