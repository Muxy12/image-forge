import { describe, expect, it } from "vitest";

import { chooseMode } from "./server";

describe("chooseMode", () => {
  it("runs in the browser when the browser can encode the target", () => {
    expect(chooseMode(true, false)).toBe("browser");
    expect(chooseMode(true, true)).toBe("browser");
  });

  it("falls back to the server when the browser cannot encode but a server exists", () => {
    expect(chooseMode(false, true)).toBe("server");
  });

  it("is unsupported when neither the browser nor a server can do it", () => {
    expect(chooseMode(false, false)).toBe("unsupported");
  });
});
