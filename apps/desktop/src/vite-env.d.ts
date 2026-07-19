/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_SPM_API_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
