const assert = require("node:assert/strict");
const test = require("node:test");
const {
  parseBenchmarkReport,
  issueTitle,
  issueBullets,
} = require("./benchmark_issue_title");

const overLine =
  "❌ behavior_tree_20_nodes: 600.00 ns (❌ FAIL, 8.2% OVER LIMIT)";
const prefixed =
  "Write-Error: ❌ astraweave-physics::raycast_batch_8_rays: 4.16 µs (❌ FAIL, 20.0% OVER LIMIT)";
const missingLine =
  "❌ No benchmark result for threshold 'binding_lookup'";

test("a leading ❌ does not hide an over-limit name", () => {
  const anchored = /^([A-Za-z0-9_./:-]+): .+OVER LIMIT/;
  assert.equal(anchored.test(overLine), false);
  const parsed = parseBenchmarkReport(overLine + "\n");
  assert.deepEqual(parsed.over, ["behavior_tree_20_nodes"]);
  assert.equal(
    issueTitle(parsed, "d510cd7e31db7953b0131863465df632fc6de3b1"),
    "Benchmark regression: behavior_tree_20_nodes",
  );
});

test("Write-Error and a package:: name still title the bench", () => {
  const parsed = parseBenchmarkReport(prefixed);
  assert.deepEqual(parsed.over, ["astraweave-physics::raycast_batch_8_rays"]);
  assert.equal(
    issueTitle(parsed, "d510cd7e"),
    "Benchmark regression: astraweave-physics::raycast_batch_8_rays",
  );
});

test("a missing key keeps its title, including the marker", () => {
  const parsed = parseBenchmarkReport(missingLine);
  assert.deepEqual(parsed.missing, ["binding_lookup"]);
  assert.equal(issueTitle(parsed, "abc"), "Benchmark results missing threshold keys");
});

test("over-limit and missing keys are both listed", () => {
  const parsed = parseBenchmarkReport(`${overLine}\n${missingLine}\n`);
  assert.equal(
    issueTitle(parsed, "d510cd7e"),
    "Benchmark regression: behavior_tree_20_nodes",
  );
  assert.equal(
    issueBullets(parsed),
    "- behavior_tree_20_nodes\n- missing `binding_lookup`",
  );
});

test("a missing history file and an unknown failure keep their titles", () => {
  const missingFile = parseBenchmarkReport("Benchmark results not found: hist\n");
  assert.equal(issueTitle(missingFile, "d510cd7e"), "Benchmark results missing");
  const unknown = parseBenchmarkReport("something else\n");
  assert.equal(issueTitle(unknown, "d510cd7e31db"), "Benchmark check failed d510cd7");
});
