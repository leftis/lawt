/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL?: string;
  /** Legacy photo-bucket CDN host. Not used for timeline renditions. */
  readonly VITE_CDN_HOST?: string;
  /** Base URL that serves `derived/{id}/{rendition}.jpeg` for the timeline. Defaults to the API base. */
  readonly VITE_RENDITION_BASE_URL?: string;
  readonly VITE_COGNITO_USER_POOL_ID?: string;
  readonly VITE_COGNITO_USER_POOL_CLIENT_ID?: string;
  readonly VITE_COGNITO_REGION?: string;
  readonly VITE_COGNITO_HOSTED_UI_DOMAIN?: string;
  readonly VITE_COGNITO_LOGIN_REDIRECT?: string;
  readonly VITE_COGNITO_LOGOUT_REDIRECT?: string;
  /** `legacy` makes the classic gallery the default UI; otherwise the timeline is the default. */
  readonly VITE_UI?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
