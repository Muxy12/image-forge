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
          // Cross-origin isolation for the WebAssembly/Worker conversion path.
          { key: "Cross-Origin-Opener-Policy", value: "same-origin" },
          { key: "Cross-Origin-Embedder-Policy", value: "require-corp" },
          // Baseline hardening.
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "X-Frame-Options", value: "DENY" },
          { key: "Content-Security-Policy", value: "frame-ancestors 'none'" },
          { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
          {
            key: "Permissions-Policy",
            value: "camera=(), microphone=(), geolocation=(), interest-cohort=()",
          },
        ],
      },
    ];
  },
};

export default withNextIntl(nextConfig);
