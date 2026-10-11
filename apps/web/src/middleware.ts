import createMiddleware from "next-intl/middleware";

import { routing } from "./i18n/routing";

export default createMiddleware(routing);

export const config = {
  // Match all pathnames except Next internals and files with an extension.
  matcher: ["/", "/(en|fr|es|de|pt|it)/:path*", "/((?!_next|_vercel|.*\\..*).*)"],
};
