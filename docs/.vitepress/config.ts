import { defineConfig } from "vitepress";

export default defineConfig({
  title: "Fresh",
  description:
    "Fresh is a fast, modern terminal text editor with intuitive keybindings, syntax highlighting, and instant startup.",
  base: "/fresh/docs/",
  srcDir: ".",
  outDir: "../dist/docs",

  head: [["link", { rel: "icon", href: "/fresh/docs/logo.svg" }]],

  cleanUrls: true,
  lastUpdated: true,
  appearance: "force-dark",
  themeConfig: {
    logo: { light: "/logo.svg", dark: "/logo.svg" },

    nav: [
      { text: "Homepage", link: "https://radiorambo.github.io/fresh" },
      { text: "Getting Started", link: "/index" },
      { text: "Download", link: "https://github.com/sinelaw/fresh/releases/latest" },
      {
        text: "Issues & Requests",
        link: "https://github.com/sinelaw/fresh/issues",
      },
    ],

    sidebar: [
      {
        items: [{ text: "Getting Started", link: "/index" }],
      },
      {
        text: "User Guide",
        collapsed: false,
        items: [
          { text: "Introduction", link: "/guide/" },
          { text: "Editing & Navigation", link: "/guide/editing" },
          { text: "Terminal", link: "/guide/terminal" },
          { text: "LSP Integration", link: "/guide/lsp" },
          { text: "Plugins", link: "/guide/plugins" },
          { text: "Themes", link: "/guide/themes" },
          { text: "Configuration", link: "/guide/configuration" },
          { text: "Keyboard Setup", link: "/guide/keyboard" },
          { text: "Internationalization", link: "/guide/i18n" },
          { text: "Troubleshooting", link: "/guide/troubleshooting" },
          { text: "Keybindings", link: "/guide/keybindings" },
        ],
      },
      {
        text: "Features",
        collapsed: false,
        items: [
          { text: "Terminal", link: "/features/terminal" },
          { text: "Vi Mode", link: "/features/vi-mode" },
        ],
      },
      {
        text: "Development",
        collapsed: false,
        items: [
          { text: "Architecture", link: "/development/architecture" },
          { text: "Plugin API", link: "/development/plugin-api" },
          { text: "Plugin Development", link: "/development/plugin-development" },
          { text: "QuickJS Migration", link: "/quickjs-migration" },
          { text: "WASM Compatibility", link: "/WASM_COMPATIBILITY_ANALYSIS" },
        ],
      },
      {
        text: "Design Documents",
        collapsed: true,
        items: [
          { text: "Config Editor", link: "/design/config-editor" },
          { text: "Paste Handling", link: "/design/paste-handling" },
          { text: "Scroll Sync", link: "/design/scroll-sync" },
          { text: "Unicode Width", link: "/design/unicode-width" },
          { text: "Visual Layout", link: "/design/visual-layout" },
          { text: "Internationalization", link: "/design/i18n" },
          { text: "Search Next Occurrence", link: "/design/search-next-occurrence" },
          { text: "Finder Abstraction", link: "/design/finder-abstraction" },
        ],
      },
      {
        text: "Internal",
        collapsed: true,
        items: [
          { text: "Overview", link: "/internal/" },
          { text: "Plugin Architecture", link: "/internal/PLUGIN_ARCHITECTURE_PLAN" },
          { text: "Event Dispatch", link: "/internal/EVENT_DISPATCH_ARCHITECTURE" },
          { text: "I/O Separation", link: "/internal/IO_SEPARATION_PLAN" },
          { text: "Plugin Usability", link: "/internal/PLUGIN_USABILITY_REVIEW" },
          { text: "Settings Indicator", link: "/internal/SETTINGS_MODIFIED_INDICATOR_DESIGN" },
          { text: "Theme Consolidation", link: "/internal/theme-consolidation-plan" },
          { text: "Config Implementation", link: "/internal/CONFIG_IMPLEMENTATION_PLAN" },
          { text: "Config Design", link: "/internal/CONFIG_DESIGN" },
          { text: "Input Calibration", link: "/internal/INPUT_CALIBRATION_WIZARD" },
          { text: "Diff View", link: "/internal/DIFF_VIEW" },
          { text: "Theme Usability", link: "/internal/theme-usability-improvements" },
          { text: "Bulk Edit Optimization", link: "/internal/bulk-edit-optimization" },
          { text: "Warning Notifications", link: "/internal/WARNING_NOTIFICATION_UX" },
          { text: "Theme User Flows", link: "/internal/theme-user-flows" },
          { text: "Code Quality", link: "/internal/CR" },
          { text: "Markdown Mode", link: "/internal/MARKDOWN" },
          { text: "TimeSource Design", link: "/internal/TIMESOURCE_DESIGN" },
        ],
      },
    ],

    outline: { level: "deep" },

    socialLinks: [{ icon: "github", link: "https://github.com/sinelaw/fresh" }],

    search: { provider: "local" },

    editLink: {
      pattern: "https://github.com/sinelaw/fresh/edit/master/docs/:path",
    },

    footer: {
      message: "Released under the Apache 2.0 License",
    },
  },
});
