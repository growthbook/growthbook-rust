/** Generate payloads from production server code, with the JS SDK as oracle. */
import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { webcrypto } from "node:crypto";
import { GrowthBook } from "@growthbook/growthbook";
import { FeatureInterface, FeatureRule } from "shared/types/feature";
import { SavedGroupInterface } from "shared/types/saved-group";
import { ApiReqContext } from "back-end/types/api";
import {
  buildSDKPayloadForConnection,
  encrypt,
  SDKPayloadRawData,
} from "back-end/src/services/features";

const decryptionKey = "MDEyMzQ1Njc4OWFiY2RlZg=="; // Synthetic 16-byte test key.
const stamp = new Date("2026-01-01T00:00:00.000Z");
const groups = [
  { id: "beta", type: "list", attributeKey: "id", values: ["u_1", "u_2"] },
  { id: "numbers", type: "list", attributeKey: "age", values: ["21", "30"] },
  { id: "tags", type: "list", attributeKey: "tags", values: ["vip"] },
  {
    id: "nested-id",
    type: "list",
    attributeKey: "account.id",
    values: ["a_1"],
  },
  { id: "parent-value", type: "list", attributeKey: "value", values: ["on"] },
  { id: "pro", type: "condition", condition: JSON.stringify({ plan: "pro" }) },
  {
    id: "parent-condition",
    type: "condition",
    condition: JSON.stringify({ value: "on" }),
  },
  {
    id: "nested",
    type: "condition",
    condition: JSON.stringify({ country: "US", $savedGroups: ["pro", "beta"] }),
  },
  {
    id: "cycle-a",
    type: "condition",
    condition: JSON.stringify({ $savedGroups: ["cycle-b"] }),
  },
  {
    id: "cycle-b",
    type: "condition",
    condition: JSON.stringify({ $savedGroups: ["cycle-a"] }),
  },
  { id: "invalid", type: "condition", condition: "not JSON" },
  { id: "unused", type: "list", attributeKey: "id", values: ["u_1"] },
].map((group) => ({
  ...group,
  organization: "org-test",
  groupName: group.id,
  dateCreated: stamp,
  dateUpdated: stamp,
})) as SavedGroupInterface[];

/** Stored feature data, not an SDK feature definition. */
function feature(
  id: string,
  rule: Partial<FeatureRule>,
  extra: Partial<FeatureInterface> = {},
): FeatureInterface {
  return {
    id,
    organization: "org-test",
    project: "",
    dateCreated: stamp,
    dateUpdated: stamp,
    defaultValue: "off",
    valueType: "string",
    archived: false,
    description: "",
    owner: "",
    version: 1,
    environmentSettings: {
      production: {
        enabled: true,
        rules: [
          {
            id: `rule-${id}`,
            type: "force",
            enabled: true,
            value: "on",
            ...rule,
          },
        ],
      },
    },
    ...extra,
  } as FeatureInterface;
}

const features = [
  feature("list", { savedGroups: [{ match: "all", ids: ["beta"] }] }),
  feature("condition", { savedGroups: [{ match: "all", ids: ["pro"] }] }),
  feature("nested", { savedGroups: [{ match: "all", ids: ["nested"] }] }),
  feature("all", { savedGroups: [{ match: "all", ids: ["beta", "pro"] }] }),
  feature("any", { savedGroups: [{ match: "any", ids: ["beta", "pro"] }] }),
  feature("none", { savedGroups: [{ match: "none", ids: ["beta", "pro"] }] }),
  feature("not-both", {
    condition: JSON.stringify({ $not: { $savedGroups: ["beta", "pro"] } }),
  }),
  feature("override", {
    condition: JSON.stringify({ backup_id: { $inGroup: "beta" } }),
  }),
  feature("exclude-override", {
    condition: JSON.stringify({ backup_id: { $notInGroup: "beta" } }),
  }),
  feature("numeric", { savedGroups: [{ match: "all", ids: ["numbers"] }] }),
  feature("array", { savedGroups: [{ match: "all", ids: ["tags"] }] }),
  feature("dot-path", { savedGroups: [{ match: "all", ids: ["nested-id"] }] }),
  feature("missing", {
    condition: JSON.stringify({ id: { $inGroup: "missing" } }),
  }),
  feature("exclude-missing", {
    condition: JSON.stringify({ id: { $notInGroup: "missing" } }),
  }),
  feature("invalid", { savedGroups: [{ match: "all", ids: ["invalid"] }] }),
  feature("cycle", { savedGroups: [{ match: "all", ids: ["cycle-a"] }] }),
  feature("parent", { condition: JSON.stringify({ country: "US" }) }),
  feature(
    "prerequisite-list",
    {},
    {
      prerequisites: [
        {
          id: "parent",
          condition: JSON.stringify({ value: { $inGroup: "parent-value" } }),
        },
      ],
    },
  ),
  feature(
    "prerequisite-condition",
    {},
    {
      prerequisites: [
        {
          id: "parent",
          condition: JSON.stringify({ $savedGroups: ["parent-condition"] }),
        },
      ],
    },
  ),
  feature("experiment", {
    type: "experiment",
    trackingKey: "saved-group-test",
    coverage: 1,
    hashAttribute: "id",
    condition: JSON.stringify({ $savedGroups: ["pro"] }),
    values: [
      { value: "control", weight: 0.5, key: "0" },
      { value: "treatment", weight: 0.5, key: "1" },
    ],
  }),
];
const users = [
  {
    id: "u_1",
    backup_id: "outside",
    plan: "pro",
    country: "US",
    age: 21,
    tags: ["basic", "vip"],
    account: { id: "a_1" },
  },
  {
    id: "u_1",
    backup_id: "outside",
    plan: "free",
    country: "CA",
    age: 22,
    tags: ["basic"],
    account: { id: "a_2" },
  },
  {
    id: "outside",
    backup_id: "u_1",
    plan: "pro",
    country: "US",
    age: 30,
    tags: [],
    account: "a_1",
  },
  {
    id: "outside",
    backup_id: "outside",
    plan: "free",
    country: "CA",
    age: "21",
    tags: ["vip"],
    account: { id: "a_1" },
  },
  {
    id: "u_2",
    backup_id: "u_2",
    plan: "pro",
    country: "US",
    age: 30,
    tags: "vip",
  },
  {},
];

it("exports server payloads whose evaluations agree across all three formats", async () => {
  // The SDK uses Web Crypto to decrypt the server's encrypted payloads.
  if (!globalThis.crypto)
    Object.defineProperty(globalThis, "crypto", { value: webcrypto });
  const context = {
    org: {
      id: "org-test",
      name: "Test",
      members: [],
      invites: [],
      settings: { attributeSchema: [{ property: "age", datatype: "number" }] },
    },
    models: {},
    userId: "test",
    email: "test@example.invalid",
    userName: "Test",
    initModels: () => {},
  } as unknown as ApiReqContext;
  const data: SDKPayloadRawData = {
    features,
    savedGroups: groups,
    groupMap: new Map(groups.map((group) => [group.id, group])),
    experimentMap: new Map(),
    safeRolloutMap: new Map(),
    holdoutsMap: new Map(),
    visualExperiments: [],
    urlRedirectExperiments: [],
  };
  const payloads = [];
  let evaluations: unknown = null;
  for (const format of ["inline", "referencesV1", "referencesV2"] as const) {
    for (const encrypted of [false, true]) {
      const payload = await buildSDKPayloadForConnection({
        context,
        data,
        connection: {
          environment: "production",
          projects: [],
          savedGroupFormat: format,
          capabilities: [
            "looseUnmarshalling",
            "bucketingV2",
            "prerequisites",
            "savedGroupReferences",
            "savedGroupReferencesV2",
          ],
          includeRuleIds: true,
          encryptPayload: encrypted,
          encryptionKey: decryptionKey,
        },
      });
      const results = [];
      for (const attributes of users) {
        const sdk = new GrowthBook({ attributes, decryptionKey });
        await sdk.setPayload(payload);
        const resultsByFeature = Object.fromEntries(
          features.map(({ id }) => {
            const result = sdk.evalFeature(id);
            return [
              id,
              {
                value: result.value,
                on: result.on,
                off: result.off,
                source: result.source,
                experimentResult: result.experimentResult
                  ? {
                      value: result.experimentResult.value,
                      variationId: result.experimentResult.variationId,
                      inExperiment: result.experimentResult.inExperiment,
                      hashUsed: result.experimentResult.hashUsed,
                    }
                  : null,
              },
            ];
          }),
        );
        results.push({ attributes, features: resultsByFeature });
        sdk.destroy();
      }
      if (evaluations === null) evaluations = results;
      else
        expect({ format, encrypted, results }).toEqual({
          format,
          encrypted,
          results: evaluations,
        });
      if (!encrypted) {
        expect(Object.keys(payload.features).sort()).toEqual(
          features.map((feature) => feature.id).sort(),
        );
        if (format === "referencesV1")
          expect(payload.savedGroups?.beta).toEqual(["u_1", "u_2"]);
        if (format === "referencesV2") {
          expect(payload.savedGroups?.beta).toEqual({
            type: "list",
            attributeKey: "id",
            values: ["u_1", "u_2"],
          });
          expect(payload.savedGroups?.pro).toEqual({
            type: "condition",
            condition: { plan: "pro" },
          });
          expect(payload.savedGroups?.unused).toBeUndefined();
          expect(JSON.stringify(payload)).not.toMatch(
            /"\$(?:inGroup|notInGroup|savedGroups)"/,
          );
        }
      } else {
        expect(payload.encryptedFeatures).toBeTruthy();
        if (format !== "inline")
          expect(payload.encryptedSavedGroups).toBeTruthy();
      }
      payloads.push({
        name: `${format}${encrypted ? "-encrypted" : ""}`,
        payload,
      });
    }
  }
  const server = process.env.GB_SERVER_REPO!;
  writeFileSync(
    process.env.GB_PAYLOAD_FIXTURE_OUTPUT!,
    JSON.stringify(
      {
        serverCommit: execFileSync("git", ["-C", server, "rev-parse", "HEAD"], {
          encoding: "utf8",
        }).trim(),
        decryptionKey,
        payloads,
        evaluations,
        invalidEncryptedSavedGroups: await Promise.all(
          ["not JSON", "[]", "null"].map((value) =>
            encrypt(value, decryptionKey),
          ),
        ),
      },
      null,
      2,
    ) + "\n",
  );
  console.log(
    `Generated ${payloads.length} payloads × ${users.length} users × ${features.length} features = ${payloads.length * users.length * features.length} evaluations`,
  );
}, 30000);
