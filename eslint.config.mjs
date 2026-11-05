import js from "@eslint/js";
export default [
  js.configs.recommended,
  { ignores: ["site/artifacts/**", "site-dist/**", "test-results/**"] },
  {
    files: ["**/*.mjs"],
    languageOptions: {
      globals: {
        process: "readonly",
        fetch: "readonly",
        AbortSignal: "readonly",
        document: "readonly",
        window: "readonly",
      },
    },
  },
];
