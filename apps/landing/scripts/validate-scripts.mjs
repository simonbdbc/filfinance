// Le marqueur seul n'autorise rien : le script doit correspondre exactement à notre source.
export const validateScripts = (document, themeSource) => {
  const errors = [];
  const scripts = [...document.querySelectorAll("script")];
  const applicationScripts = scripts.filter((script) => script.getAttribute("type") !== "application/ld+json");
  if (applicationScripts.length !== 1) errors.push("Un seul script applicatif de thème est autorisé.");
  for (const script of applicationScripts) {
    const valid = script.id === "filfinance-theme"
      && script.parentElement?.tagName === "HEAD"
      && script.getAttributeNames().every((name) => name === "id")
      && script.textContent.trim() === themeSource.trim();
    if (!valid) errors.push("Un script applicatif inattendu ou modifié est présent dans le HTML.");
  }
  return errors;
};
