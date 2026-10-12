module.exports = {
  ci: {
    collect: {
      numberOfRuns: 3,
      // Un processus séparé évite l'attente de server.close() sur les connexions Chromium en CI.
      startServerCommand: "npm run preview -- --host 127.0.0.1 --port 4179",
      startServerReadyPattern: "http://127\\.0\\.0\\.1:4179/",
      url: ["http://127.0.0.1:4179/"],
      settings: {
        chromeFlags: "--headless --no-sandbox --disable-dev-shm-usage",
        preset: "desktop",
      },
    },
    assert: {
      assertions: {
        "categories:performance": ["error", { minScore: 0.80, aggregationMethod: "pessimistic" }],
        "categories:accessibility": ["error", { minScore: 1, aggregationMethod: "pessimistic" }],
        "categories:best-practices": ["error", { minScore: 1, aggregationMethod: "pessimistic" }],
        "categories:seo": ["error", { minScore: 1, aggregationMethod: "pessimistic" }],
      },
    },
    upload: {
      target: "filesystem",
      outputDir: "./reports/lighthouse-desktop",
    },
  },
};
