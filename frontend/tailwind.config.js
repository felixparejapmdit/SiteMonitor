/** Notion-aesthetic design tokens for SiteMonitor (PLAN.md Phase 1 §3). */
/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs", "./index.html", "../shared/src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        // Backgrounds
        page: "#ffffff",
        sidebar: "#f7f7f5",
        hover: "#f1f1ef",
        // Text — deep charcoal, never pure black
        ink: "#37352f",
        muted: "#787774",
        // Borders & dividers
        line: "#ededed",
        // Status pastel tags
        tag: {
          "green-bg": "#edf3ec",
          "green-text": "#346538",
          "amber-bg": "#fbf3db",
          "amber-text": "#8f6215",
          "red-bg": "#fdebec",
          "red-text": "#9e2a2b",
          "gray-bg": "#f1f1ef",
          "gray-text": "#5a5a57",
          "blue-bg": "#e7f0f8",
          "blue-text": "#2b5a85",
        },
        pin: { amber: "#d97706", green: "#346538" },
      },
      fontFamily: {
        sans: ["Inter", "-apple-system", "sans-serif"],
        serif: ["Lyon Text", "Georgia", "serif"],
      },
      borderColor: { DEFAULT: "#ededed" },
      boxShadow: {
        float: "0 1px 2px rgba(15,15,15,0.04), 0 8px 24px rgba(15,15,15,0.08)",
        pop: "0 0 0 1px rgba(15,15,15,0.05), 0 3px 6px rgba(15,15,15,0.1), 0 9px 24px rgba(15,15,15,0.2)",
      },
      maxWidth: { doc: "52rem" },
    },
  },
  plugins: [],
};
