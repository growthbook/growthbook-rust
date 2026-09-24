/** Generate Rust fixtures using the sibling server's production payload builder. */
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const server = path.resolve(process.argv[2] || "../growthbook");
const backend = path.join(server, "packages/back-end");
const output = path.resolve(
  process.argv[3] || "tests/fixtures/saved_group_server_payloads.json",
);
const base = require(path.join(backend, "jest.config.js"));
const temp = fs.mkdtempSync(
  path.join(os.tmpdir(), "growthbook-rust-payloads-"),
);
const config = {
  ...base,
  rootDir: backend,
  roots: [__dirname],
  testMatch: ["**/saved_group_payloads.test.ts"],
  cacheDirectory: path.join(temp, "cache"),
  // The builder receives all data in memory; it does not need a database.
  globalSetup: undefined,
  globalTeardown: undefined,
  moduleNameMapper: {
    ...base.moduleNameMapper,
    "^back-end/(.*)$": path.join(backend, "$1"),
    "^shared/(.*)$": path.join(server, "packages/shared/src/$1"),
    "^shared$": path.join(server, "packages/shared/src/index.ts"),
    "^@growthbook/growthbook$": path.join(
      server,
      "packages/sdk-js/src/index.ts",
    ),
  },
};
const configFile = path.join(temp, "jest.config.json");
fs.writeFileSync(configFile, JSON.stringify(config));
try {
  const result = spawnSync(
    process.execPath,
    [
      path.join(backend, "node_modules/jest/bin/jest.js"),
      "--config",
      configFile,
      "--runInBand",
    ],
    {
      cwd: backend,
      stdio: "inherit",
      env: {
        ...process.env,
        GB_SERVER_REPO: server,
        GB_PAYLOAD_FIXTURE_OUTPUT: output,
      },
    },
  );
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
} finally {
  fs.rmSync(temp, { recursive: true, force: true });
}
