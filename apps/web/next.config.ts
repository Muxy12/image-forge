import createNextIntlPlugin from "next-intl/plugin";
import type { NextConfig } from "next";

const withNextIntl = createNextIntlPlugin();

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // The SDK is a workspace TypeScript package; let Next transpile its source.
  transpilePackages: ["@imageforge/sdk"],
  // Cross-origin isolation headers are required for WebAssembly threads and
  // SharedArrayBuffer used by the browser-side conversion workers.
  async headers() {
    return [
      {
        source: "/:path*",
        headers: [
          { key: "Cross-Origin-Opener-Policy", value: "same-origin" },
          { key: "Cross-Origin-Embedder-Policy", value: "require-corp" },
        ],
      },
    ];
  },
};

export default withNextIntl(nextConfig);
