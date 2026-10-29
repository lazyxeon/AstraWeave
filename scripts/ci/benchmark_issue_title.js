// Titles for benchmark-dashboard.yml. The checker prefixes each
// Write-Error line with "❌ ", so a ^-anchored name match never sees it.

function parseBenchmarkReport(report) {
  const over = [];
  const missing = [];
  const text = report == null ? "" : String(report);
  for (const raw of text.split(/\r?\n/)) {
    const line = raw
      .replace(/^(?:Write-Error|Exception|NativeCommandError):\s*/, "")
      .replace(/^\u274C\s*/u, "");
    const failed = line.match(/^([A-Za-z0-9_./:-]+): .+OVER LIMIT/);
    if (failed) over.push(failed[1]);
    const absent = line.match(/No benchmark result for threshold '([^']+)'/);
    if (absent) missing.push(absent[1]);
  }
  return { over, missing, report: text };
}

function issueTitle(parsed, sha) {
  const { over, missing, report } = parsed;
  if (over.length > 0) {
    const shown = over.slice(0, 4).join(", ");
    const more = over.length > 4 ? ` +${over.length - 4} more` : "";
    return `Benchmark regression: ${shown}${more}`.slice(0, 240);
  }
  if (missing.length > 0) return "Benchmark results missing threshold keys";
  if (report.includes("Benchmark results not found")) return "Benchmark results missing";
  const short = String(sha == null ? "" : sha).slice(0, 7);
  return `Benchmark check failed ${short}`;
}

function issueBullets(parsed) {
  const lines = [];
  for (const name of parsed.over) lines.push(`- ${name}`);
  for (const name of parsed.missing) lines.push("- missing `" + name + "`");
  return lines.join("\n");
}

module.exports = { parseBenchmarkReport, issueTitle, issueBullets };
