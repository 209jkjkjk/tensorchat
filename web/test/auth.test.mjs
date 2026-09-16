// Tests for the client's authentication form helpers.

import { test } from "node:test";
import assert from "node:assert/strict";
import { passwordsMatch } from "../src/ui/login.ts";

test("accepts matching passwords", () => {
  assert.equal(passwordsMatch("correct horse", "correct horse"), true);
});

test("rejects different passwords", () => {
  assert.equal(passwordsMatch("correct horse", "correct horses"), false);
});
