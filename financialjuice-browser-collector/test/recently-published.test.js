import assert from "node:assert/strict";
import test from "node:test";

import { RecentlyPublishedIds } from "../src/recently-published.js";

test("RecentlyPublishedIds remembers ids only when explicitly requested", () => {
  const recent = new RecentlyPublishedIds(2);
  const event = { external_id: "headline-1" };

  assert.equal(recent.has(event.external_id), false);
  assert.equal(recent.has(event.external_id), false);

  recent.rememberAll([event]);
  assert.equal(recent.has(event.external_id), true);
});

test("RecentlyPublishedIds evicts oldest ids when limit is reached", () => {
  const recent = new RecentlyPublishedIds(2);

  recent.remember("headline-1");
  recent.remember("headline-2");
  recent.remember("headline-3");

  assert.equal(recent.has("headline-1"), false);
  assert.equal(recent.has("headline-2"), true);
  assert.equal(recent.has("headline-3"), true);
});
