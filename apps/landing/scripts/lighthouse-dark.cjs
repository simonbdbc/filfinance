// Le navigateur d'audit reçoit la même préférence qu'un visiteur, sans paramètre public.
module.exports = async (browser, { url }) => {
  const page = await browser.newPage();
  await page.goto(url);
  await page.evaluate(() => localStorage.setItem("filfinance.landing.theme", "dark"));
  await page.reload();
  if (await page.$eval("html", (root) => root.dataset.theme) !== "dark") {
    throw new Error("La préparation du thème sombre a échoué.");
  }
  const session = await page.createCDPSession();
  await session.send("Network.clearBrowserCache");
  await session.detach();
  await page.close();
};
