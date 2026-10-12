const test = require("node:test"),
  assert = require("node:assert/strict"),
  fs = require("node:fs"),
  path = require("node:path"),
  vm = require("node:vm"),
  ts = require("typescript");
// Exécuter les classes TS existantes, sans recopier leur algorithme dans le test.
function load(file) {
  const filename = path.resolve(
    __dirname,
    "../src/features/admin/lib/rustLogger",
    file,
  );
  const exports = {};
  const code = ts.transpileModule(fs.readFileSync(filename, "utf8"), {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2022,
    },
  }).outputText;
  vm.runInNewContext(code, {
    exports,
    require: (n) => load(n + ".ts"),
    Uint8Array,
    TextDecoder,
    atob,
    Error,
    Number,
    BigInt,
  });
  return exports;
}
const { ByteWindow } = load("window.ts");
const { LIMITS, offset } = load("protocol.ts");
const chunk = (text, start = 0, generation = "1") => ({
  data: Buffer.from(text).toString("base64"),
  start: String(start),
  end: String(start + Buffer.byteLength(text)),
  size: String(start + Buffer.byteLength(text)),
  generation,
});
test("fenêtre de logs bornée en octets et lignes", () => {
  const w = new ByteWindow(16, 3);
  w.replace(chunk("a\nb\nc\n"));
  for (let i = 0; i < 100; i++) w.append(chunk("abcdefgh\n", Number(w.end)));
  assert(w.bytes.length <= 16);
  assert(w.text().split("\n").length <= 4);
  assert(w.evicted);
  assert.equal(LIMITS.viewBytes, 1024 * 1024);
  assert.equal(LIMITS.viewLines, 20000);
});
test("refus des ruptures et changements de génération", () => {
  const w = new ByteWindow();
  w.replace(chunk("abc"));
  assert.throws(() => w.append(chunk("def", 4)), /Discontinuité/);
  assert.throws(() => w.append(chunk("def", 3, "2")), /Discontinuité/);
});
test("UTF-8 incomplet conservé jusqu’au fragment suivant", () => {
  const w = new ByteWindow();
  const bytes = Buffer.from("é");
  w.replace({
    ...chunk("", 0),
    data: bytes.subarray(0, 1).toString("base64"),
    end: "1",
    size: "2",
  });
  assert.equal(w.text(), "");
  w.append({
    ...chunk("", 1),
    data: bytes.subarray(1).toString("base64"),
    end: "2",
    size: "2",
  });
  assert.equal(w.text(), "é");
  assert.equal(w.invalidUtf8(), false);
});
test("positions dangereuses refusées", () => {
  assert.throws(() => offset("9007199254740992"));
  assert.throws(() => offset("-1"));
});
