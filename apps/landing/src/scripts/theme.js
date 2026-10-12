// Ce script unique s'exécute dans le head avant le contenu pour éviter un flash de thème.
(() => {
  const root = document.documentElement;
  const meta = document.querySelector('meta[name="theme-color"]');
  const storageKey = "filfinance.landing.theme";
  let theme = "light";

  // Une préférence absente, invalide ou inaccessible conserve le clair, sans consulter le système.
  try {
    if (window.localStorage.getItem(storageKey) === "dark") theme = "dark";
  } catch {
    // Le navigateur peut refuser le stockage ; la page reste utilisable.
  }

  const applyTheme = () => {
    root.dataset.theme = theme;
    if (meta) meta.setAttribute("content", meta.getAttribute(`data-${theme}`));
  };
  applyTheme();

  // La commande n'est révélée qu'une fois son comportement installé.
  const connectToggle = () => {
    const button = document.getElementById("theme-toggle");
    if (!button) return;
    const updateButton = () => button.setAttribute("aria-pressed", String(theme === "dark"));
    updateButton();
    button.addEventListener("click", () => {
      theme = theme === "dark" ? "light" : "dark";
      applyTheme();
      updateButton();
      try {
        window.localStorage.setItem(storageKey, theme);
      } catch {
        // L'échec de persistance n'annule pas le choix pour la visite courante.
      }
    });
    button.hidden = false;
  };
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", connectToggle, { once: true });
  } else {
    connectToggle();
  }
})();
