/** SiteMonitor design tokens. Colours are CSS variables (styles/input.css) so the
 *  look can be retuned in one place. */
/** @type {import('tailwindcss').Config} */
const v = (name) => `rgb(var(--c-${name}) / <alpha-value>)`;
module.exports = {
  content: ["./src/**/*.rs", "./index.html", "../shared/src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        page: v("page"),
        surface: v("surface"),
        sidebar: v("sidebar"),
        hover: v("hover"),
        ink: v("ink"),
        muted: v("muted"),
        line: v("line"),
        // Signature emerald: `accent` fills buttons/active states, `accent-ink` is
        // the readable text/stroke tone of it on the current background.
        accent: { DEFAULT: v("accent"), ink: v("accent-ink"), soft: v("accent-soft") },
        tag: {
          "green-bg": v("green-bg"),
          "green-text": v("green-text"),
          "amber-bg": v("amber-bg"),
          "amber-text": v("amber-text"),
          "red-bg": v("red-bg"),
          "red-text": v("red-text"),
          "gray-bg": v("gray-bg"),
          "gray-text": v("gray-text"),
          "blue-bg": v("blue-bg"),
          "blue-text": v("blue-text"),
        },
        pin: { amber: "#d97706", green: "#346538" },
      },
      fontFamily: {
        sans: ["Inter", "-apple-system", "sans-serif"],
        serif: ["Lyon Text", "Georgia", "serif"],
      },
      borderColor: { DEFAULT: v("line") },
      boxShadow: {
        float: "0 1px 2px rgba(15,15,15,0.04), 0 8px 24px rgba(15,15,15,0.08)",
        pop: "0 0 0 1px rgba(15,15,15,0.05), 0 3px 6px rgba(15,15,15,0.1), 0 9px 24px rgba(15,15,15,0.2)",
      },
      maxWidth: { doc: "52rem" },
    },
  },
  plugins: [],
};
